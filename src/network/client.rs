use super::{
    message::{Message, MessageError},
    packet::Packet,
};
use crate::component::{self, online::ONLINE_CLIENTS, store::Store};
use bincode::{
    config::{LittleEndian, VarintEncoding, WithOtherEndian, WithOtherIntEncoding},
    DefaultOptions, Options,
};
use bytes::Bytes;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use lazy_static::lazy_static;
use log::{error, info};
use std::{
    collections::HashMap,
    sync::{
        atomic::{self, AtomicBool, AtomicU64, AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    time::timeout,
};
use tokio_rustls::server::TlsStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

lazy_static! {
    static ref BINCODER: WithOtherIntEncoding<WithOtherEndian<DefaultOptions, LittleEndian>, VarintEncoding> =
        DefaultOptions::new()
            .with_little_endian()
            .with_varint_encoding();
}

pub struct Client {
    device_id: AtomicU64,
    caller_tx_register: Arc<Mutex<HashMap<u8, Sender<Message>>>>,
    call_id: AtomicU8,
    send_tx: Sender<Vec<u8>>,
    shutdown_notify_tx: tokio::sync::broadcast::Sender<()>,
    shutdown_once: AtomicBool,
}

impl Client {
    pub async fn serve(stream: TlsStream<TcpStream>, store: Arc<dyn Store>) {
        let framed_stream = LengthDelimitedCodec::builder()
            .little_endian()
            .max_frame_length(16 * 1024 * 1024)
            .new_framed(stream);

        let (sink, stream) = framed_stream.split();
        let (send_tx, send_rx) = channel(32);
        let (shutdown_notify_tx, _) = tokio::sync::broadcast::channel(1);

        let client = Arc::new(Client {
            device_id: AtomicU64::new(0),
            caller_tx_register: Arc::new(Mutex::new(HashMap::new())),
            call_id: AtomicU8::new(0),
            send_tx,
            shutdown_notify_tx: shutdown_notify_tx.clone(),
            shutdown_once: AtomicBool::new(false),
        });

        tokio::spawn(Client::sink_loop(
            client.clone(),
            send_rx,
            sink,
            shutdown_notify_tx.subscribe(),
        ));

        Client::stream_loop(client, store, stream, shutdown_notify_tx.subscribe()).await;
    }

    pub async fn send(&self, message: Message) -> anyhow::Result<()> {
        self.inner_send(0, message).await
    }

    pub async fn call(
        &self,
        message: Message,
        time_out_duration: Duration,
    ) -> anyhow::Result<Message, MessageError> {
        if time_out_duration.is_zero() {
            return Err(MessageError::InvalidArguments);
        }

        let (tx, mut rx) = channel(1);

        let call_id = self.next_call_id();

        self.register_call(call_id, tx).await;

        if let Err(err) = self.inner_send(call_id, message).await {
            self.remove_call(&call_id).await;
            error!("call failed: {:?}", err);
            return Err(MessageError::InternalError);
        }

        match timeout(time_out_duration, rx.recv()).await {
            Ok(res) => match res {
                Some(message) => match message {
                    Message::Error(err) => Err(err),
                    resp_message => Ok(resp_message),
                },
                None => Err(MessageError::InternalError),
            },
            Err(_) => {
                self.remove_call(&call_id).await;
                Err(MessageError::CallTimeout)
            }
        }
    }

    async fn inner_send(&self, call_id: u8, message: Message) -> anyhow::Result<()> {
        let packet = Packet { call_id, message };
        let buf = BINCODER.serialize(&packet)?;
        self.send_tx
            .send(buf)
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    fn next_call_id(&self) -> u8 {
        let mut call_id = self.call_id.fetch_add(1, atomic::Ordering::SeqCst);

        if call_id == 0 {
            call_id = self.call_id.fetch_add(1, atomic::Ordering::SeqCst);
        }

        call_id
    }

    async fn register_call(&self, call_id: u8, tx: Sender<Message>) {
        let mut register = self.caller_tx_register.lock().await;
        register.insert(call_id, tx);
    }

    async fn remove_call(&self, call_id: &u8) -> Option<Sender<Message>> {
        let mut register = self.caller_tx_register.lock().await;
        register.remove(call_id)
    }

    async fn shutdown(&self) {
        if let Err(true) =
            self.shutdown_once
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            return;
        }

        if let Err(err) = self.shutdown_notify_tx.send(()) {
            error!("client: failed to send shutdown notification: {}", err);
        }

        if let Some(device_id) = self.device_id() {
            let mut online_clients = ONLINE_CLIENTS.write().await;
            online_clients.remove(&device_id);
            drop(online_clients);
        }

        info!(
            "client: device '{}' shutdown",
            self.device_id().unwrap_or_else(|| String::from("None"))
        );
    }

    async fn sink_loop(
        client: Arc<Client>,
        mut send_rx: Receiver<Vec<u8>>,
        mut sink: SplitSink<Framed<TlsStream<TcpStream>, LengthDelimitedCodec>, Bytes>,
        mut shutdown_notify_rx: tokio::sync::broadcast::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                _ = shutdown_notify_rx.recv() => break,
                res = send_rx.recv() => match res {
                    Some(packet_bytes) => {
                        if let Err(err) = sink.send(Bytes::from(packet_bytes)).await {
                            error!("client: sink_loop send error: {}", err);
                        }
                    }
                    None => break,
                }
            };
        }

        info!("client: sink_loop exit");
        client.shutdown().await;
    }

    async fn stream_loop(
        client: Arc<Client>,
        store: Arc<dyn Store>,
        mut stream: SplitStream<Framed<TlsStream<TcpStream>, LengthDelimitedCodec>>,
        mut shutdown_notify_rx: tokio::sync::broadcast::Receiver<()>,
    ) {
        loop {
            let packet_bytes = tokio::select! {
                _ = shutdown_notify_rx.recv() => break,
                res = stream.next() => match res {
                    Some(Ok(packet)) => packet,
                    Some(Err(err)) => match err.kind() {
                        std::io::ErrorKind::UnexpectedEof => {
                            info!("client: disconnected");
                            break;
                        },
                        std::io::ErrorKind::ConnectionReset => {
                            info!("client: connection reset");
                            break;
                        },
                        _ => {
                            error!("client: stream_loop read packet error: {:?}", err);
                            continue;
                        }
                    }
                    None => break,
                }
            };

            let packet = match BINCODER.deserialize::<Packet>(&packet_bytes) {
                Ok(packet) => packet,
                Err(err) => {
                    error!("client: stream_loop deserialize packet error: {:?}", err);
                    continue;
                }
            };

            match client.remove_call(&packet.call_id).await {
                Some(sender) => {
                    if let Err(err) = sender.send(packet.message).await {
                        error!(
                            "client: stream_loop send packet to call receiver error: {:?}",
                            err
                        );
                    }
                }
                None => {
                    let client = client.clone();
                    let store = store.clone();

                    tokio::spawn(async move {
                        let resp_message = packet
                            .message
                            .handle(client.clone(), store)
                            .await
                            .unwrap_or_else(|m| Message::Error(m));

                        if resp_message == Message::None {
                            return;
                        }

                        if let Err(err) = client.inner_send(packet.call_id, resp_message).await {
                            error!(
                                "client: stream_loop send call response message error: {:?}",
                                err
                            );
                        }
                    });
                }
            };
        }

        info!("client: stream_loop exit");
        client.shutdown().await;
    }

    #[must_use]
    pub fn device_id(&self) -> Option<String> {
        let device_id_num = self.device_id.load(Ordering::Acquire);
        if device_id_num == 0 {
            None
        } else {
            Some(component::device_id::to_string(device_id_num))
        }
    }

    pub fn set_device_id(&self, device_id: String) -> anyhow::Result<()> {
        let device_id_num = component::device_id::from_str(&device_id)?;
        self.device_id.store(device_id_num, Ordering::Release);
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        info!(
            "client: '{}' drop",
            self.device_id().unwrap_or_else(|| String::from("None"))
        );
    }
}

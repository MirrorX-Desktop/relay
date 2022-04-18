use super::client::Client;
use crate::{
    component::online::ClientManager,
    network::{
        client::BINCODER,
        packet::{Packet, ReplyPacket},
    },
    service::{
        device::DeviceService,
        message::{reply::ReplyMessage, request::RequestMessage},
        proxy::ProxyService,
    },
};
use bincode::Options;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use rustls::{Certificate, PrivateKey};
use rustls_pemfile::certs;
use std::{fs::File, io::BufReader, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tokio_util::codec::LengthDelimitedCodec;

pub async fn run(
    device_service: DeviceService,
    proxy_service: ProxyService,
    client_manager: Arc<ClientManager>,
) -> anyhow::Result<()> {
    // let cert_file = certs(&mut BufReader::new(File::open("cert/example.com.crt")?))
    //     .map(|mut certs| certs.drain(..).map(Certificate).collect())?;

    // let mut key_file: Vec<PrivateKey> = rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(
    //     File::open("cert/example.com.key")?,
    // ))
    // .map(|mut certs| certs.drain(..).map(PrivateKey).collect())?;

    // let config = rustls::ServerConfig::builder()
    //     .with_safe_defaults()
    //     .with_no_client_auth()
    //     .with_single_cert(cert_file, key_file.remove(0))?;

    // let acceptor = TcpListener::from(Arc::new(config));

    let listener = TcpListener::bind("0.0.0.0:45555").await?;

    let inner_device_service = Arc::new(device_service);
    let inner_proxy_service = Arc::new(proxy_service);

    loop {
        let (stream, _) = listener.accept().await?;

        // let acceptor = acceptor.clone();
        // let stream = acceptor.accept(stream).await?;

        serve_stream(
            stream,
            inner_device_service.clone(),
            inner_proxy_service.clone(),
            client_manager.clone(),
        );
    }
}

fn serve_stream(
    stream: TcpStream,
    device_service: Arc<DeviceService>,
    proxy_service: Arc<ProxyService>,
    client_manager: Arc<ClientManager>,
) {
    let framed_stream = LengthDelimitedCodec::builder()
        .little_endian()
        .max_frame_length(16 * 1024 * 1024)
        .new_framed(stream);

    let (mut sink, mut stream) = framed_stream.split();
    let (tx, mut rx) = mpsc::channel(16);
    let client = Arc::new(Client::new(tx));

    tokio::spawn(async move {
        loop {
            let packet_bytes = tokio::select! {
                // _ = shutdown_notify_rx.recv() => break,
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

            info!("client: packet: {:?}", packet_bytes);

            let packet = match BINCODER.deserialize::<Packet>(&packet_bytes) {
                Ok(packet) => packet,
                Err(err) => {
                    error!("client: stream_loop deserialize packet error: {:?}", err);
                    continue;
                }
            };

            if let Some(request_packet) = packet.clone().request_packet {
                if let Some(to_device_id) = request_packet.to_device_id {
                    if let Err(err) = proxy_service
                        .proxy(client.clone(), to_device_id.to_owned(), packet)
                        .await
                    {
                        let reply_packet = ReplyPacket {
                            call_id: request_packet.call_id,
                            payload: Result::Err(err),
                            to_device_id: Some(to_device_id),
                        };

                        if let Err(err) = client.reply_request(reply_packet).await {
                            error!("client reply_request failed: {:?}", err);
                        }
                    }
                } else {
                    let inner_device_service = device_service.clone();

                    let client = client.clone();
                    tokio::spawn(async move {
                        let res = match request_packet.payload {
                            RequestMessage::HeartBeatRequest(message) => inner_device_service
                                .heart_beat(client.clone(), message)
                                .await
                                .map(|msg| ReplyMessage::HeartBeatReply(msg)),
                            RequestMessage::RegisterIdRequest(message) => inner_device_service
                                .register_id(client.clone(), message)
                                .await
                                .map(|msg| ReplyMessage::RegisterIdReply(msg)),
                        };

                        let reply_packet = ReplyPacket {
                            call_id: request_packet.call_id,
                            payload: res,
                            to_device_id: client.device_id(),
                        };

                        if let Err(err) = client.reply_request(reply_packet).await {
                            error!("client reply_request failed: {:?}", err);
                        }
                    });
                }
            } else if let Some(reply_packet) = packet.reply_packet {
                client.reply_call(reply_packet.call_id, reply_packet);
            } else {
                warn!("client receive unknown packet");
            }
        }

        if let Some(device_id) = client.device_id() {
            client_manager.remove(&device_id);
        }

        info!("client: stream_loop exit");
    });

    tokio::spawn(async move {
        loop {
            let buf = match rx.recv().await {
                Some(buf) => buf,
                None => break,
            };

            if let Err(err) = sink.send(Bytes::from(buf)).await {
                error!("send error: {:?}", err);
            }
        }
        info!("client: sink_loop exit");
    });
}

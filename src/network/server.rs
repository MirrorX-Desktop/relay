use super::Client;
use crate::{
    network::{client::BINCODER, packet::Packet},
    service::{
        desktop::DesktopService,
        device::DeviceService,
        message::{reply::ReplyMessage, request::RequestMessage},
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

pub async fn server(
    device_service: DeviceService,
    desktop_service: DesktopService,
) -> anyhow::Result<()> {
    let cert_file = certs(&mut BufReader::new(File::open("cert/example.com.crt")?))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())?;

    let mut key_file: Vec<PrivateKey> = rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(
        File::open("cert/example.com.key")?,
    ))
    .map(|mut certs| certs.drain(..).map(PrivateKey).collect())?;

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_file, key_file.remove(0))?;

    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind("0.0.0.0:45555").await?;

    let inner_device_service = Arc::new(device_service);
    let inner_desktop_service = Arc::new(desktop_service);

    loop {
        let (stream, _) = listener.accept().await?;

        let acceptor = acceptor.clone();
        let stream = acceptor.accept(stream).await?;

        serve_stream(
            stream,
            inner_device_service.clone(),
            inner_desktop_service.clone(),
        );
    }
}

fn serve_stream<'a, 'b>(
    stream: TlsStream<TcpStream>,
    device_service: Arc<DeviceService>,
    desktop_service: Arc<DesktopService>,
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

            let packet = match BINCODER.deserialize::<Packet>(&packet_bytes) {
                Ok(packet) => packet,
                Err(err) => {
                    error!("client: stream_loop deserialize packet error: {:?}", err);
                    continue;
                }
            };

            if let Some(request_packet) = packet.request_packet {
                let inner_device_service = device_service.clone();
                let inner_desktop_service = desktop_service.clone();

                let client = client.clone();
                tokio::spawn(async move {
                    let res = match request_packet.request_message {
                        RequestMessage::HeartBeatRequest(message) => inner_device_service
                            .heart_beat(client.clone(), message)
                            .await
                            .map(|msg| ReplyMessage::HeartBeatReply(msg)),
                        RequestMessage::RegisterDeviceIdRequest(message) => inner_device_service
                            .register_id(client.clone(), message)
                            .await
                            .map(|msg| ReplyMessage::RegisterDeviceIdReply(msg)),
                        RequestMessage::ConnectRequest(message) => inner_desktop_service
                            .connect(client.clone(), message)
                            .await
                            .map(|msg| ReplyMessage::ConnectReply(msg)),
                    };

                    if let Ok(reply_message) = res {
                        if let Err(err) = client
                            .reply_request(request_packet.call_id, reply_message)
                            .await
                        {
                            error!("client reply_request failed: {:?}", err);
                        }
                    } else {
                        // todo: handle error
                        todo!();
                    }
                });
            } else if let Some(reply_packet) = packet.reply_packet {
                client.reply_call(reply_packet.call_id, reply_packet.reply_message);
            } else {
                warn!("client receive unknown packet");
            }
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
    });
}

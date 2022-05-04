use super::{client::Client, client_to_server::ClientToServerMessage};
use crate::{
    handler,
    instance::client_manager::CLIENT_MANAGER,
    network::{packet::Packet, server_to_client::ServerToClientMessage},
    utility::bincode::BINCODE_SERIALIZER,
};
use bincode::Options;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::mpsc,
};
use tokio_util::codec::LengthDelimitedCodec;

macro_rules! handle_client_to_server_message {
    ($client:expr, $call_id:ident, $handler:path, $req:ident, $reply_type:path) => {
        tokio::spawn(async move {
            let res = $handler($client.clone(), $req)
                .await
                .map(|reply| $reply_type(reply));

            let message = match res {
                Ok(message) => message,
                Err(err) => {
                    error!(
                        "handle_client_to_client_message: handle message error: {:?}",
                        err
                    );
                    ServerToClientMessage::Error
                }
            };

            $client
                .send(Packet::ServerToClient($call_id, message))
                .await
        });
    };
}

pub async fn run<A>(addr: A) -> anyhow::Result<()>
where
    A: ToSocketAddrs,
{
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, stream_addr) = listener.accept().await?;

        info!("accepted connection from {}", stream_addr);

        serve_stream(stream);
    }
}

fn serve_stream(stream: TcpStream) {
    let framed_stream = LengthDelimitedCodec::builder()
        .little_endian()
        .max_frame_length(16 * 1024 * 1024)
        .new_framed(stream);

    let (mut sink, mut stream) = framed_stream.split();
    let (tx, mut rx) = mpsc::channel(16);
    let client = Arc::new(Client::new(tx));

    tokio::spawn(async move {
        loop {
            let packet_bytes = match stream.next().await {
                Some(res) => match res {
                    Ok(packet_bytes) => packet_bytes,
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::UnexpectedEof => {
                            info!("serve_stream: disconnected, exit");
                            break;
                        }
                        std::io::ErrorKind::ConnectionReset => {
                            info!("serve_stream: connection reset, exit");
                            break;
                        }
                        _ => {
                            error!("serve_stream: read packet error: {:?}", err);
                            continue;
                        }
                    },
                },
                None => break,
            };

            let packet = match BINCODE_SERIALIZER.deserialize::<Packet>(&packet_bytes) {
                Ok(packet) => packet,
                Err(err) => {
                    error!("serve_stream: deserialize packet error: {:?}", err);
                    continue;
                }
            };

            match packet {
                Packet::ClientToServer(call_id, message) => {
                    if let Err(err) =
                        handle_client_to_server_message(client.clone(), call_id, message)
                    {
                        error!("{}", err);
                    }
                }
                Packet::ServerToClient(call_id, message) => {
                    warn!(
                        "serve_stream: received unexpected server to client packet: {:?}",
                        message
                    );
                }
                Packet::ClientToClient(
                    call_id,
                    from_device_id,
                    to_device_id,
                    is_secure,
                    message_bytes,
                ) => {
                    if let Some(remote_client) = CLIENT_MANAGER.get(&to_device_id) {
                        if let Err(err) = remote_client
                            .value()
                            .send(Packet::ClientToClient(
                                call_id,
                                from_device_id,
                                to_device_id,
                                is_secure,
                                message_bytes,
                            ))
                            .await
                        {
                            error!("{}", err);
                        }
                    } else {
                        warn!("serve_stream: client not found: {}", to_device_id);
                    }
                }
            };
        }

        info!("serve_stream: stream_loop exit");
        if let Some(device_id) = client.device_id() {
            CLIENT_MANAGER.remove(&device_id);
        }
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
        info!("serve_stream: sink_loop exit");
    });
}

fn handle_client_to_server_message(
    client: Arc<Client>,
    call_id: u16,
    message: ClientToServerMessage,
) -> anyhow::Result<()> {
    match message {
        ClientToServerMessage::HeartBeatRequest(req) => {
            handle_client_to_server_message!(
                client,
                call_id,
                handler::heartbeat::heartbeat,
                req,
                ServerToClientMessage::HeartBeatReply
            );
        }
        ClientToServerMessage::HandshakeRequest(req) => {
            handle_client_to_server_message!(
                client,
                call_id,
                handler::handshake::handshake,
                req,
                ServerToClientMessage::HandshakeReply
            );
        }
        _ => {
            warn!(
                "handle_client_to_client_message: unhandled message: {:?}",
                message
            );
        }
    };

    Ok(())
}

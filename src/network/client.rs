use crate::utility::serializer::BINCODE_SERIALIZER;
use bincode::Options;
use bytes::{Buf, Bytes};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{io::AsyncWriteExt, net::TcpStream};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub static WAITING_CLIENTS: Lazy<DashMap<String, (i64, Framed<TcpStream, LengthDelimitedCodec>)>> =
    Lazy::new(DashMap::new);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndPointHandshakeRequest {
    pub visit_credentials: String,
    pub device_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndPointHandshakeResponse {
    pub remote_device_id: i64,
}

pub async fn serve(
    mut framed_stream: Framed<TcpStream, LengthDelimitedCodec>,
) -> anyhow::Result<()> {
    let handshake_request_bytes = framed_stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("read handshake request failed"))??;

    let handshake_request: EndPointHandshakeRequest =
        BINCODE_SERIALIZER.deserialize_from(handshake_request_bytes.reader())?;

    tracing::info!(
        device_id = ?handshake_request.device_id,
        "new client",
    );

    if let Some((visit_credentials, stream_tuple)) =
        WAITING_CLIENTS.remove(&(handshake_request.visit_credentials))
    {
        if visit_credentials == handshake_request.visit_credentials {
            tokio::spawn(async move {
                transfer_between_endpoints(
                    (handshake_request.device_id, framed_stream),
                    stream_tuple,
                )
                .await;
            });
        }
    } else {
        WAITING_CLIENTS.insert(
            handshake_request.visit_credentials.to_owned(),
            (handshake_request.device_id, framed_stream),
        );

        // todo: find better way
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(60 * 2)).await;
            if let Some(_) = WAITING_CLIENTS.remove(&(handshake_request.visit_credentials)) {
                tracing::debug!(
                    device_id = handshake_request.device_id,
                    "remove TTL reached unused stream"
                );
            }
        });
    }

    Ok(())
}

async fn transfer_between_endpoints(
    stream_tuple1: (i64, Framed<TcpStream, LengthDelimitedCodec>),
    stream_tuple2: (i64, Framed<TcpStream, LengthDelimitedCodec>),
) {
    let (device_id_1, mut stream_1) = stream_tuple1;
    let (device_id_2, mut stream_2) = stream_tuple2;

    if let Err(err) = reply_handshake(device_id_1, &mut stream_2).await {
        tracing::error!(
            reply_device_id = device_id_2,
            ?err,
            "reply handshake failed"
        );
        return;
    }

    if let Err(err) = reply_handshake(device_id_2, &mut stream_1).await {
        tracing::error!(
            reply_device_id = device_id_1,
            ?err,
            "reply handshake failed"
        );
        return;
    }

    let mut stream_1 = stream_1.into_inner();
    let mut stream_2 = stream_2.into_inner();

    let (mut reader_1, mut writer_1) = stream_1.split();
    let (mut reader_2, mut writer_2) = stream_2.split();

    let active_to_passive = async {
        tokio::io::copy(&mut reader_1, &mut writer_2).await?;
        writer_2.shutdown().await?;
        tracing::debug!(
            reader_device_id = device_id_1,
            writer_device_id = device_id_2,
            "copy process exit"
        );
        std::io::Result::Ok(())
    };

    let passive_to_active = async {
        tokio::io::copy(&mut reader_2, &mut writer_1).await?;
        writer_1.shutdown().await?;
        tracing::debug!(
            reader_device_id = device_id_2,
            writer_device_id = device_id_1,
            "copy process exit"
        );
        std::io::Result::Ok(())
    };

    let _ = tokio::try_join!(active_to_passive, passive_to_active);
    tracing::info!("exit both");
}

async fn reply_handshake(
    other_side_device_id: i64,
    stream: &mut Framed<TcpStream, LengthDelimitedCodec>,
) -> anyhow::Result<()> {
    let resp = EndPointHandshakeResponse {
        remote_device_id: other_side_device_id,
    };

    let resp_buffer = BINCODE_SERIALIZER
        .serialize(&resp)
        .map_err(|err| anyhow::anyhow!(err))?;

    stream.send(Bytes::from(resp_buffer)).await?;

    Ok(())
}

use crate::{model::StreamingClientStat, utility::serializer::BINCODE_SERIALIZER};
use bincode::Options;
use bytes::Bytes;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
    sync::mpsc::Sender,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[allow(clippy::type_complexity)]
pub static WAITING_CLIENTS: Lazy<DashMap<Vec<u8>, (i64, Framed<TcpStream, LengthDelimitedCodec>)>> =
    Lazy::new(DashMap::new);

#[allow(clippy::type_complexity)]
pub static STREAMING_CLIENTS: Lazy<DashMap<(i64, i64), StreamingClientStat>> =
    Lazy::new(DashMap::new);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndPointHandshakeRequest {
    #[serde(with = "serde_bytes")]
    pub visit_credentials: Vec<u8>,
    pub device_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndPointHandshakeResponse {
    pub remote_device_id: i64,
}

pub async fn serve(
    mut framed_stream: Framed<TcpStream, LengthDelimitedCodec>,
    bytes_statistics_tx: Sender<u64>,
) -> anyhow::Result<()> {
    let handshake_request_bytes = framed_stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("read handshake request failed"))??;

    let handshake_request: EndPointHandshakeRequest =
        BINCODE_SERIALIZER.deserialize(handshake_request_bytes.freeze().deref())?;

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
                    bytes_statistics_tx,
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
            if WAITING_CLIENTS
                .remove(&(handshake_request.visit_credentials))
                .is_some()
            {
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
    bytes_statistics_tx: Sender<u64>,
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

    let active_addr = stream_1
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_default();

    let passive_addr = stream_2
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_default();

    let (mut reader_1, mut writer_1) = stream_1.split();
    let (mut reader_2, mut writer_2) = stream_2.split();

    STREAMING_CLIENTS.insert(
        (device_id_1, device_id_2),
        StreamingClientStat {
            active_device_id: device_id_1,
            active_addr,
            passive_device_id: device_id_2,
            passive_addr,
            timestamp: chrono::Utc::now().timestamp(),
        },
    );

    let bytes_statistics_tx_copy = bytes_statistics_tx.clone();
    let active_to_passive = async {
        copy_between_stream(&mut reader_1, &mut writer_2, bytes_statistics_tx_copy).await?;
        writer_2.shutdown().await?;
        tracing::debug!(
            reader_device_id = device_id_1,
            writer_device_id = device_id_2,
            "copy process exit"
        );

        STREAMING_CLIENTS.remove(&(device_id_1, device_id_2));

        std::io::Result::Ok(())
    };

    let passive_to_active = async {
        copy_between_stream(&mut reader_2, &mut writer_1, bytes_statistics_tx).await?;
        writer_1.shutdown().await?;
        tracing::debug!(
            reader_device_id = device_id_2,
            writer_device_id = device_id_1,
            "copy process exit"
        );

        STREAMING_CLIENTS.remove(&(device_id_1, device_id_2));

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

    tracing::info!(?resp_buffer, ?other_side_device_id, "serialize");

    stream.send(Bytes::from(resp_buffer)).await?;

    Ok(())
}

async fn copy_between_stream(
    reader: &'_ mut ReadHalf<'_>,
    writer: &'_ mut WriteHalf<'_>,
    bytes_statistics_tx: Sender<u64>,
) -> std::io::Result<()> {
    let mut buffer = [0u8; 1024 * 16];
    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 {
            return Ok(());
        }

        writer.write_all(&buffer[0..n]).await?;

        let _ = bytes_statistics_tx.try_send(n as u64);
    }
}

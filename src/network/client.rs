use anyhow::bail;
use fxhash::FxHashMap;
use log::error;
use once_cell::sync::Lazy;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};

pub static WAITING_CLIENTS: Lazy<Mutex<FxHashMap<(String, String), TcpStream>>> =
    Lazy::new(|| Mutex::new(FxHashMap::default()));

pub async fn serve(mut stream: TcpStream) -> anyhow::Result<()> {
    if let Err(err) = stream.set_nodelay(true) {
        bail!("set nodelay for stream failed ({:?})", err);
    }

    // read first fixed size handshake buffer

    // 10 bytes utf8(single byte same as ascii) active device id
    // 10 bytes utf8(single byte same as ascii) passive device id

    let mut active_device_id_buf = [0u8; 10];
    tokio::time::timeout(
        Duration::from_secs(10),
        stream.read_exact(&mut active_device_id_buf),
    )
    .await??;
    let active_device_id = String::from_utf8(active_device_id_buf.to_vec())?;

    let mut passive_device_id_buf = [0u8; 10];
    tokio::time::timeout(
        Duration::from_secs(10),
        stream.read_exact(&mut passive_device_id_buf),
    )
    .await??;
    let passive_device_id = String::from_utf8(passive_device_id_buf.to_vec())?;

    let mut clients = WAITING_CLIENTS.lock().await;
    if let Some(remote_stream) =
        clients.remove(&(active_device_id.clone(), passive_device_id.clone()))
    {
        // remote endpoint client had connected, go to transfer process
        tokio::spawn(transfer_between_endpoints(stream, remote_stream));
    } else {
        clients.insert(
            (active_device_id.clone(), passive_device_id.clone()),
            stream,
        );

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let mut clients = WAITING_CLIENTS.lock().await;
            clients.remove(&(active_device_id, passive_device_id))
        });
    }

    return Ok(());
}

async fn transfer_between_endpoints(mut active_stream: TcpStream, mut passive_stream: TcpStream) {
    // the 'active' and 'passive' flag just for identify, not realistic endpoint type.

    if let Err(err) = active_stream.write(&vec![1u8]).await {
        error!("stream write handshake response failed ({})", err);
    }

    if let Err(err) = passive_stream.write(&vec![1u8]).await {
        error!("stream write handshake response failed ({})", err);
    }

    let (mut active_reader, mut active_writer) = active_stream.split();
    let (mut passive_reader, mut passive_writer) = passive_stream.split();

    let active_to_passive = async {
        tokio::io::copy(&mut active_reader, &mut passive_writer).await?;
        passive_writer.shutdown().await
    };

    let passive_to_active = async {
        tokio::io::copy(&mut passive_reader, &mut active_writer).await?;
        active_writer.shutdown().await
    };

    let _ = tokio::try_join!(active_to_passive, passive_to_active);
}

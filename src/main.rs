mod api;
mod model;
mod network;
mod utility;

use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::time::Duration;
use tokio::{net::TcpListener, sync::RwLock};
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::codec::LengthDelimitedCodec;

pub(crate) static BYTES_TRANSFERRED_SNAPSHOT: Lazy<RwLock<u64>> = Lazy::new(|| RwLock::new(0));

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("load .env from {:?}", dotenv().unwrap());

    let tcp_listen_addr = std::env::var("TCP_LISTEN_ADDR")?;

    let listener = TcpListener::bind(&tcp_listen_addr).await?;

    let (every_day_tx, mut every_day_rx) = tokio::sync::mpsc::channel(1);
    let clear_bytes_transferred_job = Job::new("0 0 0 * * ? *", move |_, _| {
        let _ = every_day_tx.try_send(());
    })?;

    let scheduler = JobScheduler::new().await?;
    scheduler.add(clear_bytes_transferred_job).await?;
    scheduler.start().await?;

    let (bytes_statistics_tx, mut bytes_statistics_rx) =
        tokio::sync::mpsc::channel(u16::MAX as usize);

    tokio::spawn(async move {
        if let Ok(addr) = listener.local_addr() {
            tracing::info!("server listen on: {}", addr);
        }

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(endpoint) => endpoint,
                Err(err) => {
                    tracing::error!(?err, "listener accept failed");
                    break;
                }
            };

            if let Err(err) = stream.set_nodelay(true) {
                tracing::error!(?err, "set stream nodelay failed");
                continue;
            }

            let framed_stream = LengthDelimitedCodec::builder()
                .little_endian()
                .max_frame_length(32 * 1024 * 1024)
                .new_framed(stream);

            let bytes_statistics_tx_copy = bytes_statistics_tx.clone();
            tokio::spawn(async move {
                if let Err(err) =
                    network::client::serve(framed_stream, bytes_statistics_tx_copy).await
                {
                    tracing::error!(?err, "serve tcp stream failed")
                }
            });
        }
    });

    tokio::spawn(async move {
        let mut bytes_transferred = 0u64;
        let mut snapshot_interval = tokio::time::interval(Duration::from_secs(60));

        loop {
            tokio::select! {
                _ = snapshot_interval.tick() => {
                    *BYTES_TRANSFERRED_SNAPSHOT.write().await = bytes_transferred;
                    continue;
                },
                _ = every_day_rx.recv() => {
                    bytes_transferred = 0;
                    continue;
                },
                bytes_count = bytes_statistics_rx.recv() => {
                    if let Some(bytes_count) = bytes_count {
                        bytes_transferred += bytes_count;
                    } else {
                        return;
                    }
                }
            }
        }
    });

    tokio::spawn(api::launch_api_server());

    tokio::signal::ctrl_c()
        .await
        .map_err(|err| anyhow::anyhow!("failed to listen for event ({})", err))
}

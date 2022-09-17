mod network;
mod utility;

use dotenvy::dotenv;
use tokio::net::TcpListener;
use tokio_util::codec::LengthDelimitedCodec;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("load .env from {:?}", dotenv().unwrap());

    let tcp_listen_addr = std::env::var("TCP_LISTEN_ADDR")?;

    let listener = TcpListener::bind(&tcp_listen_addr).await?;

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

            tokio::spawn(async move {
                if let Err(err) = network::client::serve(framed_stream).await {
                    tracing::error!(?err, "serve tcp stream failed")
                }
            });
        }
    });

    tokio::signal::ctrl_c()
        .await
        .map_err(|err| anyhow::anyhow!("failed to listen for event ({})", err))
}

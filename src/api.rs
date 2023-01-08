use crate::{model::StatResponse, network::client::STREAMING_CLIENTS, BYTES_TRANSFERRED_SNAPSHOT};
use axum::{routing::get, Json, Router};
use std::net::{Ipv4Addr, SocketAddr};

pub async fn launch_api_server() {
    let api_listen_port: u16 = std::env::var("API_LISTEN_PORT").unwrap().parse().unwrap();

    let http_listen_addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, api_listen_port).into();

    let api = Router::new().route("/api/stat", get(get_stat));

    tracing::info!("http api server listening on {}", http_listen_addr);
    let http_future = axum::Server::bind(&http_listen_addr).serve(api.into_make_service());

    tokio::select! {
        _ = http_future => {},
        _ = tokio::signal::ctrl_c() => {},
    }

    tracing::info!("http api server exit");
    std::process::exit(1);
}

async fn get_stat() -> Json<StatResponse> {
    const CLIENTS_SNAPSHOT_CAPACITY: usize = 10;

    let bytes_transferred = *BYTES_TRANSFERRED_SNAPSHOT.read().await;
    let mut client_snapshot = Vec::new();

    for entry in STREAMING_CLIENTS.iter() {
        if client_snapshot.len() >= CLIENTS_SNAPSHOT_CAPACITY {
            break;
        }

        client_snapshot.push(entry.value().clone())
    }

    Json(StatResponse {
        bytes_transferred,
        client_snapshot,
    })
}

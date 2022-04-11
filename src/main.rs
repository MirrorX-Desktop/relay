mod component;
mod handler;
mod network;
mod service;
mod session;

use env_logger::{Builder, Target};
use log::LevelFilter;
use service::{
    rpc::{desktop_server::DesktopServer, device_server::DeviceServer},
    DesktopService, DeviceService,
};
use std::{
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    let service_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 35535);

    let store_arc = component::store::new_store(true)?;
    let session_mgr = Arc::new(session::Manager::new());

    let device_service = DeviceService::new(session_mgr.clone(), store_arc);
    let device_svc = DeviceServer::new(device_service);

    let desktop_service = DesktopService::new(session_mgr);
    let desktop_svc = DesktopServer::new(desktop_service);

    tonic::transport::Server::builder()
        .add_service(device_svc)
        .add_service(desktop_svc)
        .serve(service_addr)
        .await?;

    Ok(())
}

fn init_logger() {
    Builder::new()
        .filter_level(LevelFilter::Info)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] [{}({}#{})] {} {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"),
                record.module_path().unwrap_or(""),
                record.file().unwrap_or(""),
                record.line().unwrap_or(0),
                record.level(),
                record.args(),
            )
        })
        .target(Target::Stdout)
        .init();
}

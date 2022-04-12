mod component;
mod network;
mod service;

use env_logger::{Builder, Target};
use log::LevelFilter;
use std::{io::Write, sync::Arc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    let store = component::store::new_store().unwrap();
    let client_manager = Arc::new(component::online::ClientManager::new());

    let device_service = service::device::DeviceService::new(store.clone(), client_manager.clone());
    let desktop_service = service::desktop::DesktopService::new(store.clone(), client_manager);

    network::server::server(device_service, desktop_service).await
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

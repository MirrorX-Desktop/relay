mod component;
mod handler;
mod network;

use env_logger::{Builder, Target};
use log::LevelFilter;
use network::server::server;
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    server().await
}

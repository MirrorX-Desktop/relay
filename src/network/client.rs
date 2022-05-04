use crate::utility::bincode::BINCODE_SERIALIZER;

use super::packet::Packet;
use bincode::Options;
use once_cell::sync::OnceCell;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct Client {
    tx: mpsc::Sender<Vec<u8>>,
    device_id: OnceCell<String>,
}

impl Client {
    pub fn new(tx: mpsc::Sender<Vec<u8>>) -> Self {
        Client {
            tx,
            device_id: OnceCell::new(),
        }
    }

    pub fn device_id(&self) -> Option<String> {
        self.device_id.get().and_then(|id| Some(id.clone()))
    }

    pub fn set_device_id(&self, device_id: String) -> anyhow::Result<()> {
        self.device_id
            .set(device_id)
            .map_err(|_| anyhow::anyhow!("device_id already set"))
    }

    pub async fn send(&self, packet: Packet) -> anyhow::Result<()> {
        let buf = BINCODE_SERIALIZER.serialize(&packet)?;
        self.tx.send_timeout(buf, Duration::from_secs(1)).await?;
        Ok(())
    }
}

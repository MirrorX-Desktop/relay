use super::packet::{Packet, ReplyPacket, RequestPacket};
use crate::service::message::{
    reply::ReplyMessage, reply_error::ReplyError, request::RequestMessage,
};
use bincode::{
    config::{LittleEndian, VarintEncoding, WithOtherEndian, WithOtherIntEncoding},
    DefaultOptions, Options,
};
use dashmap::DashMap;
use lazy_static::lazy_static;
use log::error;
use std::{
    sync::atomic::{AtomicU8, Ordering},
    time::Duration,
};
use tokio::sync::{mpsc, RwLock};

lazy_static! {
    pub static ref BINCODER: WithOtherIntEncoding<WithOtherEndian<DefaultOptions, LittleEndian>, VarintEncoding> =
        DefaultOptions::new()
            .with_little_endian()
            .with_varint_encoding();
}

pub struct Client {
    tx: mpsc::Sender<Vec<u8>>,
    call_tx_map: DashMap<u8, mpsc::Sender<ReplyMessage>>,
    device_id: RwLock<String>,
    call_id: AtomicU8,
}

impl Client {
    pub fn new(tx: mpsc::Sender<Vec<u8>>) -> Self {
        Client {
            tx,
            call_tx_map: DashMap::new(),
            device_id: RwLock::new(String::new()),
            call_id: AtomicU8::new(1),
        }
    }

    pub fn device_id(&self) -> String {
        self.device_id.blocking_read().clone()
    }

    pub fn set_device_id(&self, device_id: String) {
        self.device_id.blocking_write().clone_from(&device_id)
    }

    pub async fn call(
        &self,
        request_message: RequestMessage,
        timeout: Duration,
    ) -> Result<ReplyMessage, ReplyError> {
        let call_id = self.next_call_id();

        let packet = Packet {
            request_packet: Some(RequestPacket {
                call_id,
                request_message,
            }),
            reply_packet: None,
        };

        let mut rx = self.register_call(call_id);

        self.send(packet).await.map_err(|err| {
            error!("client call failed: {:?}", err);
            self.remove_call(call_id);
            ReplyError::Internal
        })?;

        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(res) => match res {
                Some(message) => Ok(message),
                None => Err(ReplyError::Internal),
            },
            Err(_) => {
                self.remove_call(call_id);
                Err(ReplyError::Timeout)
            }
        }
    }

    pub fn reply_call(&self, call_id: u8, reply_message: ReplyMessage) {
        self.remove_call(call_id).map(|tx| {
            if let Err(err) = tx.try_send(reply_message) {
                error!(
                    "client[{:?}] reply call failed: {:?}",
                    self.device_id(),
                    err
                )
            }
        });
    }

    pub async fn reply_request(
        &self,
        call_id: u8,
        reply_message: ReplyMessage,
    ) -> anyhow::Result<()> {
        let packet = Packet {
            request_packet: None,
            reply_packet: Some(ReplyPacket {
                call_id,
                reply_message,
            }),
        };

        self.send(packet).await
    }

    async fn send(&self, packet: Packet) -> anyhow::Result<()> {
        let buf = BINCODER.serialize(&packet)?;
        self.tx.send_timeout(buf, Duration::from_secs(1)).await?;
        Ok(())
    }

    fn next_call_id(&self) -> u8 {
        loop {
            let new_call_id = self.call_id.fetch_add(1, Ordering::AcqRel);
            if new_call_id == 0 {
                continue;
            }

            return new_call_id;
        }
    }

    fn register_call(&self, call_id: u8) -> mpsc::Receiver<ReplyMessage> {
        let (tx, rx) = mpsc::channel(1);
        self.call_tx_map.insert(call_id, tx);
        rx
    }

    fn remove_call(&self, call_id: u8) -> Option<mpsc::Sender<ReplyMessage>> {
        self.call_tx_map.remove(&call_id).map(|entry| entry.1)
    }
}

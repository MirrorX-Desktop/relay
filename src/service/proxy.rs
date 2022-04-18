use std::sync::Arc;

use crate::{
    component::online::ClientManager,
    network::{
        client::Client,
        packet::{Packet, RequestPacket},
    },
};

use super::message::reply_error::ReplyError;

pub struct ProxyService {
    client_manager: Arc<ClientManager>,
}

impl ProxyService {
    pub fn new(client_manager: Arc<ClientManager>) -> Self {
        ProxyService { client_manager }
    }

    pub async fn proxy(
        &self,
        client: Arc<Client>,
        to_device_id: String,
        packet: Packet,
    ) -> anyhow::Result<(), ReplyError> {
        let to_client = if let Some(client) = self.client_manager.find(&to_device_id) {
            client
        } else {
            return Err(ReplyError::DeviceNotFound);
        };

        to_client
            .send(packet)
            .await
            .map_err(|err| ReplyError::Internal)
    }
}

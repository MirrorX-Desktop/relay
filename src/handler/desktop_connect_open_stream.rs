use std::{sync::Arc, time::Duration};

use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    component::online::ONLINE_CLIENTS,
    network::{
        message::{Message, MessageError},
        Client,
    },
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DesktopConnectOpenStreamReq {
    offer_device_id: String,
    ask_device_id: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DesktopConnectOpenStreamResp {}

impl DesktopConnectOpenStreamReq {
    pub async fn handle(self, _: Arc<Client>) -> anyhow::Result<Message, MessageError> {
        info!("handle desktop_connect_open_stream: {:?}", self);

        let online_clients = ONLINE_CLIENTS.read().await;
        let ask_client = match online_clients.get(&self.ask_device_id) {
            Some(client) => client.clone(),
            None => {
                return Err(MessageError::RemoteClientOfflineOrNotExist);
            }
        };

        drop(online_clients);

        let open_stream_resp_message = match ask_client
            .call(
                Message::DesktopConnectOpenStreamReq(DesktopConnectOpenStreamReq {
                    offer_device_id: self.offer_device_id,
                    ask_device_id: self.ask_device_id,
                }),
                Duration::from_secs(10),
            )
            .await?
        {
            Message::DesktopConnectOpenStreamResp(message) => message,
            _ => return Err(MessageError::MismatchedResponseMessage),
        };

        Ok(Message::DesktopConnectOpenStreamResp(
            open_stream_resp_message,
        ))
    }
}

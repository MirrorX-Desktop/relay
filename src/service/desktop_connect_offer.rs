use crate::{
    component::online::ONLINE_CLIENTS,
    service::desktop_connect_ask::DesktopConnectAskReq,
    network::{
        message::{Message, MessageError},
        Client,
    },
};
use log::info;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DesktopConnectOfferReq {
    pub offer_device_id: String,
    pub ask_device_id: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DesktopConnectOfferResp {
    pub agree: bool,
    pub password_auth_public_key_n: Vec<u8>,
    pub password_auth_public_key_e: Vec<u8>,
}

impl DesktopConnectOfferReq {
    pub async fn handle(self, client: Arc<Client>) -> anyhow::Result<Message, MessageError> {
        info!("handle desktop_connect_offer: {:?}", self);

        let online_clients = ONLINE_CLIENTS.read().await;
        let ask_client = match online_clients.get(&self.ask_device_id) {
            Some(client) => client.clone(),
            None => {
                return Err(MessageError::RemoteClientOfflineOrNotExist);
            }
        };

        drop(online_clients);

        let ask_resp_message = match ask_client
            .call(
                Message::DesktopConnectAskReq(DesktopConnectAskReq {
                    offer_device_id: self.offer_device_id,
                }),
                Duration::from_secs(10),
            )
            .await?
        {
            Message::DesktopConnectAskResp(message) => message,
            _ => return Err(MessageError::MismatchedResponseMessage),
        };

        Ok(Message::DesktopConnectOfferResp(DesktopConnectOfferResp {
            agree: ask_resp_message.agree,
            password_auth_public_key_n: ask_resp_message.password_auth_public_key_n,
            password_auth_public_key_e: ask_resp_message.password_auth_public_key_e,
        }))
    }
}

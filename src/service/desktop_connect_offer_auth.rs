use std::{sync::Arc, time::Duration};

use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    component::online::ONLINE_CLIENTS,
    service::desktop_connect_ask_auth::DesktopConnectAskAuthReq,
    network::{
        message::{Message, MessageError},
        Client,
    },
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DesktopConnectOfferAuthReq {
    pub offer_device_id: String,
    pub ask_device_id: String,
    pub secret_message: Vec<u8>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DesktopConnectOfferAuthResp {
    pub password_correct: bool,
}

impl DesktopConnectOfferAuthReq {
    pub async fn handle(self, client: Arc<Client>) -> anyhow::Result<Message, MessageError> {
        info!("handle desktop_connect_offer_auth_req: {:?}", self);

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
                Message::DesktopConnectAskAuthReq(DesktopConnectAskAuthReq {
                    offer_device_id: self.offer_device_id,
                    secret_message: self.secret_message,
                }),
                Duration::from_secs(10),
            )
            .await?
        {
            Message::DesktopConnectAskAuthResp(message) => message,
            _ => return Err(MessageError::MismatchedResponseMessage),
        };

        Ok(Message::DesktopConnectOfferAuthResp(
            DesktopConnectOfferAuthResp {
                password_correct: ask_resp_message.password_correct,
            },
        ))
    }
}

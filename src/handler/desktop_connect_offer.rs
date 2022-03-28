use crate::{
    component::online::ONLINE_CLIENTS,
    handler::desktop_connect_ask::DesktopConnectAskReq,
    network::{message::Message, Client},
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
}

impl DesktopConnectOfferReq {
    pub async fn handle(self, client: Arc<Client>) -> anyhow::Result<Option<Message>> {
        info!("handle desktop_connect_offer: {:?}", self);

        let online_clients = ONLINE_CLIENTS.read().await;
        let ask_client = match online_clients.get(&self.ask_device_id) {
            Some(client) => client.clone(),
            None => {
                return Ok(Some(Message::DesktopConnectOfferResp(
                    DesktopConnectOfferResp { agree: false },
                )));
            }
        };

        drop(online_clients);

        let ask_resp = ask_client
            .call(
                Message::DesktopConnectAskReq(DesktopConnectAskReq {
                    offer_device_id: self.offer_device_id.clone(),
                }),
                Duration::from_secs(10),
            )
            .await?;

        let ask_resp_message = match ask_resp {
            Message::DesktopConnectAskResp(message) => message,
            _ => {
                return Err(anyhow::anyhow!(
                    "handle_desktop_connect_offer: mismatched response message"
                ));
            }
        };

        Ok(Some(Message::DesktopConnectOfferResp(
            DesktopConnectOfferResp {
                agree: ask_resp_message.agree,
            },
        )))
    }
}

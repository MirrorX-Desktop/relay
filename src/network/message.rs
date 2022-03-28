use crate::component;

use super::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Message {
    DeviceGoesOnlineReq(crate::handler::device_goes_online::DeviceGoesOnlineReq),
    DeviceGoesOnlineResp(crate::handler::device_goes_online::DeviceGoesOnlineResp),
    HeartBeatReq(crate::handler::heart_beat::HeartBeatReq),
    HeartBeatResp(crate::handler::heart_beat::HeartBeatResp),
    DesktopConnectOfferReq(crate::handler::desktop_connect_offer::DesktopConnectOfferReq),
    DesktopConnectOfferResp(crate::handler::desktop_connect_offer::DesktopConnectOfferResp),
    DesktopConnectAskReq(crate::handler::desktop_connect_ask::DesktopConnectAskReq),
    DesktopConnectAskResp(crate::handler::desktop_connect_ask::DesktopConnectAskResp),
}

impl Message {
    pub async fn handle(
        self,
        client: Arc<Client>,
        store: Arc<dyn component::store::Store>,
    ) -> anyhow::Result<Option<Message>> {
        match self {
            Message::DeviceGoesOnlineReq(message) => message.handle(client, store).await,
            Message::HeartBeatReq(message) => message.handle().await,
            Message::DesktopConnectOfferReq(message) => message.handle(client).await,
            _ => Ok(None),
        }
    }
}

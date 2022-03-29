use super::Client;
use crate::component;
use crate::handler::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum Message {
    None,
    Error(MessageError),
    DeviceGoesOnlineReq(device_goes_online::DeviceGoesOnlineReq),
    DeviceGoesOnlineResp(device_goes_online::DeviceGoesOnlineResp),
    HeartBeatReq(heart_beat::HeartBeatReq),
    HeartBeatResp(heart_beat::HeartBeatResp),
    DesktopConnectOfferReq(desktop_connect_offer::DesktopConnectOfferReq),
    DesktopConnectOfferResp(desktop_connect_offer::DesktopConnectOfferResp),
    DesktopConnectAskReq(desktop_connect_ask::DesktopConnectAskReq),
    DesktopConnectAskResp(desktop_connect_ask::DesktopConnectAskResp),
    DesktopConnectOfferAuthReq(desktop_connect_offer_auth::DesktopConnectOfferAuthReq),
    DesktopConnectOfferAuthResp(desktop_connect_offer_auth::DesktopConnectOfferAuthResp),
    DesktopConnectAskAuthReq(desktop_connect_ask_auth::DesktopConnectAskAuthReq),
    DesktopConnectAskAuthResp(desktop_connect_ask_auth::DesktopConnectAskAuthResp),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum MessageError {
    InternalError,
    CallTimeout,
    InvalidArguments,
    MismatchedResponseMessage,
    RemoteClientOfflineOrNotExist,
}

impl Message {
    pub async fn handle(
        self,
        client: Arc<Client>,
        store: Arc<dyn component::store::Store>,
    ) -> anyhow::Result<Message, MessageError> {
        match self {
            Message::DeviceGoesOnlineReq(message) => message.handle(client, store).await,
            Message::HeartBeatReq(message) => message.handle().await,
            Message::DesktopConnectOfferReq(message) => message.handle(client).await,
            Message::DesktopConnectOfferAuthReq(message) => message.handle(client).await,
            _ => Ok(Message::None),
        }
    }
}

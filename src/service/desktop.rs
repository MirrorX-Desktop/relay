use super::message::{reply::ConnectReply, reply_error::ReplyError, request::ConnectRequest};
use crate::{
    component::online::ClientManager,
    network::client::Client,
    service::message::{reply::ReplyMessage, request::RequestMessage},
};
use log::info;
use std::{sync::Arc, time::Duration};

pub struct DesktopService {
    client_manager: Arc<ClientManager>,
}

impl DesktopService {
    pub fn new(client_manager: Arc<ClientManager>) -> Self {
        DesktopService { client_manager }
    }

    pub async fn connect(
        &self,
        client: Arc<Client>,
        req: ConnectRequest,
    ) -> Result<ConnectReply, ReplyError> {
        info!("handle connect, client: {}", client.device_id());

        let ask_client = match self.client_manager.find(&req.ask_device_id) {
            Some(client) => client,
            None => return Err(ReplyError::DeviceNotFound),
        };

        let reply_message = ask_client
            .call(RequestMessage::ConnectRequest(req), Duration::from_secs(10))
            .await?;

        if let ReplyMessage::ConnectReply(message) = reply_message {
            Ok(message)
        } else {
            Err(ReplyError::Internal)
        }
    }
}

use std::{sync::Arc, time::Duration};

use log::info;

use crate::{
    component::{online::ClientManager, store::Store},
    network::Client,
    service::message::request::RequestMessage,
};

use super::message::{reply::ConnectReply, reply_error::ReplyError, request::ConnectRequest};

pub struct DesktopService {
    store: Arc<dyn Store>,
    client_manager: Arc<ClientManager>,
}

impl DesktopService {
    pub fn new(store: Arc<dyn Store>, client_manager: Arc<ClientManager>) -> Self {
        DesktopService {
            store,
            client_manager,
        }
    }

    pub async fn connect(
        &self,
        client: Arc<Client>,
        req: ConnectRequest,
    ) -> Result<ConnectReply, ReplyError> {
        info!("handle connect");

        let ask_client = match self.client_manager.find(&req.ask_device_id) {
            Some(client) => client,
            None => return Err(ReplyError::DeviceNotFound),
        };

        ask_client
            .call(RequestMessage::ConnectRequest(req), Duration::from_secs(10))
            .await?;

        Err(ReplyError::DeviceNotFound)
    }
}

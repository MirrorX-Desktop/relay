use super::message::{
    reply::{HeartBeatReply, RegisterDeviceIdReply},
    reply_error::ReplyError,
    request::{HeartBeatRequest, RegisterDeviceIdRequest},
};
use crate::{
    component::{self, online::ClientManager, store::Store},
    network::client::Client,
};
use log::{debug, error, info};
use std::sync::Arc;

pub struct DeviceService {
    store: Arc<dyn Store>,
    client_manager: Arc<ClientManager>,
}

impl DeviceService {
    pub fn new(store: Arc<dyn Store>, client_manager: Arc<ClientManager>) -> Self {
        DeviceService {
            store,
            client_manager,
        }
    }

    pub async fn heart_beat(
        &self,
        client: Arc<Client>,
        req: HeartBeatRequest,
    ) -> Result<HeartBeatReply, ReplyError> {
        debug!(
            "handle heart_beat, client: {}, timestamp: {}",
            client.device_id(),
            req.time_stamp
        );

        Ok(HeartBeatReply {
            time_stamp: chrono::Utc::now().timestamp() as u32,
        })
    }

    pub async fn register_id(
        &self,
        client: Arc<Client>,
        req: RegisterDeviceIdRequest,
    ) -> Result<RegisterDeviceIdReply, ReplyError> {
        info!("handle register_id");

        if let Some(device_id) = &req.device_id {
            match self.store.device_id_renew(device_id) {
                Ok(Some(expire_at)) => {
                    client.set_device_id(device_id.clone());

                    self.client_manager.add(device_id.clone(), client);

                    return Ok(RegisterDeviceIdReply {
                        device_id: device_id.to_string(),
                        expire_at,
                    });
                }
                Err(err) => {
                    error!("register_id: {:?}", err);
                    return Err(ReplyError::Internal);
                }
                _ => {}
            };
        }

        // allocate a new device id

        let mut failure_counter = 0;

        loop {
            // alphabet without 0, O, I, L
            let new_device_id = component::device_id::generate_device_id();

            match self.store.set_device_id(&new_device_id) {
                Ok(Some(expire_at)) => {
                    client.set_device_id(new_device_id.clone());

                    self.client_manager.add(new_device_id.clone(), client);

                    return Ok(RegisterDeviceIdReply {
                        device_id: new_device_id.to_string(),
                        expire_at,
                    });
                }
                Ok(None) => continue,
                Err(err) => {
                    // only error increase fail counter
                    failure_counter += 1;
                    if failure_counter < 10 {
                        continue;
                    }

                    error!("register_id: too many failures, lastest error: {:?}", err);
                    return Err(ReplyError::Internal);
                }
            };
        }
    }
}

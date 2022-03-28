use std::sync::Arc;

use crate::component::{self, online::ONLINE_CLIENTS};
use crate::network::message::Message;
use crate::network::Client;
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DeviceGoesOnlineReq {
    pub device_id: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DeviceGoesOnlineResp {
    pub device_id: String,
    pub device_id_expire_time_stamp: u32,
}

impl DeviceGoesOnlineReq {
    pub async fn handle(
        self,
        client: Arc<Client>,
        store: Arc<dyn component::store::Store>,
    ) -> anyhow::Result<Option<Message>> {
        info!("handle device_goes_online: {:?}", self);

        if let Some(device_id) = &self.device_id {
            if let Some(new_expire_time_stamp) = store.device_id_renew(device_id)? {
                client.set_device_id(device_id.clone());

                let mut online_clients = ONLINE_CLIENTS.write().await;
                online_clients.insert(device_id.to_string(), client);
                drop(online_clients);

                return Ok(Some(Message::DeviceGoesOnlineResp(DeviceGoesOnlineResp {
                    device_id: device_id.to_string(),
                    device_id_expire_time_stamp: new_expire_time_stamp,
                })));
            }
        }

        // allocate a new device id

        let mut failure_counter = 0;

        loop {
            // alphabet without 0, O, I, L
            let new_device_id = component::device_id::generate_device_id();

            match store.set_device_id(&new_device_id) {
                Ok(Some(expire_time_stamp)) => {
                    client.set_device_id(new_device_id.clone());

                    let mut online_clients = ONLINE_CLIENTS.write().await;
                    online_clients.insert(new_device_id.to_string(), client);
                    drop(online_clients);

                    return Ok(Some(Message::DeviceGoesOnlineResp(DeviceGoesOnlineResp {
                        device_id: new_device_id,
                        device_id_expire_time_stamp: expire_time_stamp,
                    })));
                }
                Ok(None) => continue,
                Err(_) => {
                    // only error increase fail counter
                    failure_counter += 1;
                    if failure_counter < 5 {
                        continue;
                    }
                    return Err(anyhow::anyhow!("set_device_id: too many failures"));
                }
            };
        }
    }
}

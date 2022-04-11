use std::sync::Arc;

use log::error;
use tonic::{Request, Response, Status};

use crate::{
    component,
    session::{Manager, Session},
};

use super::rpc::{device_server::Device, RegisterIdRequest, RegisterIdResponse};

pub struct DeviceService {
    session_mgr: Arc<Manager>,
    store: Arc<dyn component::store::Store>,
}

impl DeviceService {
    pub fn new(session_mgr: Arc<Manager>, store: Arc<dyn component::store::Store>) -> Self {
        DeviceService { session_mgr, store }
    }
}

#[tonic::async_trait]
impl Device for DeviceService {
    async fn register_id(
        &self,
        request: Request<RegisterIdRequest>,
    ) -> Result<Response<RegisterIdResponse>, Status> {
        let register_id_request = request.into_inner();
        if register_id_request.device_id.is_empty() {
            match self.store.device_id_renew(&register_id_request.device_id) {
                Ok(Some(expire_at)) => {
                    let session = Session::new(register_id_request.device_id.clone());
                    self.session_mgr.add(session);

                    return Ok(Response::new(RegisterIdResponse {
                        device_id: register_id_request.device_id.clone(),
                        expire_at,
                    }));
                }
                Err(err) => {
                    error!("renew device_id error: {:?}", err);
                    return Err(Status::internal("renew device_id failed"));
                }
                _ => {
                    // device_id not exist in pool, allocate a new one
                }
            };
        }

        let mut failure_counter = 0;

        loop {
            // alphabet without 0, O, I, L
            let new_device_id = component::device_id::generate_device_id();

            match self.store.set_device_id(&new_device_id) {
                Ok(Some(expire_at)) => {
                    let session = Session::new(register_id_request.device_id.clone());
                    self.session_mgr.add(session);

                    return Ok(Response::new(RegisterIdResponse {
                        device_id: register_id_request.device_id.clone(),
                        expire_at,
                    }));
                }
                Ok(None) => continue,
                Err(err) => {
                    // only error increase fail counter
                    failure_counter += 1;
                    if failure_counter < 5 {
                        continue;
                    }

                    error!(
                        "allocate new device_id whith too many failures, the lastest error: {:?}",
                        err
                    );
                    return Err(Status::internal("too many failures"));
                }
            };
        }
    }
}

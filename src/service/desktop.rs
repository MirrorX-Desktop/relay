use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::session::{Manager, ManagerError};

use super::rpc::desktop_server::Desktop;
use super::rpc::{
    ConnectAuthorizationRequest, ConnectAuthorizationResponse, DesktopConnectRequest,
    DesktopConnectResponse,
};
use futures::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};

pub struct DesktopService {
    ses_mgr: Arc<Manager>,
}

impl DesktopService {
    pub fn new(ses_mgr: Arc<Manager>) -> Self {
        DesktopService { ses_mgr }
    }
}

#[tonic::async_trait]
impl Desktop for DesktopService {
    async fn connect(
        &self,
        request: Request<DesktopConnectRequest>,
    ) -> Result<Response<DesktopConnectResponse>, Status> {
        let desktop_connect_request = request.into_inner();
        if !self
            .ses_mgr
            .exist(&desktop_connect_request.ask_device_id)
            .await
        {
            return Err(Status::not_found("remote device not found"));
        }

        let resp: DesktopConnectResponse = self
            .ses_mgr
            .request(
                &desktop_connect_request.ask_device_id.to_owned(),
                desktop_connect_request,
                Duration::from_secs(10),
            )
            .await
            .map_err(|err| match err {
                ManagerError::Timeout => Status::deadline_exceeded("request timeout"),
                _ => Status::internal("internal error"),
            })?;

        Ok(Response::new(resp))
    }

    type SubscribeConnectRequestStream =
        Pin<Box<dyn Stream<Item = Result<DesktopConnectRequest, Status>> + Send>>;

    async fn subscribe_connect_request(
        &self,
        request: Request<Streaming<DesktopConnectResponse>>,
    ) -> Result<Response<Self::SubscribeConnectRequestStream>, Status> {
        let device_id = match request.metadata().get("device_id") {
            Some(device_id) => match device_id.to_str() {
                Ok(device_id_str) => String::from(device_id_str),
                Err(err) => return Err(Status::internal("message")),
            },
            None => return Err(Status::not_found("device not found")),
        };

        let mut in_stream = request.into_inner();
        tokio::spawn(async move { while let Some(result) = in_stream.next().await {} });

        let out_stream = self
            .ses_mgr
            .subscribe_request::<DesktopConnectRequest>(&device_id)
            .unwrap();

        Ok(Response::new(
            Box::pin(out_stream) as Self::SubscribeConnectRequestStream
        ))
    }

    async fn connect_authorization(
        &self,
        request: Request<ConnectAuthorizationRequest>,
    ) -> Result<Response<ConnectAuthorizationResponse>, Status> {
        todo!()
    }

    type SubscribeConnectAuthorizationRequestStream =
        ReceiverStream<Result<ConnectAuthorizationRequest, Status>>;

    async fn subscribe_connect_authorization_request(
        &self,
        request: Request<Streaming<ConnectAuthorizationResponse>>,
    ) -> Result<Response<Self::SubscribeConnectAuthorizationRequestStream>, Status> {
        todo!()
    }
}

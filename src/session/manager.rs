use std::{
    any::{Any, TypeId},
    time::Duration,
};

use async_stream::AsyncStream;
use futures::Future;
use tokio::sync::mpsc;
use tonic::Status;

use super::session::Session;

pub struct Manager {
    sessions: dashmap::DashMap<String, Session>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            sessions: dashmap::DashMap::new(),
        }
    }

    pub async fn add(&self, session: Session) {
        self.sessions
            .insert(session.device_id().to_owned(), session);
    }

    pub async fn exist(&self, device_id: &str) -> bool {
        self.sessions.contains_key(device_id)
    }

    pub async fn request<Request, Reply>(
        &self,
        to_device_id: &str,
        request: Request,
        timeout: Duration,
    ) -> anyhow::Result<Reply, ManagerError>
    where
        Request: prost::Message + 'static,
        Reply: prost::Message + 'static,
    {
        let session = match self.sessions.get(to_device_id) {
            Some(session) => session,
            None => return Err(ManagerError::SessionNotFound),
        };

        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        session.request(request, reply_tx, timeout).await;

        let proto_message = reply_rx.recv().await.ok_or_else(|| ManagerError::Timeout)?;

        if TypeId::of::<Reply>() == proto_message.type_id() {
            return Err(ManagerError::CastFailed);
        }

        unsafe {
            let reply_message: Reply = std::mem::transmute_copy(&proto_message);
            Ok(reply_message)
        }
    }

    pub fn subscribe_request<T>(
        &self,
        device_id: &str,
    ) -> Option<AsyncStream<Result<T, Status>, impl Future<Output = ()>>>
    where
        T: prost::Message + 'static,
    {
        self.sessions.get(device_id).and_then(|session| {
            session.subscribe_request::<T>().map_or(None, |mut rx| {
                let stream = async_stream::stream! {
                    while let Some(item) = rx.recv().await {
                        if item.type_id()  == TypeId::of::<T>(){
                            unsafe {
                                let reply_message: T = std::mem::transmute_copy(&item);
                                yield Ok(reply_message);
                            }
                        } else {
                            yield Err(Status::internal("message"))
                        }
                    }
                };
                Some(stream)
            })
        })
    }

    // pub async fn publish(&self, to_device_id: &str, event: ManagerEvent) {
    //     self.sessions.get(to_device_id).map(|v| v.publish(event));
    // }

    // pub async fn subscribe(
    //     &self,
    //     device_id: &str,
    // ) -> Option<tokio::sync::mpsc::UnboundedReceiver<ManagerEvent>> {
    //     self.sessions.get(device_id).map(|v| v.subscribe())
    // }
}

pub enum ManagerError {
    SessionNotFound,
    RequestMustHasNoneZeroTimeoutDuration,
    Timeout,
    CastFailed,
}

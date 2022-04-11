use std::{
    any::TypeId,
    sync::{atomic::AtomicU8, Arc},
    time::Duration,
};

use tokio::sync::mpsc;

pub struct Session {
    device_id: String,
    subscribe_request_map: Arc<dashmap::DashMap<TypeId, mpsc::Sender<Box<dyn prost::Message>>>>,
    reply_map: Arc<dashmap::DashMap<u8, mpsc::Sender<Box<dyn prost::Message>>>>,
    request_id: AtomicU8,
}

impl Session {
    pub fn new(device_id: String) -> Self {
        Session {
            device_id,
            subscribe_request_map: Arc::new(dashmap::DashMap::new()),
            reply_map: Arc::new(dashmap::DashMap::new()),
            request_id: AtomicU8::new(0),
        }
    }

    /// Get a reference to the session's device id.
    #[must_use]
    pub fn device_id(&self) -> &str {
        self.device_id.as_ref()
    }

    pub async fn request<T>(
        &self,
        request: T,
        reply_tx: mpsc::Sender<Box<dyn prost::Message>>,
        timeout: Duration,
    ) -> anyhow::Result<()>
    where
        T: prost::Message + 'static,
    {
        let type_id = TypeId::of::<T>();
        match self.subscribe_request_map.get(&type_id) {
            Some(v) => {
                let request_id = self
                    .request_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                self.reply_map.insert(request_id, reply_tx);
                let reply_tx_map_arc = self.reply_map.clone();

                v.send(Box::new(request))
                    .await
                    .and_then(|_| {
                        tokio::spawn(async move {
                            tokio::time::sleep(timeout).await;
                            reply_tx_map_arc.remove(&request_id);
                        });
                        Ok(())
                    })
                    .map_err(|err| anyhow::anyhow!(""))
            }
            None => Err(anyhow::anyhow!("")),
        }
    }

    pub fn reply(&self, reply_id: u8, message: Box<dyn prost::Message>) {
        self.reply_map
            .remove(&reply_id)
            .map(|v| v.1.try_send(message));
    }

    pub fn subscribe_request<T>(&self) -> anyhow::Result<mpsc::Receiver<Box<dyn prost::Message>>>
    where
        T: prost::Message + 'static,
    {
        let type_id = TypeId::of::<T>();
        if self.subscribe_request_map.contains_key(&type_id) {
            Err(anyhow::anyhow!("already subscribed"))
        } else {
            let (tx, rx) = mpsc::channel(16);
            self.subscribe_request_map.insert(type_id, tx);
            Ok(rx)
        }
    }

    // pub fn publish(&self, event: ManagerEvent) {
    //     self.tx.send(event);
    // }

    // pub fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<ManagerEvent> {
    //     self.subscribe()
    // }
}

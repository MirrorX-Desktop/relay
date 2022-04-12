use crate::network::client::Client;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ClientManager {
    clients: DashMap<String, Arc<Client>>,
}

impl ClientManager {
    pub fn new() -> Self {
        ClientManager {
            clients: DashMap::new(),
        }
    }

    pub fn add(&self, device_id: String, client: Arc<Client>) {
        self.clients.insert(device_id, client);
    }

    pub fn find(&self, device_id: &str) -> Option<Arc<Client>> {
        match self.clients.get(device_id) {
            Some(client) => Some(client.clone()),
            None => None,
        }
    }

    pub fn remove(&self, device_id: &str) -> Option<Arc<Client>> {
        self.clients
            .remove(device_id)
            .map_or(None, |entry| Some(entry.1))
    }
}

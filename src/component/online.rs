use crate::network::Client;
use lazy_static::lazy_static;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

lazy_static! {
    pub static ref ONLINE_CLIENTS: RwLock<HashMap<String, Arc<Client>>> =
        RwLock::new(HashMap::new());
}

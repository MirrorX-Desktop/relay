use std::sync::Arc;

use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::network::client::Client;

pub static CLIENT_MANAGER: Lazy<DashMap<String, Arc<Client>>> = Lazy::new(|| DashMap::new());

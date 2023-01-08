use serde::Serialize;

#[derive(Serialize)]
pub struct StatResponse {
    pub bytes_transferred: u64,
    pub client_snapshot: Vec<StreamingClientStat>,
}

#[derive(Serialize, Clone)]
pub struct StreamingClientStat {
    pub active_device_id: i64,
    pub active_addr: String,
    pub passive_device_id: i64,
    pub passive_addr: String,
    pub timestamp: i64,
}

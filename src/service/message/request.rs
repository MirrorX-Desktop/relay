use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RequestMessage {
    HeartBeatRequest(HeartBeatRequest),
    RegisterIdRequest(RegisterRequest),
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct HeartBeatRequest {
    pub time_stamp: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegisterRequest {
    pub device_id: Option<String>,
}

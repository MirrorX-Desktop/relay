use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum RequestMessage {
    HeartBeatRequest(HeartBeatRequest),
    RegisterDeviceIdRequest(RegisterDeviceIdRequest),
    ConnectRequest(ConnectRequest),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct RegisterDeviceIdRequest {
    pub device_id: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct HeartBeatRequest {
    pub time_stamp: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ConnectRequest {
    pub offer_device_id: String,
    pub ask_device_id: String,
}

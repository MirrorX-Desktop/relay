use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum ReplyMessage {
    HeartBeatReply(HeartBeatReply),
    RegisterDeviceIdReply(RegisterDeviceIdReply),
    ConnectReply(ConnectReply),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct HeartBeatReply {
    pub time_stamp: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct RegisterDeviceIdReply {
    pub device_id: String,
    pub expire_at: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct ConnectReply {
    pub offer_device_id: String,
    pub ask_device_id: String,
    pub pub_key_n: u64,
    pub pub_key_e: u64,
}

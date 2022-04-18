use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ReplyMessage {
    HeartBeatReply(HeartBeatReply),
    RegisterIdReply(RegisterIdReply),
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct HeartBeatReply {
    pub time_stamp: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegisterIdReply {
    pub device_id: String,
    pub expire_at: u32,
}

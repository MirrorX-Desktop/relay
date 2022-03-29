use crate::network::message::{Message, MessageError};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct HeartBeatReq {
    pub time_stamp: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct HeartBeatResp {
    pub time_stamp: u32,
}

impl HeartBeatReq {
    pub async fn handle(self) -> anyhow::Result<Message, MessageError> {
        info!("handle heart_beat: {:?}", self);

        Ok(Message::HeartBeatResp(HeartBeatResp {
            time_stamp: self.time_stamp,
        }))
    }
}

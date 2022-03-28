use crate::network::message::Message;
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
    pub async fn handle(self) -> anyhow::Result<Option<Message>> {
        info!("handle heart_beat: {:?}", self);

        Ok(Some(Message::HeartBeatResp(HeartBeatResp {
            time_stamp: self.time_stamp,
        })))
    }
}

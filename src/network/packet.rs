use serde::{Deserialize, Serialize};

use crate::service::message::{reply::ReplyMessage, request::RequestMessage};

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestPacket {
    pub call_id: u8,
    pub request_message: RequestMessage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReplyPacket {
    pub call_id: u8,
    pub reply_message: ReplyMessage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Packet {
    pub request_packet: Option<RequestPacket>,
    pub reply_packet: Option<ReplyPacket>,
}

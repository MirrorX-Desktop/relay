use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DesktopConnectAskReq {
    pub offer_device_id: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct DesktopConnectAskResp {
    pub agree: bool,
    pub password_auth_public_key_n: Vec<u8>,
    pub password_auth_public_key_e: Vec<u8>,
}

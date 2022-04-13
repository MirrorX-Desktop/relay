use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ReplyError {
    Internal,
    Timeout,
    DeviceNotFound,
    CastFailed,
}

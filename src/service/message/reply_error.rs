use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ReplyError {
    Internal,
    Timeout,
    DeviceNotFound,
    CastFailed,
    RepeatedRequest,
}

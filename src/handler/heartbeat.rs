use crate::network::{
    client::Client, client_to_server::HeartBeatRequest, server_to_client::HeartBeatReply,
};
use log::info;
use std::sync::Arc;

pub async fn heartbeat(
    client: Arc<Client>,
    req: HeartBeatRequest,
) -> anyhow::Result<HeartBeatReply> {
    info!("heartbeat: {:?}", req);

    Ok(HeartBeatReply {
        time_stamp: req.time_stamp,
    })
}

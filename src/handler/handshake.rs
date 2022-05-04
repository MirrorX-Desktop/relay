use crate::{
    instance::client_manager::CLIENT_MANAGER,
    network::{
        client::Client,
        client_to_server::HandshakeRequest,
        server_to_client::{HandshakeReply, HandshakeStatus},
    },
};
use anyhow::bail;
use log::info;
use std::sync::Arc;

pub async fn handshake(
    client: Arc<Client>,
    req: HandshakeRequest,
) -> anyhow::Result<HandshakeReply> {
    info!("handshake: {:?}", req);

    let splited: Vec<&str> = req.token.split(".").collect();
    if splited.len() != 3 {
        bail!("parse_register_token: token format is invalid");
    }

    let device_id = splited[0];

    client.set_device_id(device_id.to_owned())?;

    // todo: check and insert should be automic

    if CLIENT_MANAGER.contains_key(device_id) {
        return Ok(HandshakeReply {
            status: HandshakeStatus::Repeated,
        });
    }

    CLIENT_MANAGER.insert(device_id.to_owned(), client);
    Ok(HandshakeReply {
        status: HandshakeStatus::Accepted,
    })
}

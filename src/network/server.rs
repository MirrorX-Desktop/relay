use super::Client;
use crate::component;
use rustls::{Certificate, PrivateKey};
use rustls_pemfile::certs;
use std::{fs::File, io::BufReader, sync::Arc};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

pub async fn server() -> anyhow::Result<()> {
    let cert_file = certs(&mut BufReader::new(File::open("cert/example.com.crt")?))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())?;

    let mut key_file: Vec<PrivateKey> = rustls_pemfile::pkcs8_private_keys(&mut BufReader::new(
        File::open("cert/example.com.key")?,
    ))
    .map(|mut certs| certs.drain(..).map(PrivateKey).collect())?;

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_file, key_file.remove(0))?;

    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind("0.0.0.0:45555").await?;

    let store = component::store::new_store(true)?;

    loop {
        let (stream, _) = listener.accept().await?;

        let acceptor = acceptor.clone();
        let stream = acceptor.accept(stream).await?;

        let store_clone = store.clone();

        tokio::spawn(async move {
            Client::serve(stream, store_clone).await;
        });
    }
}

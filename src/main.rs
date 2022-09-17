use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use log::info;
use tokio::sync::{mpsc, RwLock};
use warp::ws::Message;

mod cipher;
mod websocket;
mod filters;
mod access;

type ActiveConnections = Arc<RwLock<HashMap<u64, mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting {} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let active_connections = ActiveConnections::default();

    let key = cipher::load_or_create_key();

    let routes = filters::routes(key, active_connections);

    let addr = IpAddr::V6(Ipv6Addr::from(0_u128));

    match cipher::load_tls_cert() {
        Some(credentials) => {
            info!("TLS encrypted! Cert: {}", credentials.cert.display());
            warp::serve(routes)
                .tls()
                .cert_path(credentials.cert)
                .key_path(credentials.key)
                .run(SocketAddr::new(addr, 8443)).await;
        }
        None => {
            warp::serve(routes)
                .run(SocketAddr::new(addr, 8080)).await;
        }
    }
}

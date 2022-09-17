use std::sync::Arc;

use warp::{Filter, Reply};

use crate::{ActiveConnections, cipher, cipher::Key, websocket};
use crate::access::UserId;

pub fn routes(key: Key,
              active_connections: ActiveConnections)
              -> impl Filter<Extract=(impl Reply, ), Error=warp::Rejection> + Clone {
    index()
        .or(websocket(key.clone(), Arc::clone(&active_connections)))
        .or(post_data(key.clone(), Arc::clone(&active_connections)))
        .or(create(key.clone()))
        .with(warp::log("access"))
}

fn index()
    -> impl Filter<Extract=(impl Reply, ), Error=warp::Rejection> + Clone {
    warp::path::end().map(|| "OK")
}

fn websocket(key: Key,
             active_connections: ActiveConnections)
             -> impl Filter<Extract=(impl Reply, ), Error=warp::Rejection> + Clone
{
    version()
        .and(id_restricted_path(String::from("ws"), key))
        .and(warp::ws())
        .map(move |user_id: UserId, ws: warp::ws::Ws| {
            let active_connections = Arc::clone(&active_connections);
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| websocket::user_connected(socket, active_connections, user_id))
        })
}

fn post_data(key: Key,
             active_connections: ActiveConnections)
             -> impl Filter<Extract=(impl Reply, ), Error=warp::Rejection> + Clone {
    version()
        .and(warp::post())
        .and(id_restricted_path(String::from("post"), key))
        .and(warp::any().map(move || Arc::clone(&active_connections)))
        .and(warp::body::content_length_limit(2048))
        .and(warp::body::bytes())
        .and_then(websocket::send_message)
}

fn create(key: Key)
          -> impl Filter<Extract=(impl Reply, ), Error=warp::Rejection> + Clone {
    version()
        .and(warp::path("create"))
        .and(warp::get())
        .and(with_key(key))
        .and_then(create_handler)
}

async fn create_handler(key: Key) -> Result<String, warp::Rejection> {
    let string = cipher::create_keys(key)
        .map(|array| array.join("\n"));
    return string;
}

fn version() -> impl Filter<Extract=(), Error=warp::Rejection> + Clone {
    warp::path("v1")
}

fn with_key(key: Key) -> impl Filter<Extract=(Key, ), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || key)
}

fn id_restricted_path(path: String, key: Key)
                      -> impl Filter<Extract=(UserId, ), Error=warp::Rejection> + Clone {
    warp::path(path)
        .and(warp::path::param())
        .and(with_key(key))
        .and_then(decrypt_id_handler)
}

async fn decrypt_id_handler(user_id: String, key: Key) -> Result<UserId, warp::Rejection> {
    return cipher::decrypt_id(user_id, key);
}


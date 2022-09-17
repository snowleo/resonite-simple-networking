use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use futures_util::future::{Either, select};
use futures_util::stream::{SplitSink, SplitStream};
use log::{debug, error, info, warn};
use tokio::pin;
use tokio::sync::{mpsc, mpsc::UnboundedReceiver};
use tokio::time::{Instant, interval};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::http::StatusCode;
use warp::hyper::body::Bytes;
use warp::ws::{Message, WebSocket};

use crate::ActiveConnections;
use crate::access::{AccessCheck, UserId};

pub async fn user_connected(ws: WebSocket, active_connections: ActiveConnections, user_id: UserId) {
    let read_only = user_id.read_only();
    let user_id = user_id.base();
    info!("websocket user connected: {} read_only: {}", user_id, read_only);

    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    if read_only {
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::task::spawn(async move {
            websocket_send_thread(&mut user_ws_tx, rx).await
        });

        active_connections.write().await.insert(user_id, tx);
    }

    read_messages_from_websocket(&mut user_ws_rx, !read_only, &active_connections, user_id).await;

    user_disconnected(user_id, &active_connections).await;
}

async fn read_messages_from_websocket(user_ws_rx: &mut SplitStream<WebSocket>,
                                      should_send_to_queue: bool,
                                      active_connections: &ActiveConnections,
                                      user_id: UserId) {
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("websocket error(uid={}): {}", user_id, e);
                break;
            }
        };
        if should_send_to_queue && msg.is_text() {
            send_message_to_queue(active_connections, user_id, msg.clone()).await;
        }
    }
}

async fn websocket_send_thread(user_ws_tx: &mut SplitSink<WebSocket, Message>,
                               rx: UnboundedReceiver<Message>) {
    let mut rx = UnboundedReceiverStream::new(rx);
    let mut interval = interval(Duration::from_secs(10));

    loop {
        let next_msg = rx.next();
        pin!(next_msg);

        let tick = interval.tick();
        pin!(tick);

        match select(next_msg, tick).await {
            Either::Left((msg, _)) =>
                match send_to_websocket_tx(user_ws_tx, msg).await {
                    Some(_) => {}
                    None => {
                        break;
                    }
                },
            Either::Right((instant, _)) =>
                match send_to_websocket_tx(user_ws_tx, ping_message(instant)).await {
                    Some(_) => {}
                    None => {
                        break;
                    }
                }
        }
    }
    debug!("Closing thread");
}

async fn send_to_websocket_tx(user_ws_tx: &mut SplitSink<WebSocket, Message>, message: Option<Message>) -> Option<()> {
    match message {
        Some(message) => user_ws_tx
            .send(message)
            .await
            .map_err(|e| {
                warn!("websocket send error: {}", e);
                e
            }).ok(),
        None => None
    }
}

fn ping_message(instant: Instant) -> Option<Message> {
    Some(Message::ping(instant.elapsed().as_secs().to_be_bytes()))
}

async fn user_disconnected(user_id: UserId, users: &ActiveConnections) {
    info!("websocket user disconnected: {}", user_id);
    users.write().await.remove(&user_id);
}

pub async fn send_message(user_id: UserId, active_connections: ActiveConnections, body: Bytes) -> Result<StatusCode, warp::Rejection> {
    if user_id.read_only() {
        warn!("User {} with read bit tried to send message", user_id);
        return Ok(StatusCode::FORBIDDEN);
    }
    let msg = Message::text(parse_bytes(body)?);
    match send_message_to_queue(&active_connections, user_id.base(), msg).await {
        Some(_) => Ok(StatusCode::ACCEPTED),
        None => Ok(StatusCode::NOT_FOUND)
    }
}

fn parse_bytes(body: Bytes) -> Result<String, warp::Rejection> {
    return String::from_utf8(body.to_vec())
        .map_err(|e| {
            warn!("Unable to parse bytes {}", e);
            warp::reject::not_found()
        });
}

async fn send_message_to_queue(active_connections: &ActiveConnections, user_id: UserId, msg: Message) -> Option<()> {
    active_connections.read()
        .await
        .get(&user_id)
        .and_then(|tx|
            tx.send(msg)
                .map_err(|e| {
                    warn!("Failed to send message to {}: {:?}", user_id, e);
                }).ok()
        )
}
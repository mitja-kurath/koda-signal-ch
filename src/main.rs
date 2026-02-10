mod protocol;

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use dashmap::DashMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use uuid::Uuid;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use protocol::KodaSignal;

// Use DashMap for high-performance concurrent access in Switzerland
type PeerMap = Arc<DashMap<Uuid, mpsc::UnboundedSender<Message>>>;

#[derive(Clone)]
struct AppState {
    peers: PeerMap,
    jwt_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    let state = AppState {
        peers: Arc::new(DashMap::new()),
        jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
    };

    let app = Router::new()
        .route("/pulse", get(ws_handler))
        .with_state(state);

    let addr = "0.0.0.0:3000";
    println!("Koda Signal Node [ZRH] starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut authenticated_user_id: Option<Uuid> = None;

    // Task 1: Forward messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        let mut ping_interval = time::interval(Duration::from_secs(30));
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    if sender.send(msg).await.is_err() { break; }
                }
                _ = ping_interval.tick() => {
                    if sender.send(Message::Ping(vec![].into())).await.is_err() { break; }
                }
            }
        }
    });

    // Task 2: Receive and Route messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(signal) = serde_json::from_str::<KodaSignal>(&text) {
                match signal {
                    // STEP 1: Identification using the API's JWT
                    KodaSignal::Identify { token } => {
                        let decoding_key = DecodingKey::from_secret(state.jwt_secret.as_bytes());
                        // Use the local Claims struct which matches koda-api
                        if let Ok(token_data) = decode::<Claims>(
                            &token, &decoding_key, &Validation::default()
                        ) {
                            let uid = token_data.claims.sub;
                            authenticated_user_id = Some(uid);
                            state.peers.insert(uid, tx.clone());
                            
                            let _ = tx.send(Message::Text(serde_json::to_string(
                                &KodaSignal::Authenticated { user_id: uid }
                            ).unwrap().into()));
                        }
                    },

                    // STEP 2: Secure Routing
                    KodaSignal::Signal { target_id, data, .. } => {
                        match authenticated_user_id {
                            Some(sender_id) => {
                                // Only route if the target is online
                                if let Some(peer_tx) = state.peers.get(&target_id) {
                                    let routed_msg = KodaSignal::Signal {
                                        target_id,
                                        sender_id: Some(sender_id),
                                        data,
                                    };
                                    let _ = peer_tx.send(Message::Text(
                                        serde_json::to_string(&routed_msg).unwrap().into()
                                    ));
                                } else {
                                    // Let the sender know their friend is offline
                                    let _ = tx.send(Message::Text(serde_json::to_string(
                                        &KodaSignal::PeerOffline { peer_id: target_id }
                                    ).unwrap().into()));
                                }
                            },
                            None => {
                                // Send error if they try to signal without identifying
                                let _ = tx.send(Message::Text(serde_json::to_string(
                                    &KodaSignal::Error { message: "IDENTIFY_REQUIRED".into() }
                                ).unwrap().into()));
                            }
                        }
                    },
                    _ => {}
                }
            } else {
                // Handle Malformatted JSON
                let _ = tx.send(Message::Text(serde_json::to_string(
                    &KodaSignal::Error { message: "MALFORMATTED_JSON".into() }
                ).unwrap().into()));
            }
        }
    }

    // Cleanup: Remove user when they disconnect
    if let Some(uid) = authenticated_user_id {
        state.peers.remove(&uid);
        println!("User {} disconnected from ZRH node", uid);
    }
    send_task.abort();
}

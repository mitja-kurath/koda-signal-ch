use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KodaSignal {
    // 1. Handshake: Client sends JWT immediately upon connecting
    Identify { token: String },
    
    // 2. Signaling: Passing WebRTC/MoQ data
    // target_id is the Friend's UUID from koda-api
    Signal { 
        target_id: Uuid, 
        sender_id: Option<Uuid>, // Filled by the server for security
        data: serde_json::Value  // The actual SDP or ICE candidate
    },

    // 3. System: Server sending updates to the client
    Authenticated { user_id: Uuid },
    PeerOffline { peer_id: Uuid },
    Error { message: String }
}

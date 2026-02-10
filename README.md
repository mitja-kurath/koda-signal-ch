# Koda Signal Node (ZRH)

Professional-grade, stateful WebSocket signaling server for the Koda collaboration platform. This node acts as the secure bridge between Identity (Koda API) and Media (P2P pipes/WebRTC).

## Overview

The Koda Signal Node is a high-performance router designed to handle real-time signaling data. Unlike the stateless API, this node maintains a live map of active connections in memory, allowing for low-latency message routing between authenticated peers.

### Key Features

- **Stateful Routing**: Uses `DashMap` for thread-safe, high-speed concurrent access to active peer connections.
- **Identity Security**: Validates JWTs using the same `JWT_SECRET` as the Koda API.
- **Anti-Spoofing**: Automatically populates `sender_id` from the authenticated session, preventing users from impersonating others.
- **Heartbeat & Cleanup**: Built-in Ping/Pong mechanism to detect and prune "ghost" connections.
- **Robust Protocol**: Tagged JSON protocol for easy consumption by modern frontend frameworks (Angular v21, etc.).

## Technical Stack

- **Language**: Rust (Edition 2024)
- **Framework**: Axum (WebSocket)
- **Concurrency**: Tokio (Async Runtime), DashMap (Concurrent Hash Map)
- **Security**: JWT (jsonwebtoken)
- **Serialization**: Serde (JSON)

## Shared Message Protocol

The node and clients communicate using a specific JSON structure defined by the `KodaSignal` enum.

### Protocol Schema (`SCREAMING_SNAKE_CASE`)

1. **Identify**: Client sends their JWT immediately upon connecting.
   ```json
   { "type": "IDENTIFY", "payload": { "token": "your_jwt_here" } }
   ```
2. **Signal**: Passing WebRTC/MoQ data to a specific peer.
   ```json
   { 
     "type": "SIGNAL", 
     "payload": { 
       "target_id": "friend-uuid", 
       "data": { "sdp": "...", "type": "offer" } 
     } 
   }
   ```
3. **Authenticated**: Server confirms successful identification.
   ```json
   { "type": "AUTHENTICATED", "payload": { "user_id": "your-uuid" } }
   ```
4. **PeerOffline**: Server notifies if the target peer is not connected.
   ```json
   { "type": "PEER_OFFLINE", "payload": { "peer_id": "friend-uuid" } }
   ```
5. **Error**: Server sends error messages (e.g., `IDENTIFY_REQUIRED`, `MALFORMATTED_JSON`).
   ```json
   { "type": "ERROR", "payload": { "message": "..." } }
   ```

## Setup & Configuration

### Prerequisites

- Rust (latest stable)
- Shared `JWT_SECRET` with Koda API

### Environment Variables

Create a `.env` file in the root:

```env
JWT_SECRET=your_super_secret_key
RUST_LOG=koda_signal_ch=debug
```

### Running the Node

```bash
cargo run
```

The node will start on `0.0.0.0:3000`. The signaling endpoint is available at `ws://localhost:3000/pulse`.

## Security Architecture

1. **Handshake**: Clients must connect and immediately send an `IDENTIFY` message.
2. **Verification**: The node decodes the JWT. If invalid, the user remains unauthenticated.
3. **Restricted Actions**: `SIGNAL` messages are rejected with `IDENTIFY_REQUIRED` unless the connection is authenticated.
4. **Verified Origin**: The `sender_id` in routed signals is always overwritten by the server using the authenticated UUID, ensuring trust between peers.

# Networking Envelopes (MVP)

## Message Types
- Input: buttons/axes per client
- Intent: AI `PlanIntent`
- Snapshot: world state (delta-compressed)
- ReplayFrame: deterministic verification frame

## Snapshot Delta
- Serialize component tables with change masks
- Interest filtering by region and proximity

## Transport
- WebSockets (tokio-tungstenite over TCP); QUIC/UDP not implemented
- Server authoritative; client prediction for local movement (post-MVP)

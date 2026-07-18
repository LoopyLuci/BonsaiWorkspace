//! API Bridge - unified protocol gateway
//!
//! A single axum HTTP(+WebSocket) server ([`gateway::run`]) that fronts
//! several backend services behind one capability-authenticated API:
//! REST route translation ([`protocol::rest`]), MCP JSON-RPC passthrough
//! ([`protocol::mcp`]), a live event WebSocket ([`protocol::websocket`]),
//! bearer-capability and API-key auth ([`auth`]), consistent-load-based
//! routing ([`routing`]), per-backend circuit breaking and per-subject
//! rate limiting (in [`gateway`]), a [`telemetry`] event bus recorded into
//! the `universe` event log, and outbound [`webhooks`].
//!
//! Not included: the archived crate's gRPC gateway (`protocol::grpc`,
//! built from `proto/bridge.proto` via `tonic::include_proto!`) needed a
//! build.rs invoking `protoc`, which is not available in this environment;
//! and its TransferDaemon P2P backend adapter (`transfer_adapter`,
//! `transfer_client`) depends on the `transfer-client` crate, which is
//! still archived and out of scope for this restoration. Neither omission
//! affects the REST/WebSocket/JSON-RPC gateway.

pub mod auth;
pub mod gateway;
pub mod protocol;
pub mod routing;
pub mod telemetry;
pub mod webhooks;

pub use gateway::{run, BridgeState, CircuitState};

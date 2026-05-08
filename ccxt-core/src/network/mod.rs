//! Network module containing HTTP client, WebSocket client, and networking utilities

pub mod circuit_breaker;
pub mod endpoint_manager;
pub mod http_client;
pub mod rate_limiter;
pub mod retry_strategy;
pub mod signed_request;
pub mod ws_client;

// Re-export http_client and ws_client for backward compatibility
pub use http_client::*;
pub use ws_client::*;

// Re-export endpoint manager types
pub use endpoint_manager::{
    ExchangeEndpointManager, ExchangeEndpoints, RestEndpoints, WsChannel, WsEndpoints,
};

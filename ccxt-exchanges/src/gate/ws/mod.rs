//! Gate.io WebSocket module
//!
//! Contains WebSocket client, builder, and message parsers.
//!
//! # Architecture
//!
//! Uses the unified WebSocket architecture from ccxt-core:
//! - `GateSubscriptionBuilder`: Builds subscription messages
//! - `GateStreamParser`: Parses incoming WebSocket messages
//! - `GenericWsClient`: Generic WebSocket client from ccxt-core
//!
//! # Usage
//!
//! ```no_run
//! use ccxt_exchanges::gate::ws::{GateWsClient, create_gate_ws_client};
//!
//! // Create WebSocket client
//! let ws_client = create_gate_ws_client(false, "usdt"); // false = production
//! ```

mod builder;
mod parser;

pub use builder::GateSubscriptionBuilder;
pub use parser::GateStreamParser;

use crate::gate::auth::GateWsAuth;
use crate::gate::network::endpoint_router::GateWsEndpointProvider;
use ccxt_core::network::ws_client::config::{HeartbeatMode, WsConfig};
use ccxt_core::ws::{GenericWsClient, NoAuth};
use tokio_tungstenite::tungstenite::protocol::Message;

/// Gate.io WebSocket client type alias (public channels)
///
/// Used for public channel subscriptions
pub type GateWsClient =
    GenericWsClient<GateSubscriptionBuilder, GateStreamParser, GateWsEndpointProvider, NoAuth>;

/// Gate.io WebSocket client type alias (private channels)
///
/// Used for private channel subscriptions (orders, trades, balances)
/// Requires API credentials for authentication
pub type GateWsClientAuth =
    GenericWsClient<GateSubscriptionBuilder, GateStreamParser, GateWsEndpointProvider, GateWsAuth>;

/// Create Gate.io WebSocket client (public channels)
///
/// # Arguments
///
/// - `is_testnet`: Whether to use testnet environment
/// - `settle`: Settlement currency (e.g., "usdt", "usdc", "btc"). Defaults to "usdt".
///
/// # Example
///
/// ```no_run
/// use ccxt_exchanges::gate::ws::create_gate_ws_client;
///
/// let ws_client = create_gate_ws_client(false, "usdt"); // Production, USDT-margined
/// let ws_client = create_gate_ws_client(true, "usdc");  // Testnet, USDC-margined
/// ```
pub fn create_gate_ws_client(is_testnet: bool, settle: impl Into<String>) -> GateWsClient {
    // Gate.io 心跳配置：客户端主动发送 PING
    // Gate 要求每 30 秒发送一次心跳，超时 10 秒判定断开
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 25000, // 25 秒（略小于 30 秒要求）
        heartbeat_timeout: 10000,  // 10 秒超时
        ping_message: Message::Text(
            format!(
                "{{\"time\":{},\"channel\":\"spot.ping\"}}",
                chrono::Utc::now().timestamp()
            )
            .into(),
        ),
        ..Default::default()
    };

    GateWsClient::with_config(
        GateSubscriptionBuilder,
        GateStreamParser,
        GateWsEndpointProvider::new(is_testnet, settle),
        NoAuth,
        ws_config,
    )
}

/// Create Gate.io WebSocket client (private channels)
///
/// # Arguments
///
/// - `is_testnet`: Whether to use testnet environment
/// - `settle`: Settlement currency (e.g., "usdt", "usdc", "btc")
/// - `api_key`: API key for authentication
/// - `api_secret`: API secret for authentication
///
/// # Example
///
/// ```no_run
/// use ccxt_exchanges::gate::ws::create_gate_ws_client_auth;
///
/// let ws_client = create_gate_ws_client_auth(
///     false,
///     "usdt",
///     "your_api_key",
///     "your_api_secret"
/// );
/// ```
pub fn create_gate_ws_client_auth(
    is_testnet: bool,
    settle: impl Into<String>,
    api_key: impl Into<String>,
    api_secret: impl Into<String>,
) -> GateWsClientAuth {
    GateWsClientAuth::new(
        GateSubscriptionBuilder,
        GateStreamParser,
        GateWsEndpointProvider::new(is_testnet, settle),
        GateWsAuth::new(api_key, api_secret),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_client_creation() {
        let client = create_gate_ws_client(false, "usdt");
        // Verify client can be created
        drop(client);
    }

    #[test]
    fn test_ws_client_testnet() {
        let client = create_gate_ws_client(true, "usdt");
        // Verify testnet client can be created
        drop(client);
    }
}

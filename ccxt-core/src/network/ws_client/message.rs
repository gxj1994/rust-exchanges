//! WebSocket message types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// WebSocket message types for exchange communication.
///
/// Note: Subscribe/Unsubscribe variants have been removed.
/// Exchange-specific subscription messages are now built via injected SubscribeFn/UnsubscribeFn.
/// Ping/Pong variants have been removed as heartbeat is handled by HeartbeatManager.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WsMessage {
    /// Authentication message
    Auth {
        /// API key
        api_key: String,
        /// HMAC signature
        signature: String,
        /// Timestamp in milliseconds
        timestamp: i64,
    },
    /// Custom message payload
    Custom(Value),
}

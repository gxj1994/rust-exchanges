//! Gate.io WebSocket authentication
//!
//! Implements SubscribeAuthenticator for Gate.io WebSocket private channels.
//!
//! # Authentication Mechanism
//!
//! Gate.io uses message-level signature authentication for private channels:
//! - Private channels: `spot.orders`, `spot.usertrades`, `spot.balances`, etc.
//! - Signature method: HMAC-SHA512
//! - Signature format: `channel={channel}&event={event}&time={timestamp}`
//! - Auth fields: `method`, `KEY`, `SIGN`
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_exchanges::gate::auth::GateWsAuth;
//!
//! let auth = GateWsAuth::new(api_key, api_secret);
//! let subscribe_msg = serde_json::json!({
//!     "channel": "spot.orders",
//!     "event": "subscribe",
//!     "time": 1234567890,
//!     "payload": ["BTC_USDT"]
//! });
//! let authenticated_msg = auth.authenticate(subscribe_msg).await?;
//! // authenticated_msg now contains "auth" field with signature
//! ```

use ccxt_core::error::{Error, Result};
use ccxt_core::ws::auth::{AuthMode, SubscribeAuthenticator, WsAuthCore};
use hmac::KeyInit;
use serde_json::{Value, json};
use std::sync::Arc;

/// Gate.io WebSocket authentication handler
#[derive(Clone)]
pub struct GateWsAuth {
    /// API key
    api_key: String,
    /// API secret
    api_secret: Arc<String>,
}

impl GateWsAuth {
    /// Create a new GateWsAuth instance
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_secret: Arc::new(api_secret.into()),
        }
    }

    /// Generate HMAC-SHA512 signature
    fn generate_signature(&self, message: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha512;

        type HmacSha512 = Hmac<Sha512>;

        let mut mac = HmacSha512::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }
}

impl WsAuthCore for GateWsAuth {
    fn mode(&self) -> AuthMode {
        AuthMode::SubscribeAuth
    }
}

impl SubscribeAuthenticator for GateWsAuth {
    async fn authenticate(&self, subscribe: Value) -> Result<Value> {
        // Extract required fields
        let channel = subscribe["channel"]
            .as_str()
            .ok_or_else(|| Error::invalid_request("Missing channel in subscribe message"))?;
        let event = subscribe["event"]
            .as_str()
            .ok_or_else(|| Error::invalid_request("Missing event in subscribe message"))?;
        let time = subscribe["time"]
            .as_i64()
            .ok_or_else(|| Error::invalid_request("Missing time in subscribe message"))?;

        // Build signature message: channel={channel}&event={event}&time={time}
        let message = format!("channel={}&event={}&time={}", channel, event, time);

        // Generate signature
        let signature = self.generate_signature(&message);

        // Add auth field to subscribe message
        let mut authenticated = subscribe.clone();
        authenticated["auth"] = json!({
            "method": "api_key",
            "KEY": self.api_key,
            "SIGN": signature
        });

        Ok(authenticated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_ws_auth_creation() {
        let auth = GateWsAuth::new("test_key", "test_secret");
        assert_eq!(auth.mode(), AuthMode::SubscribeAuth);
    }

    #[test]
    fn test_generate_signature() {
        let auth = GateWsAuth::new("test_key", "test_secret");
        let message = "channel=spot.orders&event=subscribe&time=1234567890";
        let signature = auth.generate_signature(message);
        // HMAC-SHA512 produces a 128-character hex string
        assert_eq!(signature.len(), 128);
    }
}

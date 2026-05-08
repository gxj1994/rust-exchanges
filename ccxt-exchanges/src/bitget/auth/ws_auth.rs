//! Bitget WebSocket 认证策略
//!
//! 实现 MessageAuthenticator trait，用于 Bitget WebSocket 私有频道认证。
//!
//! # 认证流程
//!
//! 1. 连接 WebSocket 后发送登录消息
//! 2. 等待登录确认
//! 3. 可以订阅私有频道
//!
//! # 登录消息格式
//!
//! ```json
//! {
//!     "op": "login",
//!     "args": [{
//!         "apiKey": "...",
//!         "passphrase": "...",
//!         "timestamp": "...",
//!         "sign": "..."
//!     }]
//! }
//! ```
//!
//! # 签名方法
//!
//! sign = Base64(HMAC-SHA256(timestamp + "GET" + "/user/verify", secret_key))

use crate::bitget::auth::BitgetAuth;
use ccxt_core::ws::{AuthMode, MessageAuthenticator, WsAuthCore};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bitget WebSocket 认证策略
///
/// 实现 MessageAuthenticator trait，用于 Bitget WebSocket 私有频道认证。
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::bitget::{BitgetAuth, BitgetWsAuth};
/// use ccxt_core::ws::{GenericWsClient, MessageAuthenticator};
///
/// let auth = BitgetAuth::new(
///     "api-key".to_string(),
///     "secret".to_string(),
///     "passphrase".to_string(),
/// );
/// let ws_auth = BitgetWsAuth::new(auth);
///
/// // 获取登录消息
/// if let Some(login_msg) = ws_auth.login_message() {
///     client.send_json(&login_msg).await?;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct BitgetWsAuth {
    /// Bitget 认证器（复用 REST API 认证逻辑）
    auth: BitgetAuth,
}

impl BitgetWsAuth {
    /// 创建新的 Bitget WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `auth`: Bitget 认证器（包含 API key、secret、passphrase）
    pub fn new(auth: BitgetAuth) -> Self {
        Self { auth }
    }

    /// 从凭证创建 Bitget WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `api_key`: API Key
    /// - `secret`: Secret Key
    /// - `passphrase`: Passphrase
    pub fn from_credentials(api_key: String, secret: String, passphrase: String) -> Self {
        Self {
            auth: BitgetAuth::new(api_key, secret, passphrase),
        }
    }

    /// 生成当前时间戳（毫秒）
    fn generate_timestamp() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis().to_string())
            .unwrap_or_else(|_| "0".to_string())
    }

    /// 生成 WebSocket 登录签名
    ///
    /// Bitget WebSocket 签名方法：
    /// sign = Base64(HMAC-SHA256(timestamp + "GET" + "/user/verify", secret))
    fn generate_ws_signature(&self, timestamp: &str) -> String {
        // 使用 BitgetAuth 的 sign 方法
        // WebSocket 签名路径固定为 /user/verify，method 为 GET，body 为空
        self.auth.sign(timestamp, "GET", "/user/verify", "")
    }
}

impl WsAuthCore for BitgetWsAuth {
    fn mode(&self) -> AuthMode {
        AuthMode::MessageAuth
    }
}

impl MessageAuthenticator for BitgetWsAuth {
    /// 构建登录消息
    ///
    /// 返回 Bitget WebSocket 登录格式的 JSON 消息
    fn login_message(&self) -> Option<serde_json::Value> {
        let timestamp = Self::generate_timestamp();
        let sign = self.generate_ws_signature(&timestamp);

        Some(serde_json::json!({
            "op": "login",
            "args": [{
                "apiKey": self.auth.api_key(),
                "passphrase": self.auth.passphrase(),
                "timestamp": timestamp,
                "sign": sign
            }]
        }))
    }

    /// 检查是否登录成功
    ///
    /// Bitget 登录成功消息格式：
    /// ```json
    /// {
    ///     "event": "login",
    ///     "code": "0"
    /// }
    /// ```
    fn is_success(&self, msg: &serde_json::Value) -> bool {
        msg.get("event").and_then(|v| v.as_str()) == Some("login")
            && msg.get("code").and_then(|v| v.as_str()) == Some("0")
    }

    /// 检查是否登录失败
    ///
    /// Bitget 登录失败消息格式：
    /// ```json
    /// {
    ///     "event": "error",
    ///     "code": "...",
    ///     "msg": "..."
    /// }
    /// ```
    fn is_failure(&self, msg: &serde_json::Value) -> bool {
        msg.get("event").and_then(|v| v.as_str()) == Some("error")
            || msg.get("event").and_then(|v| v.as_str()) == Some("login")
                && msg.get("code").and_then(|v| v.as_str()) != Some("0")
    }

    /// 登录超时（毫秒）
    fn login_timeout_ms(&self) -> u64 {
        10000 // 10 秒
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitget_ws_auth_mode() {
        let auth = BitgetWsAuth::from_credentials(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );
        assert_eq!(auth.mode(), AuthMode::MessageAuth);
    }

    #[test]
    fn test_bitget_ws_auth_login_message() {
        let auth = BitgetWsAuth::from_credentials(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let msg = auth.login_message().expect("Should generate login message");
        assert_eq!(msg["op"], "login");
        assert!(msg["args"].is_array());

        let args = msg["args"].as_array().expect("args should be array");
        assert_eq!(args.len(), 1);
        assert_eq!(args[0]["apiKey"], "test-key");
        assert_eq!(args[0]["passphrase"], "test-passphrase");
        assert!(args[0]["timestamp"].is_string());
        assert!(args[0]["sign"].is_string());
    }

    #[test]
    fn test_bitget_ws_auth_is_success() {
        let auth = BitgetWsAuth::from_credentials(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let success_msg = serde_json::json!({
            "event": "login",
            "code": "0"
        });
        assert!(auth.is_success(&success_msg));

        let fail_msg = serde_json::json!({
            "event": "login",
            "code": "1"
        });
        assert!(!auth.is_success(&fail_msg));
    }

    #[test]
    fn test_bitget_ws_auth_is_failure() {
        let auth = BitgetWsAuth::from_credentials(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let error_msg = serde_json::json!({
            "event": "error",
            "code": "40001",
            "msg": "Invalid signature"
        });
        assert!(auth.is_failure(&error_msg));
    }
}

//! OKX WebSocket 认证策略
//!
//! 实现 MessageAuthenticator trait，用于 OKX WebSocket 私有频道认证。
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
//! sign = Base64(HMAC-SHA256(timestamp + "GET" + "/users/self/verify", secret_key))

use crate::okx::auth::OkxAuth;
use ccxt_core::ws::{AuthMode, MessageAuthenticator, WsAuthCore};
use chrono::Utc;
use serde_json::Value;

/// OKX WebSocket 认证策略
///
/// 实现 MessageAuthenticator trait，用于 OKX WebSocket 私有频道认证。
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::okx::auth::{OkxAuth, OkxWsAuth};
/// use ccxt_core::ws::{GenericWsClient, MessageAuthenticator};
///
/// let auth = OkxAuth::new(
///     "api-key".to_string(),
///     "secret".to_string(),
///     "passphrase".to_string(),
/// );
/// let ws_auth = OkxWsAuth::new(auth);
///
/// // 获取登录消息
/// if let Some(login_msg) = ws_auth.login_message() {
///     // 发送登录消息
///     client.send_json(&login_msg).await?;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct OkxWsAuth {
    /// OKX 认证器（复用 REST API 认证逻辑）
    auth: OkxAuth,
}

impl OkxWsAuth {
    /// 创建新的 OKX WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `auth`: OKX 认证器（包含 API key、secret、passphrase）
    pub fn new(auth: OkxAuth) -> Self {
        Self { auth }
    }

    /// 从凭证创建 OKX WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `api_key`: API Key
    /// - `secret`: Secret Key
    /// - `passphrase`: Passphrase
    pub fn from_credentials(api_key: String, secret: String, passphrase: String) -> Self {
        Self {
            auth: OkxAuth::new(api_key, secret, passphrase),
        }
    }

    /// 生成当前时间戳（ISO 8601 格式）
    fn generate_timestamp() -> String {
        Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }

    /// 生成 WebSocket 登录签名
    ///
    /// OKX WebSocket 签名方法：
    /// sign = Base64(HMAC-SHA256(timestamp + "GET" + "/users/self/verify", secret_key))
    fn generate_ws_signature(&self, timestamp: &str) -> String {
        // OKX WebSocket 签名路径固定为 /users/self/verify
        // 方法固定为 GET
        // body 为空
        self.auth.sign(timestamp, "GET", "/users/self/verify", "")
    }
}

impl WsAuthCore for OkxWsAuth {
    fn mode(&self) -> AuthMode {
        AuthMode::MessageAuth
    }
}

impl MessageAuthenticator for OkxWsAuth {
    /// 构建登录消息
    ///
    /// 返回 OKX WebSocket 登录格式的 JSON 消息
    fn login_message(&self) -> Option<Value> {
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
    /// OKX 登录成功消息格式：
    /// ```json
    /// {
    ///     "event": "login",
    ///     "code": "0",
    ///     "msg": ""
    /// }
    /// ```
    fn is_success(&self, msg: &Value) -> bool {
        msg.get("event").and_then(|v| v.as_str()) == Some("login")
            && msg.get("code").and_then(|v| v.as_str()) == Some("0")
    }

    /// 检查是否登录失败
    ///
    /// OKX 登录失败消息格式：
    /// ```json
    /// {
    ///     "event": "error",
    ///     "code": "...",
    ///     "msg": "..."
    /// }
    /// ```
    fn is_failure(&self, msg: &Value) -> bool {
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
    fn test_okx_ws_auth_mode() {
        let auth = OkxWsAuth::from_credentials(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );
        assert_eq!(auth.mode(), AuthMode::MessageAuth);
    }

    #[test]
    fn test_okx_ws_auth_login_message() {
        let auth = OkxWsAuth::from_credentials(
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
    fn test_okx_ws_auth_is_success() {
        let auth = OkxWsAuth::from_credentials(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let success_msg = serde_json::json!({
            "event": "login",
            "code": "0",
            "msg": ""
        });
        assert!(auth.is_success(&success_msg));

        let fail_msg = serde_json::json!({
            "event": "login",
            "code": "1",
            "msg": "Invalid signature"
        });
        assert!(!auth.is_success(&fail_msg));
    }

    #[test]
    fn test_okx_ws_auth_is_failure() {
        let auth = OkxWsAuth::from_credentials(
            "test-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );

        let error_msg = serde_json::json!({
            "event": "error",
            "code": "60001",
            "msg": "Invalid API key"
        });
        assert!(auth.is_failure(&error_msg));

        let login_fail_msg = serde_json::json!({
            "event": "login",
            "code": "1",
            "msg": "Invalid signature"
        });
        assert!(auth.is_failure(&login_fail_msg));
    }
}

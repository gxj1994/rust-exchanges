//! Bybit WebSocket 认证策略
//!
//! 实现 MessageAuthenticator trait，用于 Bybit WebSocket 私有频道认证。
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
//!     "op": "auth",
//!     "args": [api_key, timestamp, signature]
//! }
//! ```
//!
//! # 签名方法
//!
//! signature = HMAC-SHA256(timestamp + api_key + recv_window, secret_key)
//! 默认 recv_window = 20000

use crate::bybit::auth::BybitAuth;
use ccxt_core::ws::{AuthMode, MessageAuthenticator, WsAuthCore};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bybit WebSocket 认证策略
///
/// 实现 MessageAuthenticator trait，用于 Bybit WebSocket 私有频道认证。
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::bybit::{BybitAuth, BybitWsAuth};
/// use ccxt_core::ws::{GenericWsClient, MessageAuthenticator};
///
/// let auth = BybitAuth::new(
///     "api-key".to_string(),
///     "secret".to_string(),
/// );
/// let ws_auth = BybitWsAuth::new(auth);
///
/// // 获取登录消息
/// if let Some(login_msg) = ws_auth.login_message() {
///     client.send_json(&login_msg).await?;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct BybitWsAuth {
    /// Bybit 认证器（复用 REST API 认证逻辑）
    auth: BybitAuth,
    /// 接收窗口（毫秒）
    recv_window: u64,
}

impl BybitWsAuth {
    /// 默认接收窗口
    const DEFAULT_RECV_WINDOW: u64 = 20000;

    /// 创建新的 Bybit WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `auth`: Bybit 认证器（包含 API key 和 secret）
    pub fn new(auth: BybitAuth) -> Self {
        Self {
            auth,
            recv_window: Self::DEFAULT_RECV_WINDOW,
        }
    }

    /// 创建带自定义接收窗口的认证策略
    pub fn with_recv_window(auth: BybitAuth, recv_window: u64) -> Self {
        Self { auth, recv_window }
    }

    /// 从凭证创建 Bybit WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `api_key`: API Key
    /// - `secret`: Secret Key
    pub fn from_credentials(api_key: String, secret: String) -> Self {
        Self::new(BybitAuth::new(api_key, secret))
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
    /// Bybit WebSocket 签名方法：
    /// signature = HMAC-SHA256(timestamp + api_key + recv_window, secret)
    fn generate_ws_signature(&self, timestamp: &str) -> String {
        // 使用 BybitAuth 的 sign 方法
        // 注意：WebSocket 签名不需要 params，传空字符串
        self.auth.sign(timestamp, self.recv_window, "")
    }
}

impl WsAuthCore for BybitWsAuth {
    fn mode(&self) -> AuthMode {
        AuthMode::MessageAuth
    }
}

impl MessageAuthenticator for BybitWsAuth {
    /// 构建登录消息
    ///
    /// 返回 Bybit WebSocket 登录格式的 JSON 消息
    fn login_message(&self) -> Option<serde_json::Value> {
        let timestamp = Self::generate_timestamp();
        let signature = self.generate_ws_signature(&timestamp);

        Some(serde_json::json!({
            "op": "auth",
            "args": [
                self.auth.api_key(),
                timestamp,
                signature
            ]
        }))
    }

    /// 检查是否登录成功
    ///
    /// Bybit 登录成功消息格式：
    /// ```json
    /// {
    ///     "success": true,
    ///     "ret_msg": "",
    ///     "op": "auth"
    /// }
    /// ```
    fn is_success(&self, msg: &serde_json::Value) -> bool {
        msg.get("op").and_then(|v| v.as_str()) == Some("auth")
            && msg.get("success").and_then(|v| v.as_bool()) == Some(true)
    }

    /// 检查是否登录失败
    ///
    /// Bybit 登录失败消息格式：
    /// ```json
    /// {
    ///     "success": false,
    ///     "ret_msg": "Invalid signature",
    ///     "op": "auth"
    /// }
    /// ```
    fn is_failure(&self, msg: &serde_json::Value) -> bool {
        msg.get("op").and_then(|v| v.as_str()) == Some("auth")
            && msg.get("success").and_then(|v| v.as_bool()) == Some(false)
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
    fn test_bybit_ws_auth_mode() {
        let auth = BybitWsAuth::from_credentials("test-key".to_string(), "test-secret".to_string());
        assert_eq!(auth.mode(), AuthMode::MessageAuth);
    }

    #[test]
    fn test_bybit_ws_auth_login_message() {
        let auth = BybitWsAuth::from_credentials("test-key".to_string(), "test-secret".to_string());

        let msg = auth.login_message().expect("Should generate login message");
        assert_eq!(msg["op"], "auth");
        assert!(msg["args"].is_array());

        let args = msg["args"].as_array().expect("args should be array");
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "test-key");
        // args[1] 是时间戳
        // args[2] 是签名
    }

    #[test]
    fn test_bybit_ws_auth_is_success() {
        let auth = BybitWsAuth::from_credentials("test-key".to_string(), "test-secret".to_string());

        let success_msg = serde_json::json!({
            "success": true,
            "ret_msg": "",
            "op": "auth"
        });
        assert!(auth.is_success(&success_msg));

        let fail_msg = serde_json::json!({
            "success": false,
            "ret_msg": "Invalid signature",
            "op": "auth"
        });
        assert!(!auth.is_success(&fail_msg));
    }

    #[test]
    fn test_bybit_ws_auth_is_failure() {
        let auth = BybitWsAuth::from_credentials("test-key".to_string(), "test-secret".to_string());

        let error_msg = serde_json::json!({
            "success": false,
            "ret_msg": "Invalid API key",
            "op": "auth"
        });
        assert!(auth.is_failure(&error_msg));
    }
}

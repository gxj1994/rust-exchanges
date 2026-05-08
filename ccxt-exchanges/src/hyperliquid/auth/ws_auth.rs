//! Hyperliquid WebSocket 认证策略
//!
//! 实现 RequestSigner trait，用于 Hyperliquid WebSocket 私有请求签名。
//!
//! # 认证方式
//!
//! Hyperliquid 使用 EIP-712 签名（与 REST API 相同）。
//! 每个私有请求都需要包含签名，而不是单独的登录消息。
//!
//! # 签名格式
//!
//! 与 REST API 相同的 EIP-712 签名，使用私钥签名请求内容。

use crate::hyperliquid::auth::HyperLiquidAuth;
use ccxt_core::ws::{AuthMode, RequestSigner, WsAuthCore};
use std::time::{SystemTime, UNIX_EPOCH};

/// Hyperliquid WebSocket 认证策略
///
/// 实现 RequestSigner trait，用于 Hyperliquid WebSocket 私有请求签名。
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::hyperliquid::{HyperLiquidAuth, HyperliquidWsAuth};
/// use ccxt_core::ws::{GenericWsClient, RequestSigner};
///
/// let auth = HyperLiquidAuth::from_private_key(
///     "0x1234..."
/// )?;
/// let ws_auth = HyperliquidWsAuth::new(auth, true);
///
/// // 签名请求
/// let signed_request = ws_auth.sign_request(&request)?;
/// client.send_json(&signed_request).await?;
/// ```
#[derive(Debug, Clone)]
pub struct HyperliquidWsAuth {
    /// Hyperliquid 认证器（复用 REST API 认证逻辑）
    auth: HyperLiquidAuth,
    /// 是否为主网
    is_mainnet: bool,
}

impl HyperliquidWsAuth {
    /// 创建新的 Hyperliquid WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `auth`: Hyperliquid 认证器（包含私钥）
    /// - `is_mainnet`: 是否为主网
    pub fn new(auth: HyperLiquidAuth, is_mainnet: bool) -> Self {
        Self { auth, is_mainnet }
    }

    /// 从私钥创建 Hyperliquid WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `private_key_hex`: 私钥（十六进制字符串）
    /// - `is_mainnet`: 是否为主网
    ///
    /// # 错误
    ///
    /// 如果私钥格式无效，返回错误
    pub fn from_private_key(
        private_key_hex: &str,
        is_mainnet: bool,
    ) -> ccxt_core::error::Result<Self> {
        let auth = HyperLiquidAuth::from_private_key(private_key_hex)?;
        Ok(Self::new(auth, is_mainnet))
    }

    /// 获取钱包地址
    pub fn wallet_address(&self) -> &str {
        self.auth.wallet_address()
    }

    /// 生成当前 nonce（时间戳毫秒）
    fn generate_nonce() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// 对请求进行 EIP-712 签名
    ///
    /// # 参数
    ///
    /// - `action`: 请求动作（JSON 格式）
    ///
    /// # 返回
    ///
    /// 签名后的完整请求（包含 action 和签名）
    #[allow(unused)]
    fn sign_action(
        &self,
        action: &serde_json::Value,
    ) -> ccxt_core::error::Result<serde_json::Value> {
        let nonce = Self::generate_nonce();

        // 使用 HyperLiquidAuth 的签名方法
        let signature = self.auth.sign_l1_action(action, nonce, self.is_mainnet)?;

        // 构建完整请求
        Ok(serde_json::json!({
            "action": action,
            "nonce": nonce,
            "signature": signature.to_hex()
        }))
    }
}

impl WsAuthCore for HyperliquidWsAuth {
    fn mode(&self) -> AuthMode {
        AuthMode::RequestSign
    }
}

impl RequestSigner for HyperliquidWsAuth {
    /// 签名 WebSocket 请求
    ///
    /// 为 Hyperliquid WebSocket 请求添加签名
    fn sign(&self, request: &mut serde_json::Value) -> ccxt_core::error::Result<()> {
        let action = if let Some(action) = request.get("action") {
            action.clone()
        } else {
            // 如果没有 action 字段，将整个请求作为 action
            request.clone()
        };

        let nonce = Self::generate_nonce();

        // 使用 HyperLiquidAuth 的签名方法
        let signature = self.auth.sign_l1_action(&action, nonce, self.is_mainnet)?;

        // 修改请求，添加签名字段
        request["nonce"] = serde_json::json!(nonce);
        request["signature"] = serde_json::json!(signature.to_hex());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：实际的签名测试需要有效的私钥
    // 这里只测试基本功能

    #[test]
    fn test_hyperliquid_ws_auth_mode() {
        // 创建一个有效的私钥进行测试（32字节）
        let private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let auth = HyperliquidWsAuth::from_private_key(private_key, true);

        if let Ok(auth) = auth {
            assert_eq!(auth.mode(), AuthMode::RequestSign);
            assert!(auth.wallet_address().starts_with("0x"));
        }
    }

    #[test]
    fn test_hyperliquid_ws_auth_wallet_address() {
        let private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let auth = HyperliquidWsAuth::from_private_key(private_key, true);

        if let Ok(auth) = auth {
            assert!(auth.wallet_address().starts_with("0x"));
        }
    }
}

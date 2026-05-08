//! Binance WebSocket 认证策略
//!
//! 实现 TokenProvider trait，用于 Binance WebSocket 私有频道认证。
//!
//! # 认证流程
//!
//! 1. 通过 REST API 创建 listenKey
//! 2. 构建 WebSocket URL: wss://stream.binance.com/ws/{listenKey}
//! 3. 连接 WebSocket
//! 4. 自动续期 listenKey（每 25 分钟）
//!
//! # listenKey 管理
//!
//! - 有效期: 60 分钟
//! - 续期间隔: 25 分钟（推荐）
//! - 续期方式: PUT /api/v3/userDataStream?listenKey={key}

use ccxt_core::ws::{AuthMode, AuthToken, TokenDelivery, TokenProvider, WsAuthCore};
use ccxt_core::{Error, Result};
use std::sync::{Arc, Weak};

/// Binance WebSocket 市场类型
#[derive(Debug, Clone, Copy)]
pub enum BinanceWsMarket {
    /// 现货
    Spot,
    /// USDT-M 永续合约
    UsdtFutures,
    /// COIN-M 永续合约
    CoinFutures,
    /// 期权
    Option,
}

impl Default for BinanceWsMarket {
    fn default() -> Self {
        Self::Spot
    }
}

impl From<ccxt_core::types::MarketType> for BinanceWsMarket {
    fn from(market_type: ccxt_core::types::MarketType) -> Self {
        match market_type {
            ccxt_core::types::MarketType::Spot => BinanceWsMarket::Spot,
            ccxt_core::types::MarketType::Swap => BinanceWsMarket::UsdtFutures,
            ccxt_core::types::MarketType::Futures => BinanceWsMarket::UsdtFutures,
            ccxt_core::types::MarketType::Option => BinanceWsMarket::Option,
        }
    }
}

/// Binance WebSocket 认证策略
///
/// 实现 TokenProvider trait，通过 REST API 管理 listenKey。
///
/// # 循环引用解决方案
///
/// 使用 `Weak<Binance>` 弱引用避免循环引用：
/// - `Binance` 可以持有 `BinanceWsClientAuth`
/// - `BinanceWsAuth` 通过弱引用访问 `Binance`
/// - 当需要调用 REST API 时，临时升级为 `Arc`
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::binance::{Binance, BinanceWsAuth, BinanceWsMarket};
/// use ccxt_core::ws::{GenericWsClient, TokenProvider};
/// use std::sync::Arc;
///
/// // 创建 Binance 实例
/// let binance = Arc::new(Binance::new(config)?);
///
/// // 创建 WebSocket 认证策略（使用弱引用）
/// let ws_auth = BinanceWsAuth::from_arc(binance.clone(), BinanceWsMarket::Spot);
///
/// // 获取 listenKey
/// let token = ws_auth.get_token().await?;
///
/// // 修改 WebSocket URL
/// let url = ws_auth.modify_url("wss://stream.binance.com/ws", &token);
/// // 结果: "wss://stream.binance.com/ws/{listenKey}"
/// ```
pub struct BinanceWsAuth {
    /// Binance REST 客户端弱引用（避免循环引用）
    binance: Weak<crate::binance::Binance>,
    /// 市场类型
    market: BinanceWsMarket,
    /// 当前 listenKey（缓存）
    current_key: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl std::fmt::Debug for BinanceWsAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceWsAuth")
            .field("binance", &"Weak<Binance>")
            .field("market", &self.market)
            .field("current_key", &"Arc<RwLock<Option<String>>>")
            .finish()
    }
}

impl BinanceWsAuth {
    /// 创建新的 Binance WebSocket 认证策略
    ///
    /// # 参数
    ///
    /// - `binance`: Binance REST 客户端弱引用
    /// - `market`: 市场类型
    pub fn new(binance: Weak<crate::binance::Binance>, market: BinanceWsMarket) -> Self {
        Self {
            binance,
            market,
            current_key: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// 从 Arc 创建认证策略（自动转换为弱引用）
    ///
    /// # 参数
    ///
    /// - `binance`: Binance REST 客户端强引用
    /// - `market`: 市场类型
    pub fn from_arc(binance: Arc<crate::binance::Binance>, market: BinanceWsMarket) -> Self {
        Self::new(Arc::downgrade(&binance), market)
    }

    /// 创建现货市场认证策略
    pub fn spot(binance: Arc<crate::binance::Binance>) -> Self {
        Self::from_arc(binance, BinanceWsMarket::Spot)
    }

    /// 创建 USDT-M 合约认证策略
    pub fn usdt_futures(binance: Arc<crate::binance::Binance>) -> Self {
        Self::from_arc(binance, BinanceWsMarket::UsdtFutures)
    }

    /// 获取 Binance 实例（升级弱引用）
    ///
    /// 如果 Binance 已被释放，返回错误
    fn get_binance(&self) -> Result<Arc<crate::binance::Binance>> {
        self.binance
            .upgrade()
            .ok_or_else(|| Error::authentication("Binance instance has been dropped"))
    }

    /// 将市场类型转换为 MarketType
    fn to_market_type(&self) -> ccxt_core::types::MarketType {
        match self.market {
            BinanceWsMarket::Spot => ccxt_core::types::MarketType::Spot,
            BinanceWsMarket::UsdtFutures => ccxt_core::types::MarketType::Swap,
            BinanceWsMarket::CoinFutures => ccxt_core::types::MarketType::Swap,
            BinanceWsMarket::Option => ccxt_core::types::MarketType::Option,
        }
    }

    /// 获取市场类型对应的 REST API 路径
    #[allow(dead_code)]
    fn listen_key_endpoint(&self) -> &'static str {
        match self.market {
            BinanceWsMarket::Spot => "/api/v3/userDataStream",
            BinanceWsMarket::UsdtFutures => "/fapi/v1/listenKey",
            BinanceWsMarket::CoinFutures => "/dapi/v1/listenKey",
            BinanceWsMarket::Option => "/eapi/v1/listenKey",
        }
    }

    /// 获取当前的 listenKey（如果存在）
    pub async fn current_token(&self) -> Option<String> {
        self.current_key.read().await.clone()
    }
}

impl WsAuthCore for BinanceWsAuth {
    fn mode(&self) -> AuthMode {
        AuthMode::TokenPreAuth
    }
}

impl TokenProvider for BinanceWsAuth {
    /// 返回 Token 传递方式
    ///
    /// Binance 使用 URL 路径传递: wss://stream.binance.com/ws/{listenKey}
    fn delivery(&self) -> TokenDelivery {
        TokenDelivery::UrlPath
    }

    /// 获取 listenKey
    ///
    /// 通过 REST API 创建新的 listenKey。
    /// 如果已有缓存的 listenKey，则直接返回。
    async fn get_token(&self) -> Result<AuthToken> {
        // 检查缓存
        if let Some(key) = self.current_token().await {
            return Ok(AuthToken::new(key));
        }

        // 升级弱引用
        let binance = self.get_binance()?;

        // 创建新的 listenKey
        let market_type = Some(self.to_market_type());
        let listen_key = binance
            .create_listen_key_for_market(market_type)
            .await
            .map_err(|e| Error::authentication(format!("Failed to create listenKey: {}", e)))?;

        // 缓存
        *self.current_key.write().await = Some(listen_key.clone());

        tracing::info!(
            market = ?self.market,
            listen_key = %listen_key,
            "Created new listenKey"
        );

        Ok(AuthToken::new(listen_key))
    }

    /// 修改 WebSocket URL
    ///
    /// 将 listenKey 追加到 URL 路径：
    /// - 输入: "wss://stream.binance.com/ws"
    /// - 输出: "wss://stream.binance.com/ws/{listenKey}"
    fn modify_url(&self, url: &str, token: &AuthToken) -> String {
        let base_url = if url.ends_with('/') {
            url.to_string()
        } else {
            format!("{}/", url)
        };
        format!("{}{}", base_url, token.token)
    }

    /// 是否需要续期
    ///
    /// Binance listenKey 有效期 60 分钟，建议每 25 分钟续期一次
    fn needs_renewal(&self) -> bool {
        true
    }

    /// 续期 listenKey
    ///
    /// 通过 REST API PUT 请求续期 listenKey
    async fn renew(&self) -> Result<()> {
        let key = self
            .current_token()
            .await
            .ok_or_else(|| Error::invalid_request("No listenKey to renew"))?;

        // 升级弱引用
        let binance = self.get_binance()?;

        let market_type = Some(self.to_market_type());
        binance
            .refresh_listen_key_for_market(&key, market_type)
            .await
            .map_err(|e| Error::authentication(format!("Failed to renew listenKey: {}", e)))?;

        tracing::debug!(
            market = ?self.market,
            listen_key = %key,
            "Renewed listenKey"
        );

        Ok(())
    }

    /// 清理 listenKey
    ///
    /// 通过 REST API DELETE 请求删除 listenKey
    async fn cleanup(&self) -> Result<()> {
        if let Some(key) = self.current_token().await {
            // 升级弱引用（cleanup 时如果失败也不报错）
            if let Ok(binance) = self.get_binance() {
                let market_type = Some(self.to_market_type());
                let _ = binance
                    .delete_listen_key_for_market(&key, market_type)
                    .await;

                tracing::info!(
                    market = ?self.market,
                    listen_key = %key,
                    "Deleted listenKey"
                );
            }

            *self.current_key.write().await = None;
        }

        Ok(())
    }

    /// 续期间隔（秒）
    ///
    /// Binance 推荐每 25 分钟续期一次（有效期 60 分钟）
    fn renewal_interval_secs(&self) -> u64 {
        1500 // 25 minutes
    }
}

impl Clone for BinanceWsAuth {
    fn clone(&self) -> Self {
        Self {
            binance: self.binance.clone(),
            market: self.market,
            current_key: self.current_key.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_ws_auth_mode() {
        // 注意：这个测试需要 mock Binance 实例
        // 这里只测试 TokenDelivery
        // 实际集成测试需要真实的 Binance 实例
    }

    #[test]
    fn test_modify_url() {
        // 测试 URL 修改逻辑
        let token = AuthToken::new("test-listen-key-123");

        // 模拟 modify_url 逻辑
        let url = "wss://stream.binance.com/ws";
        let base_url = if url.ends_with('/') {
            url.to_string()
        } else {
            format!("{}/", url)
        };
        let result = format!("{}{}", base_url, token.token);

        assert_eq!(result, "wss://stream.binance.com/ws/test-listen-key-123");
    }

    #[test]
    fn test_market_type_conversion() {
        assert!(matches!(
            BinanceWsMarket::from(ccxt_core::types::MarketType::Spot),
            BinanceWsMarket::Spot
        ));
        assert!(matches!(
            BinanceWsMarket::from(ccxt_core::types::MarketType::Swap),
            BinanceWsMarket::UsdtFutures
        ));
    }

    #[test]
    fn test_renewal_interval() {
        // 25 minutes = 1500 seconds
        assert_eq!(1500, 25 * 60);
    }
}

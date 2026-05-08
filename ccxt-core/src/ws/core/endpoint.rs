//! WebSocket 端点提供者 Trait
//!
//! 用于处理多 URL 问题

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::ws::subscription::{MarketType, SubscriptionChannel};

/// WebSocket 上下文
///
/// 包含订阅相关的上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsContext {
    /// 市场类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_type: Option<MarketType>,
    /// 是否私有频道
    #[serde(default)]
    pub is_private: bool,
    /// 频道类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_type: Option<String>,
    /// 其他上下文信息
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

impl WsContext {
    /// 创建新的上下文
    pub fn new() -> Self {
        Self {
            market_type: None,
            is_private: false,
            channel_type: None,
            extra: HashMap::new(),
        }
    }

    /// 从订阅频道创建上下文
    pub fn from_channel(channel: &SubscriptionChannel) -> Self {
        Self {
            market_type: channel.market_type,
            is_private: channel.is_private(),
            channel_type: Some(channel.channel_type.to_string()),
            extra: HashMap::new(),
        }
    }

    /// 设置市场类型
    pub fn with_market_type(mut self, market_type: MarketType) -> Self {
        self.market_type = Some(market_type);
        self
    }

    /// 设置为私有频道
    pub fn with_private(mut self) -> Self {
        self.is_private = true;
        self
    }

    /// 设置频道类型
    pub fn with_channel_type(mut self, channel_type: impl Into<String>) -> Self {
        self.channel_type = Some(channel_type.into());
        self
    }

    /// 添加额外信息
    pub fn with_extra(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    /// 检查是否是 Kline 频道
    pub fn is_kline(&self) -> bool {
        self.channel_type
            .as_ref()
            .map(|t| t == "kline")
            .unwrap_or(false)
    }
}

impl Default for WsContext {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket 端点提供者 Trait
///
/// 用于解决不同交易所的多 URL 问题。
///
/// # 问题背景
///
/// 不同交易所的 WebSocket URL 策略差异巨大：
/// - **Bybit**: 按市场类型分 URL（spot/linear/inverse/option）
/// - **Bitget**: 公共/私有分离
/// - **OKX**: 单一 URL
/// - **Hyperliquid**: 单一 URL
///
/// # 实现示例
///
/// ```rust,ignore
/// use ccxt_core::ws::{WsEndpointProvider, WsContext};
///
/// pub struct BybitWsEndpointProvider {
///     is_sandbox: bool,
/// }
///
/// impl WsEndpointProvider for BybitWsEndpointProvider {
///     fn ws_public_url(&self, context: &WsContext) -> String {
///         let category = match context.market_type {
///             Some(MarketType::Spot) => "spot",
///             Some(MarketType::Swap) => "linear",
///             Some(MarketType::Future) => "inverse",
///             Some(MarketType::Option) => "option",
///             None => "linear",
///         };
///
///         if self.is_sandbox {
///             format!("wss://stream-testnet.bybit.com/v5/public/{}", category)
///         } else {
///             format!("wss://stream.bybit.com/v5/public/{}", category)
///         }
///     }
///
///     fn ws_private_url(&self, _context: &WsContext) -> String {
///         if self.is_sandbox {
///             "wss://stream-testnet.bybit.com/v5/private".to_string()
///         } else {
///             "wss://stream.bybit.com/v5/private".to_string()
///         }
///     }
/// }
/// ```
pub trait WsEndpointProvider: Clone + Send + Sync + 'static {
    /// 获取公共 WebSocket URL
    ///
    /// # 参数
    ///
    /// - `context`: WebSocket 上下文，包含市场类型等信息
    ///
    /// # 返回
    ///
    /// 返回对应的公共 WebSocket URL
    fn ws_public_url(&self, context: &WsContext) -> String;

    /// 获取私有 WebSocket URL
    ///
    /// # 参数
    ///
    /// - `context`: WebSocket 上下文
    ///
    /// # 返回
    ///
    /// 返回对应的私有 WebSocket URL
    fn ws_private_url(&self, context: &WsContext) -> String;

    /// 获取 WebSocket URL（根据上下文自动选择）
    ///
    /// 根据上下文中的 `is_private` 字段自动选择公共或私有 URL
    fn ws_url(&self, context: &WsContext) -> String {
        if context.is_private {
            self.ws_private_url(context)
        } else {
            self.ws_public_url(context)
        }
    }

    /// 是否支持多 URL
    ///
    /// 返回 true 表示该交易所根据市场类型使用不同的 URL
    fn supports_multiple_urls(&self) -> bool {
        false
    }

    /// 获取所有可能的 URL 列表
    ///
    /// 用于预连接或健康检查
    fn all_urls(&self) -> Vec<String> {
        vec![
            self.ws_public_url(&WsContext::new()),
            self.ws_private_url(&WsContext::new().with_private()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_context() {
        let ctx = WsContext::new()
            .with_market_type(MarketType::Swap)
            .with_private();

        assert_eq!(ctx.market_type, Some(MarketType::Swap));
        assert!(ctx.is_private);
    }

    #[test]
    fn test_ws_context_from_channel() {
        let channel = SubscriptionChannel::ticker("BTC/USDT").with_market_type(MarketType::Spot);
        let ctx = WsContext::from_channel(&channel);

        assert_eq!(ctx.market_type, Some(MarketType::Spot));
        assert!(!ctx.is_private);
    }

    #[test]
    fn test_ws_context_from_private_channel() {
        let channel = SubscriptionChannel::balance();
        let ctx = WsContext::from_channel(&channel);

        assert!(ctx.is_private);
    }

    #[derive(Clone)]
    struct TestEndpointProvider {
        is_sandbox: bool,
    }

    impl WsEndpointProvider for TestEndpointProvider {
        fn ws_public_url(&self, _context: &WsContext) -> String {
            if self.is_sandbox {
                "wss://test.example.com/ws".to_string()
            } else {
                "wss://api.example.com/ws".to_string()
            }
        }

        fn ws_private_url(&self, _context: &WsContext) -> String {
            if self.is_sandbox {
                "wss://test.example.com/private".to_string()
            } else {
                "wss://api.example.com/private".to_string()
            }
        }
    }

    #[test]
    fn test_endpoint_provider() {
        let provider = TestEndpointProvider { is_sandbox: false };

        let public_url = provider.ws_public_url(&WsContext::new());
        assert_eq!(public_url, "wss://api.example.com/ws");

        let private_url = provider.ws_private_url(&WsContext::new().with_private());
        assert_eq!(private_url, "wss://api.example.com/private");
    }

    #[test]
    fn test_endpoint_provider_auto_select() {
        let provider = TestEndpointProvider { is_sandbox: false };

        let ctx = WsContext::new().with_private();
        let url = provider.ws_url(&ctx);
        assert_eq!(url, "wss://api.example.com/private");
    }
}

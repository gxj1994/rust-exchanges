//! Bybit WebSocket endpoint provider.
//!
//! This module provides WebSocket endpoint routing for Bybit.
//! REST endpoint routing is now handled by ExchangeEndpointManager.

use ccxt_core::ws::{WsContext, WsEndpointProvider, subscription::MarketType};

// ============================================================================
// WebSocket Endpoint Provider
// ============================================================================

/// Bybit WebSocket 端点提供者
///
/// Bybit 按市场类型使用不同的 URL：
/// - SPOT: /v5/public/spot
/// - SWAP (linear): /v5/public/linear
/// - FUTURES (inverse): /v5/public/inverse
/// - OPTION: /v5/public/option
/// - 私有频道: /v5/private
/// **DEPRECATED**: Use `ExchangeEndpointManager` instead.
#[derive(Debug, Clone)]
pub struct BybitWsEndpointProvider {
    /// 是否使用沙箱环境
    is_sandbox: bool,
}

impl BybitWsEndpointProvider {
    /// 创建新的端点提供者
    pub fn new(is_sandbox: bool) -> Self {
        Self { is_sandbox }
    }
}

impl WsEndpointProvider for BybitWsEndpointProvider {
    /// 获取公共 WebSocket URL
    ///
    /// 复用静态配置，根据 context 中的 market_type 选择对应的 URL
    fn ws_public_url(&self, context: &WsContext) -> String {
        use crate::bybit::core::endpoints::{PRODUCTION_ENDPOINTS, TESTNET_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*TESTNET_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        let market_type = context.market_type.unwrap_or(MarketType::Spot);

        // 从 by_market 获取 URL，如果不存在则回退到 public
        endpoints
            .websocket
            .by_market
            .as_ref()
            .and_then(|m| m.get(&market_type))
            .unwrap_or(&endpoints.websocket.public)
            .to_string()
    }

    /// 获取私有 WebSocket URL
    ///
    /// 复用静态配置
    fn ws_private_url(&self, _context: &WsContext) -> String {
        use crate::bybit::core::endpoints::{PRODUCTION_ENDPOINTS, TESTNET_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*TESTNET_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        endpoints.websocket.private.to_string()
    }

    /// Bybit 支持按市场类型分 URL
    fn supports_multiple_urls(&self) -> bool {
        true
    }

    /// 获取所有 URL
    fn all_urls(&self) -> Vec<String> {
        use crate::bybit::core::endpoints::{PRODUCTION_ENDPOINTS, TESTNET_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*TESTNET_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        let mut urls = vec![
            endpoints.websocket.public.to_string(),
            endpoints.websocket.private.to_string(),
        ];

        // 添加 by_market 中的 URL
        if let Some(by_market) = &endpoints.websocket.by_market {
            for url in by_market.values() {
                urls.push(url.to_string());
            }
        }

        urls
    }
}

#[cfg(test)]
mod tests {
    use crate::bybit::{Bybit, BybitOptions};
    use ccxt_core::ExchangeConfig;
    use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};

    fn create_test_bybit() -> Bybit {
        Bybit::new(ExchangeConfig::default()).unwrap()
    }

    fn create_sandbox_bybit() -> Bybit {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        Bybit::new(config).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_production() {
        let bybit = create_test_bybit();
        let url = bybit.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        assert!(url.contains("api.bybit.com"));
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_rest_endpoint_sandbox() {
        let bybit = create_sandbox_bybit();
        let url = bybit.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        assert!(url.contains("api-testnet.bybit.com"));
    }

    // ==================== WebSocket Public Endpoint Tests ====================

    #[test]
    fn test_ws_public_endpoint_spot() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bybit = create_test_bybit();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.contains("stream.bybit.com"));
        assert!(url.ends_with("/v5/public/spot"));
    }

    #[test]
    fn test_ws_public_endpoint_linear() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bybit = create_test_bybit();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT:USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Swap,
            linear: Some(true),
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.contains("stream.bybit.com"));
        assert!(url.ends_with("/v5/public/linear"));
    }

    #[test]
    fn test_ws_public_endpoint_inverse() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bybit = create_test_bybit();
        let market = Market {
            id: "BTCUSD".to_string(),
            symbol: Symbol::new_unchecked("BTC/USD:BTC"),
            base: "BTC".to_string(),
            quote: "USD".to_string(),
            market_type: MarketType::Futures,
            linear: Some(false),
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.contains("stream.bybit.com"));
        assert!(url.ends_with("/v5/public/inverse"));
    }

    #[test]
    fn test_ws_public_endpoint_option() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bybit = create_test_bybit();
        let market = Market {
            id: "BTC-31DEC24-50000-C".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT:USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Option,
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.contains("stream.bybit.com"));
        assert!(url.ends_with("/v5/public/option"));
    }

    #[test]
    fn test_ws_public_endpoint_sandbox_spot() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bybit = create_sandbox_bybit();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.contains("stream-testnet.bybit.com"));
        assert!(url.ends_with("/v5/public/spot"));
    }

    #[test]
    fn test_ws_public_endpoint_sandbox_linear() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bybit = create_sandbox_bybit();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT:USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Swap,
            linear: Some(true),
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.contains("stream-testnet.bybit.com"));
        assert!(url.ends_with("/v5/public/linear"));
    }

    // ==================== WebSocket Private Endpoint Tests ====================

    #[test]
    fn test_ws_private_endpoint_production() {
        use ccxt_core::ws::WsContext;
        use ccxt_core::ws::WsEndpointProvider;

        let bybit = create_test_bybit();
        let mut context = WsContext::new();
        context.is_private = true;
        let url = bybit.ws_private_url(&context);
        assert!(url.contains("stream.bybit.com"));
        assert!(url.contains("/v5/private"));
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_ws_private_endpoint_sandbox() {
        use ccxt_core::ws::WsContext;
        use ccxt_core::ws::WsEndpointProvider;

        let bybit = create_sandbox_bybit();
        let mut context = WsContext::new();
        context.is_private = true;
        let url = bybit.ws_private_url(&context);
        assert!(url.contains("stream-testnet.bybit.com"));
        assert!(url.contains("/v5/private"));
    }

    // ==================== Category Path Construction Tests ====================

    #[test]
    fn test_ws_public_endpoint_path_format() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bybit = create_test_bybit();

        // Test spot
        let market_spot = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market_spot);
        assert!(url.ends_with("/v5/public/spot"));

        // Test linear
        let market_linear = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT:USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Swap,
            linear: Some(true),
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market_linear);
        assert!(url.ends_with("/v5/public/linear"));

        // Test inverse
        let market_inverse = Market {
            id: "BTCUSD".to_string(),
            symbol: Symbol::new_unchecked("BTC/USD:BTC"),
            base: "BTC".to_string(),
            quote: "USD".to_string(),
            market_type: MarketType::Futures,
            linear: Some(false),
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market_inverse);
        assert!(url.ends_with("/v5/public/inverse"));

        // Test option
        let market_option = Market {
            id: "BTC-31DEC24-50000-C".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT:USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Option,
            ..Default::default()
        };
        let url = bybit.ws_endpoint_for_market(&market_option);
        assert!(url.ends_with("/v5/public/option"));
    }

    // ==================== Testnet Option Tests ====================

    #[test]
    fn test_rest_endpoint_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            testnet: true,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        let url = bybit.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        assert!(url.contains("api-testnet.bybit.com"));
    }

    #[test]
    fn test_ws_private_endpoint_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            testnet: true,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        let url = bybit.default_ws_endpoint_by_options();
        assert!(url.contains("stream-testnet.bybit.com"));
    }

    // ==================== Integration with Default Type Tests ====================

    #[test]
    fn test_ws_public_endpoint_with_linear_default_type() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT:USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Swap,
            linear: Some(true),
            ..Default::default()
        };

        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.ends_with("/v5/public/linear"));
    }

    #[test]
    fn test_ws_public_endpoint_with_inverse_default_type() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();

        let market = Market {
            id: "BTCUSD".to_string(),
            symbol: Symbol::new_unchecked("BTC/USD:BTC"),
            base: "BTC".to_string(),
            quote: "USD".to_string(),
            market_type: MarketType::Futures,
            linear: Some(false),
            ..Default::default()
        };

        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.ends_with("/v5/public/inverse"));
    }
}

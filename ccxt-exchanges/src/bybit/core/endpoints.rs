//! Bybit 统一端点配置

use ccxt_core::network::endpoint_manager::{
    ExchangeEndpointManager, ExchangeEndpoints, RestEndpoints, WsEndpoints,
};
use ccxt_core::ws::subscription::MarketType as WsMarketType;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::bybit::Bybit;

// ============================================================================
// 静态端点配置
// ============================================================================

/// 生产环境端点配置
pub(crate) static PRODUCTION_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://api.bybit.com",
            linear_swap: None, // Bybit使用统一REST
            inverse_swap: None,
            option: None,
            special: HashMap::new(),
        },
        websocket: WsEndpoints {
            public: "wss://stream.bybit.com/v5/public/spot",
            private: "wss://stream.bybit.com/v5/private",
            by_market: Some({
                let mut map = HashMap::new();
                map.insert(WsMarketType::Spot, "wss://stream.bybit.com/v5/public/spot");
                map.insert(
                    WsMarketType::Swap,
                    "wss://stream.bybit.com/v5/public/linear",
                );
                map.insert(
                    WsMarketType::Future,
                    "wss://stream.bybit.com/v5/public/inverse",
                );
                map.insert(
                    WsMarketType::Option,
                    "wss://stream.bybit.com/v5/public/option",
                );
                map
            }),
            by_channel: None,
        },
    });

/// 测试网环境端点配置
pub(crate) static TESTNET_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://api-testnet.bybit.com",
            linear_swap: None,
            inverse_swap: None,
            option: None,
            special: HashMap::new(),
        },
        websocket: WsEndpoints {
            public: "wss://stream-testnet.bybit.com/v5/public/spot",
            private: "wss://stream-testnet.bybit.com/v5/private",
            by_market: Some({
                let mut map = HashMap::new();
                map.insert(
                    WsMarketType::Spot,
                    "wss://stream-testnet.bybit.com/v5/public/spot",
                );
                map.insert(
                    WsMarketType::Swap,
                    "wss://stream-testnet.bybit.com/v5/public/linear",
                );
                map.insert(
                    WsMarketType::Future,
                    "wss://stream-testnet.bybit.com/v5/public/inverse",
                );
                map.insert(
                    WsMarketType::Option,
                    "wss://stream-testnet.bybit.com/v5/public/option",
                );
                map
            }),
            by_channel: None,
        },
    });

// ============================================================================
// ExchangeEndpointManager 实现
// ============================================================================

impl Bybit {
    /// 获取端点配置
    pub fn endpoints_config(&self) -> &ExchangeEndpoints {
        if self.is_sandbox() {
            &TESTNET_ENDPOINTS
        } else {
            &PRODUCTION_ENDPOINTS
        }
    }

    /// 获取特定市场的REST端点
    ///
    /// Bybit使用统一的REST端点，所有市场类型都使用相同的URL
    pub fn rest_endpoint_for_market(&self, _market: &ccxt_core::types::Market) -> String {
        self.endpoints().rest.spot.to_string()
    }

    /// 获取特定市场的WebSocket端点
    ///
    /// Bybit的WebSocket端点按市场类型区分：spot/linear/inverse/option
    pub fn ws_endpoint_for_market(&self, market: &ccxt_core::types::Market) -> String {
        let endpoints = self.endpoints();

        // 根据市场类型选择对应的WebSocket URL
        let market_type = match market.market_type {
            ccxt_core::types::MarketType::Spot => WsMarketType::Spot,
            ccxt_core::types::MarketType::Futures | ccxt_core::types::MarketType::Swap => {
                // Bybit将futures和swap都映射到Swap（linear）
                if market.linear.unwrap_or(true) {
                    WsMarketType::Swap
                } else {
                    WsMarketType::Future
                }
            }
            ccxt_core::types::MarketType::Option => WsMarketType::Option,
        };

        // 从 by_market 获取 URL，如果不存在则回退到 public
        endpoints
            .websocket
            .by_market
            .as_ref()
            .and_then(|m| m.get(&market_type))
            .unwrap_or(&endpoints.websocket.public)
            .to_string()
    }

    /// 根据配置选项获取默认REST端点
    ///
    /// Bybit只有一个REST端点，所以忽略endpoint_type
    pub fn default_rest_endpoint_by_options(
        &self,
        _endpoint_type: ccxt_core::types::EndpointType,
    ) -> String {
        self.endpoints().rest.spot.to_string()
    }

    /// 根据配置选项获取默认WebSocket端点
    ///
    /// Bybit默认返回现货WebSocket端点
    pub fn default_ws_endpoint_by_options(&self) -> String {
        self.endpoints().websocket.public.to_string()
    }
}

impl ExchangeEndpointManager for Bybit {
    fn endpoints(&self) -> &ExchangeEndpoints {
        self.endpoints_config()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;
    use ccxt_core::ws::WsContext;
    use ccxt_core::ws::WsEndpointProvider;

    fn create_test_bybit() -> Bybit {
        Bybit::new(ExchangeConfig::default()).unwrap()
    }

    fn create_testnet_bybit() -> Bybit {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        Bybit::new(config).unwrap()
    }

    #[test]
    fn test_production_endpoints() {
        let bybit = create_test_bybit();
        let endpoints = bybit.endpoints();

        assert_eq!(endpoints.rest.spot, "https://api.bybit.com");
        assert_eq!(
            endpoints.websocket.public,
            "wss://stream.bybit.com/v5/public/spot"
        );
    }

    #[test]
    fn test_testnet_endpoints() {
        let bybit = create_testnet_bybit();
        let endpoints = bybit.endpoints();

        assert!(endpoints.rest.spot.contains("api-testnet"));
        assert!(endpoints.websocket.public.contains("stream-testnet"));
    }

    #[test]
    fn test_ws_endpoint_provider_auto_impl() {
        let bybit = create_test_bybit();
        let context = WsContext::new();

        let public_url = bybit.ws_public_url(&context);
        assert_eq!(public_url, "wss://stream.bybit.com/v5/public/spot");

        let private_url = bybit.ws_private_url(&context);
        assert_eq!(private_url, "wss://stream.bybit.com/v5/private");

        assert!(bybit.supports_multiple_urls());
    }

    #[test]
    fn test_rest_endpoint_for_market() {
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

        let url = bybit.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://api.bybit.com");
    }

    #[test]
    fn test_ws_endpoint_for_market_spot() {
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
        assert_eq!(url, "wss://stream.bybit.com/v5/public/spot");
    }

    #[test]
    fn test_ws_endpoint_for_market_swap() {
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
        assert_eq!(url, "wss://stream.bybit.com/v5/public/linear");
    }

    #[test]
    fn test_ws_endpoint_for_market_future() {
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
        assert_eq!(url, "wss://stream.bybit.com/v5/public/inverse");
    }

    #[test]
    fn test_default_rest_endpoint_by_options() {
        use ccxt_core::types::EndpointType;

        let bybit = create_test_bybit();
        let url = bybit.default_rest_endpoint_by_options(EndpointType::Public);
        assert_eq!(url, "https://api.bybit.com");

        let url = bybit.default_rest_endpoint_by_options(EndpointType::Private);
        assert_eq!(url, "https://api.bybit.com");
    }

    #[test]
    fn test_default_ws_endpoint_by_options() {
        let bybit = create_test_bybit();
        let url = bybit.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://stream.bybit.com/v5/public/spot");
    }

    #[test]
    fn test_testnet_ws_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bybit = create_testnet_bybit();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = bybit.ws_endpoint_for_market(&market);
        assert!(url.contains("stream-testnet"));
    }
}

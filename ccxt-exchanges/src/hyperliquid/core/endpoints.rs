//! HyperLiquid 统一端点配置

use ccxt_core::network::endpoint_manager::{
    ExchangeEndpointManager, ExchangeEndpoints, RestEndpoints, WsEndpoints,
};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::hyperliquid::HyperLiquid;

// ============================================================================
// 静态端点配置
// ============================================================================

/// 生产环境端点配置
pub(crate) static PRODUCTION_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://api.hyperliquid.xyz",
            linear_swap: None,
            inverse_swap: None,
            option: None,
            special: HashMap::new(),
        },
        websocket: WsEndpoints {
            public: "wss://api.hyperliquid.xyz/ws",
            private: "wss://api.hyperliquid.xyz/ws",
            by_market: None,
            by_channel: None,
        },
    });

/// 测试网环境端点配置
pub(crate) static TESTNET_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://api.hyperliquid-testnet.xyz",
            linear_swap: None,
            inverse_swap: None,
            option: None,
            special: HashMap::new(),
        },
        websocket: WsEndpoints {
            public: "wss://api.hyperliquid-testnet.xyz/ws",
            private: "wss://api.hyperliquid-testnet.xyz/ws",
            by_market: None,
            by_channel: None,
        },
    });

// ============================================================================
// ExchangeEndpointManager 实现
// ============================================================================

impl HyperLiquid {
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
    /// HyperLiquid只有一个统一的端点，所以总是返回spot端点
    pub fn rest_endpoint_for_market(&self, _market: &ccxt_core::types::Market) -> String {
        self.endpoints().rest.spot.to_string()
    }

    /// 获取特定市场的WebSocket端点
    ///
    /// HyperLiquid只有一个统一的WebSocket端点
    pub fn ws_endpoint_for_market(&self, _market: &ccxt_core::types::Market) -> String {
        self.endpoints().websocket.public.to_string()
    }

    /// 根据配置选项获取默认REST端点
    ///
    /// HyperLiquid只有一个端点，所以忽略endpoint_type
    pub fn default_rest_endpoint_by_options(
        &self,
        _endpoint_type: ccxt_core::types::EndpointType,
    ) -> String {
        self.endpoints().rest.spot.to_string()
    }

    /// 根据配置选项获取默认WebSocket端点
    ///
    /// HyperLiquid只有一个WebSocket端点
    pub fn default_ws_endpoint_by_options(&self) -> String {
        self.endpoints().websocket.public.to_string()
    }
}

impl ExchangeEndpointManager for HyperLiquid {
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
    use ccxt_core::ws::WsContext;
    use ccxt_core::ws::WsEndpointProvider;

    fn create_test_hyperliquid() -> HyperLiquid {
        HyperLiquid::builder().build().unwrap()
    }

    fn create_testnet_hyperliquid() -> HyperLiquid {
        HyperLiquid::builder().testnet(true).build().unwrap()
    }

    #[test]
    fn test_production_endpoints() {
        let hl = create_test_hyperliquid();
        let endpoints = hl.endpoints();

        assert_eq!(endpoints.rest.spot, "https://api.hyperliquid.xyz");
        assert_eq!(endpoints.websocket.public, "wss://api.hyperliquid.xyz/ws");
    }

    #[test]
    fn test_testnet_endpoints() {
        let hl = create_testnet_hyperliquid();
        let endpoints = hl.endpoints();

        assert!(endpoints.rest.spot.contains("testnet"));
        assert!(endpoints.websocket.public.contains("testnet"));
    }

    #[test]
    fn test_ws_endpoint_provider_auto_impl() {
        let hl = create_test_hyperliquid();
        let context = WsContext::new();

        let public_url = hl.ws_public_url(&context);
        assert_eq!(public_url, "wss://api.hyperliquid.xyz/ws");

        assert!(!hl.supports_multiple_urls());
    }

    #[test]
    fn test_rest_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let hl = create_test_hyperliquid();
        let market = Market {
            id: "BTC-USD".to_string(),
            symbol: Symbol::new_unchecked("BTC/USD"),
            base: "BTC".to_string(),
            quote: "USD".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = hl.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://api.hyperliquid.xyz");
    }

    #[test]
    fn test_ws_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let hl = create_test_hyperliquid();
        let market = Market {
            id: "BTC-USD".to_string(),
            symbol: Symbol::new_unchecked("BTC/USD"),
            base: "BTC".to_string(),
            quote: "USD".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = hl.ws_endpoint_for_market(&market);
        assert_eq!(url, "wss://api.hyperliquid.xyz/ws");
    }

    #[test]
    fn test_default_rest_endpoint_by_options() {
        use ccxt_core::types::EndpointType;

        let hl = create_test_hyperliquid();
        let url = hl.default_rest_endpoint_by_options(EndpointType::Public);
        assert_eq!(url, "https://api.hyperliquid.xyz");

        let url = hl.default_rest_endpoint_by_options(EndpointType::Private);
        assert_eq!(url, "https://api.hyperliquid.xyz");
    }

    #[test]
    fn test_default_ws_endpoint_by_options() {
        let hl = create_test_hyperliquid();
        let url = hl.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://api.hyperliquid.xyz/ws");
    }

    #[test]
    fn test_testnet_rest_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let hl = create_testnet_hyperliquid();
        let market = Market {
            id: "BTC-USD".to_string(),
            symbol: Symbol::new_unchecked("BTC/USD"),
            base: "BTC".to_string(),
            quote: "USD".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = hl.rest_endpoint_for_market(&market);
        assert!(url.contains("testnet"));
    }

    #[test]
    fn test_testnet_ws_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let hl = create_testnet_hyperliquid();
        let market = Market {
            id: "BTC-USD".to_string(),
            symbol: Symbol::new_unchecked("BTC/USD"),
            base: "BTC".to_string(),
            quote: "USD".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = hl.ws_endpoint_for_market(&market);
        assert!(url.contains("testnet"));
    }
}

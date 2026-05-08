//! Bitget 统一端点配置

use ccxt_core::network::endpoint_manager::{
    ExchangeEndpointManager, ExchangeEndpoints, RestEndpoints, WsEndpoints,
};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::bitget::Bitget;

// ============================================================================
// 静态端点配置
// ============================================================================

/// 生产环境端点配置
///
/// Bitget UTA V3 API端点：
/// - REST: https://api.bitget.com
/// - WS Public: wss://ws.bitget.com/v3/ws/public
/// - WS Private: wss://ws.bitget.com/v3/ws/private
pub(crate) static PRODUCTION_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://api.bitget.com",
            linear_swap: None,
            inverse_swap: None,
            option: None,
            special: HashMap::new(),
        },
        websocket: WsEndpoints {
            public: "wss://ws.bitget.com/v3/ws/public",   // V3 UTA
            private: "wss://ws.bitget.com/v3/ws/private", // V3 UTA
            by_market: None,
            by_channel: None,
        },
    });

/// 测试网环境端点配置
///
/// Bitget UTA V3 模拟盘特点：
/// - REST API: 使用相同的_production_ URL，但需要添加 `PAPTRADING: 1` 请求头
/// - WebSocket: 使用不同的域名 `wspap.bitget.com/v3/ws/`
///   - 公共频道: wss://wspap.bitget.com/v3/ws/public
///   - 私有频道: wss://wspap.bitget.com/v3/ws/private
pub(crate) static TESTNET_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://api.bitget.com", // Bitget测试网使用相同REST URL，通过header区分
            linear_swap: None,
            inverse_swap: None,
            option: None,
            special: HashMap::new(),
        },
        websocket: WsEndpoints {
            public: "wss://wspap.bitget.com/v3/ws/public", // V3 UTA 模拟盘
            private: "wss://wspap.bitget.com/v3/ws/private", // V3 UTA 模拟盘
            by_market: None,
            by_channel: None,
        },
    });

// ============================================================================
// ExchangeEndpointManager 实现
// ============================================================================

impl Bitget {
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
    /// Bitget使用统一的REST端点，所有市场类型都使用相同的URL
    pub fn rest_endpoint_for_market(&self, _market: &ccxt_core::types::Market) -> String {
        self.endpoints().rest.spot.to_string()
    }

    /// 获取特定市场的WebSocket端点
    ///
    /// Bitget只有一个统一的WebSocket端点
    pub fn ws_endpoint_for_market(&self, _market: &ccxt_core::types::Market) -> String {
        self.endpoints().websocket.public.to_string()
    }

    /// 根据配置选项获取默认REST端点
    ///
    /// Bitget只有一个端点，所以忽略endpoint_type
    pub fn default_rest_endpoint_by_options(
        &self,
        _endpoint_type: ccxt_core::types::EndpointType,
    ) -> String {
        self.endpoints().rest.spot.to_string()
    }

    /// 根据配置选项获取默认WebSocket端点
    ///
    /// Bitget只有一个WebSocket端点
    pub fn default_ws_endpoint_by_options(&self) -> String {
        self.endpoints().websocket.public.to_string()
    }
}

impl ExchangeEndpointManager for Bitget {
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

    fn create_test_bitget() -> Bitget {
        Bitget::new(ExchangeConfig::default()).unwrap()
    }

    #[test]
    fn test_production_endpoints() {
        let bitget = create_test_bitget();
        let endpoints = bitget.endpoints();

        assert_eq!(endpoints.rest.spot, "https://api.bitget.com");
        assert_eq!(
            endpoints.websocket.public,
            "wss://ws.bitget.com/v3/ws/public" // V3 UTA
        );
    }

    #[test]
    fn test_ws_endpoint_provider_auto_impl() {
        let bitget = create_test_bitget();
        let context = WsContext::new();

        let public_url = bitget.ws_public_url(&context);
        assert_eq!(public_url, "wss://ws.bitget.com/v3/ws/public"); // V3 UTA

        assert!(!bitget.supports_multiple_urls());
    }

    #[test]
    fn test_rest_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bitget = create_test_bitget();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = bitget.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://api.bitget.com");
    }

    #[test]
    fn test_ws_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let bitget = create_test_bitget();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = bitget.ws_endpoint_for_market(&market);
        assert_eq!(url, "wss://ws.bitget.com/v3/ws/public"); // V3 UTA
    }

    #[test]
    fn test_default_rest_endpoint_by_options() {
        use ccxt_core::types::EndpointType;

        let bitget = create_test_bitget();
        let url = bitget.default_rest_endpoint_by_options(EndpointType::Public);
        assert_eq!(url, "https://api.bitget.com");

        let url = bitget.default_rest_endpoint_by_options(EndpointType::Private);
        assert_eq!(url, "https://api.bitget.com");
    }

    #[test]
    fn test_default_ws_endpoint_by_options() {
        let bitget = create_test_bitget();
        let url = bitget.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://ws.bitget.com/v3/ws/public"); // V3 UTA
    }

    #[test]
    fn test_testnet_rest_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let bitget = Bitget::new(config).unwrap();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = bitget.rest_endpoint_for_market(&market);
        // Bitget测试网REST使用相同URL
        assert_eq!(url, "https://api.bitget.com");
    }

    #[test]
    fn test_testnet_ws_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let bitget = Bitget::new(config).unwrap();
        let market = Market {
            id: "BTCUSDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = bitget.ws_endpoint_for_market(&market);
        assert!(url.contains("wspap.bitget.com"));
        assert!(url.contains("/v3/ws/")); // V3 UTA
    }
}

//! OKX 统一端点配置

use ccxt_core::network::endpoint_manager::{
    ExchangeEndpointManager, ExchangeEndpoints, RestEndpoints, WsChannel, WsEndpoints,
};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::okx::Okx;

// ============================================================================
// 静态端点配置
// ============================================================================

/// 生产环境端点配置
pub(crate) static PRODUCTION_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://www.okx.com",
            linear_swap: None, // OKX使用统一REST
            inverse_swap: None,
            option: None,
            special: HashMap::new(),
        },
        websocket: WsEndpoints {
            public: "wss://ws.okx.com:8443/ws/v5/public",
            private: "wss://ws.okx.com:8443/ws/v5/private",
            by_market: None,
            by_channel: Some({
                let mut map = HashMap::new();
                map.insert(WsChannel::Public, "wss://ws.okx.com:8443/ws/v5/public");
                map.insert(WsChannel::Private, "wss://ws.okx.com:8443/ws/v5/private");
                map.insert(WsChannel::Business, "wss://ws.okx.com:8443/ws/v5/business");
                map
            }),
        },
    });

/// 模拟盘环境端点配置
pub(crate) static DEMO_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://www.okx.com", // OKX模拟盘REST使用相同URL
            linear_swap: None,
            inverse_swap: None,
            option: None,
            special: HashMap::new(),
        },
        websocket: WsEndpoints {
            public: "wss://wspap.okx.com:8443/ws/v5/public?brokerId=9999",
            private: "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999",
            by_channel: Some({
                let mut map = HashMap::new();
                map.insert(
                    WsChannel::Public,
                    "wss://wspap.okx.com:8443/ws/v5/public?brokerId=9999",
                );
                map.insert(
                    WsChannel::Private,
                    "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999",
                );
                map.insert(
                    WsChannel::Business,
                    "wss://wspap.okx.com:8443/ws/v5/business?brokerId=9999",
                );
                map
            }),
            by_market: None,
        },
    });

// ============================================================================
// ExchangeEndpointManager 实现
// ============================================================================

impl Okx {
    /// 获取端点配置
    pub fn endpoints_config(&self) -> &ExchangeEndpoints {
        if self.is_testnet_trading() {
            &DEMO_ENDPOINTS
        } else {
            &PRODUCTION_ENDPOINTS
        }
    }

    /// 获取特定市场的REST端点
    ///
    /// OKX使用统一的REST端点，所有市场类型都使用相同的URL
    pub fn rest_endpoint_for_market(&self, _market: &ccxt_core::types::Market) -> String {
        self.endpoints().rest.spot.to_string()
    }

    /// 获取特定市场的WebSocket端点
    ///
    /// OKX的WebSocket端点不区分市场类型，但区分公共/私有/业务频道
    /// 这里默认返回公共端点
    pub fn ws_endpoint_for_market(&self, _market: &ccxt_core::types::Market) -> String {
        self.endpoints().websocket.public.to_string()
    }

    /// 根据配置选项获取默认REST端点
    ///
    /// OKX只有一个REST端点，所以忽略endpoint_type
    pub fn default_rest_endpoint_by_options(
        &self,
        _endpoint_type: ccxt_core::types::EndpointType,
    ) -> String {
        self.endpoints().rest.spot.to_string()
    }

    /// 根据配置选项获取默认WebSocket端点
    ///
    /// OKX默认返回公共WebSocket端点
    pub fn default_ws_endpoint_by_options(&self) -> String {
        self.endpoints().websocket.public.to_string()
    }

    /// 获取特殊端点（如business频道）
    ///
    /// OKX支持多种WebSocket频道：public/private/business
    pub fn special_endpoint(&self, name: &str) -> String {
        // 先尝试从by_channel获取
        if let Some(by_channel) = &self.endpoints().websocket.by_channel {
            let channel = match name {
                "public" => WsChannel::Public,
                "private" => WsChannel::Private,
                "business" => WsChannel::Business,
                _ => return self.endpoints().websocket.public.to_string(),
            };

            if let Some(url) = by_channel.get(&channel) {
                return url.to_string();
            }
        }

        // 回退到默认
        self.endpoints().websocket.public.to_string()
    }
}

impl ExchangeEndpointManager for Okx {
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

    fn create_test_okx() -> Okx {
        Okx::new(ExchangeConfig::default()).unwrap()
    }

    fn create_demo_okx() -> Okx {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        Okx::new(config).unwrap()
    }

    #[test]
    fn test_production_endpoints() {
        let okx = create_test_okx();
        let endpoints = okx.endpoints();

        assert_eq!(endpoints.rest.spot, "https://www.okx.com");
        assert_eq!(
            endpoints.websocket.public,
            "wss://ws.okx.com:8443/ws/v5/public"
        );
        assert_eq!(
            endpoints.websocket.private,
            "wss://ws.okx.com:8443/ws/v5/private"
        );
    }

    #[test]
    fn test_demo_endpoints() {
        let okx = create_demo_okx();
        let endpoints = okx.endpoints();

        assert!(endpoints.websocket.public.contains("wspap.okx.com"));
        assert!(endpoints.websocket.public.contains("brokerId=9999"));
    }

    #[test]
    fn test_ws_endpoint_provider_auto_impl() {
        let okx = create_test_okx();
        let context = WsContext::new();

        let public_url = okx.ws_public_url(&context);
        assert_eq!(public_url, "wss://ws.okx.com:8443/ws/v5/public");

        let private_url = okx.ws_private_url(&context);
        assert_eq!(private_url, "wss://ws.okx.com:8443/ws/v5/private");

        assert!(okx.supports_multiple_urls()); // OKX按channel区分，支持多URL
    }

    #[test]
    fn test_all_ws_endpoints() {
        let okx = create_test_okx();
        let endpoints = okx.all_ws_endpoints();

        assert_eq!(
            endpoints.get("public"),
            Some(&"wss://ws.okx.com:8443/ws/v5/public")
        );
        assert_eq!(
            endpoints.get("private"),
            Some(&"wss://ws.okx.com:8443/ws/v5/private")
        );
        assert_eq!(
            endpoints.get("ws_business"),
            Some(&"wss://ws.okx.com:8443/ws/v5/business")
        );
    }

    #[test]
    fn test_rest_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let okx = create_test_okx();
        let market = Market {
            id: "BTC-USDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = okx.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://www.okx.com");
    }

    #[test]
    fn test_ws_endpoint_for_market() {
        use ccxt_core::types::{Market, MarketType, Symbol};

        let okx = create_test_okx();
        let market = Market {
            id: "BTC-USDT".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT"),
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            market_type: MarketType::Spot,
            ..Default::default()
        };

        let url = okx.ws_endpoint_for_market(&market);
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/public");
    }

    #[test]
    fn test_default_rest_endpoint_by_options() {
        use ccxt_core::types::EndpointType;

        let okx = create_test_okx();
        let url = okx.default_rest_endpoint_by_options(EndpointType::Public);
        assert_eq!(url, "https://www.okx.com");

        let url = okx.default_rest_endpoint_by_options(EndpointType::Private);
        assert_eq!(url, "https://www.okx.com");
    }

    #[test]
    fn test_default_ws_endpoint_by_options() {
        let okx = create_test_okx();
        let url = okx.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/public");
    }

    #[test]
    fn test_special_endpoint_public() {
        let okx = create_test_okx();
        let url = okx.special_endpoint("public");
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/public");
    }

    #[test]
    fn test_special_endpoint_private() {
        let okx = create_test_okx();
        let url = okx.special_endpoint("private");
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/private");
    }

    #[test]
    fn test_special_endpoint_business() {
        let okx = create_test_okx();
        let url = okx.special_endpoint("business");
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/business");
    }

    #[test]
    fn test_demo_special_endpoint() {
        let okx = create_demo_okx();
        let url = okx.special_endpoint("private");
        assert!(url.contains("wspap.okx.com"));
        assert!(url.contains("brokerId=9999"));
    }
}

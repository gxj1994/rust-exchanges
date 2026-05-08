//! Binance 统一端点配置
//!
//! 使用新的 ExchangeEndpointManager 架构，替代旧的 BinanceUrls 和 BinanceEndpointRouter。

use ccxt_core::network::endpoint_manager::{
    ExchangeEndpointManager, ExchangeEndpoints, RestEndpoints, WsEndpoints,
};
use ccxt_core::types::EndpointType;
use ccxt_core::types::market::{Market, MarketType};
use ccxt_core::ws::subscription::MarketType as WsMarketType;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::binance::Binance;

// ============================================================================
// 静态端点配置
// ============================================================================

/// 生产环境端点配置
pub(crate) static PRODUCTION_ENDPOINTS: LazyLock<ExchangeEndpoints> =
    LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://api.binance.com/api/v3",
            linear_swap: Some("https://fapi.binance.com/fapi/v1"),
            inverse_swap: Some("https://dapi.binance.com/dapi/v1"),
            option: Some("https://eapi.binance.com/eapi/v1"),
            special: {
                let mut map = HashMap::new();
                map.insert("sapi", "https://api.binance.com/sapi/v1");
                map.insert("sapi_v2", "https://api.binance.com/sapi/v2");
                map.insert("papi", "https://papi.binance.com/papi/v1");
                map
            },
        },
        websocket: WsEndpoints {
            public: "wss://stream.binance.com:9443/ws",
            private: "wss://stream.binance.com:9443/ws",
            by_market: Some({
                let mut map = HashMap::new();
                map.insert(WsMarketType::Spot, "wss://stream.binance.com:9443/ws");
                // Binance 2026-03 WebSocket upgrade: Use /market endpoint for swap futures
                // Most market data (ticker, trades, kline, markPrice) belong to /market
                map.insert(WsMarketType::Swap, "wss://fstream.binance.com/market/ws");
                map.insert(WsMarketType::Future, "wss://dstream.binance.com/ws");
                map.insert(
                    WsMarketType::Option,
                    "wss://nbstream.binance.com/eoptions/ws",
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
            spot: "https://demo-api.binance.com/api/v3",
            linear_swap: Some("https://demo-fapi.binance.com/fapi/v1"),
            inverse_swap: Some("https://demo-dapi.binance.com/dapi/v1"),
            option: Some("https://demo-eapi.binance.com/eapi/v1"),
            special: {
                let mut map = HashMap::new();
                map.insert("sapi", "https://demo-api.binance.com/sapi/v1");
                map.insert("sapi_v2", "https://demo-api.binance.com/sapi/v2");
                map.insert("papi", "https://demo-papi.binance.com/papi/v1");
                map
            },
        },
        websocket: WsEndpoints {
            public: "wss://demo-stream.binance.com:9443/ws",
            private: "wss://demo-stream.binance.com:9443/ws",
            by_market: Some({
                let mut map = HashMap::new();
                map.insert(WsMarketType::Spot, "wss://demo-stream.binance.com:9443/ws");
                map.insert(WsMarketType::Swap, "wss://fstream.binancefuture.com");
                map.insert(WsMarketType::Future, "wss://demo-api.binance.com/ws");
                map.insert(WsMarketType::Option, "wss://demo-api.binance.com/ws-api/v3");
                map
            }),
            by_channel: None,
        },
    });

// ============================================================================
// ExchangeEndpointManager 实现
// ============================================================================

impl Binance {
    /// 获取端点配置
    ///
    /// 根据 sandbox 模式返回对应的端点配置。
    /// 这是单一数据源，REST和WebSocket都使用此配置。
    pub fn endpoints_config(&self) -> &ExchangeEndpoints {
        if self.is_sandbox() {
            &TESTNET_ENDPOINTS
        } else {
            &PRODUCTION_ENDPOINTS
        }
    }
}

impl ExchangeEndpointManager for Binance {
    fn endpoints(&self) -> &ExchangeEndpoints {
        self.endpoints_config()
    }
}

// ============================================================================
// 便捷方法
// ============================================================================

impl Binance {
    /// 获取特定市场的 REST 端点
    ///
    /// 根据市场类型自动选择对应的 API 域名：
    /// - Spot: api.binance.com
    /// - Linear Swap/Futures: fapi.binance.com
    /// - Inverse Swap/Futures: dapi.binance.com
    /// - Option: eapi.binance.com
    pub fn rest_endpoint_for_market(&self, market: &Market) -> String {
        let endpoints = self.endpoints();

        match market.market_type {
            MarketType::Spot => endpoints.rest.spot.to_string(),
            MarketType::Swap | MarketType::Futures => {
                let is_linear = market.linear.unwrap_or(true);
                if is_linear {
                    endpoints
                        .rest
                        .linear_swap
                        .unwrap_or(endpoints.rest.spot)
                        .to_string()
                } else {
                    endpoints
                        .rest
                        .inverse_swap
                        .unwrap_or(endpoints.rest.spot)
                        .to_string()
                }
            }
            MarketType::Option => endpoints
                .rest
                .option
                .unwrap_or(endpoints.rest.spot)
                .to_string(),
        }
    }

    /// 获取特定市场的 WebSocket 端点
    ///
    /// 根据市场类型自动选择对应的 WebSocket 域名：
    /// - Spot: stream.binance.com
    /// - Linear Swap: fstream.binance.com
    /// - Inverse Swap/Future: dstream.binance.com
    /// - Option: nbstream.binance.com
    pub fn ws_endpoint_for_market(&self, market: &Market) -> String {
        let endpoints = self.endpoints();

        match market.market_type {
            MarketType::Spot => endpoints.websocket.public.to_string(),
            MarketType::Swap | MarketType::Futures => {
                let is_linear = market.linear.unwrap_or(true);
                let ws_type = if is_linear {
                    WsMarketType::Swap
                } else {
                    WsMarketType::Future
                };

                endpoints
                    .websocket
                    .by_market
                    .as_ref()
                    .and_then(|m| m.get(&ws_type))
                    .unwrap_or(&endpoints.websocket.public)
                    .to_string()
            }
            MarketType::Option => endpoints
                .websocket
                .by_market
                .as_ref()
                .and_then(|m| m.get(&WsMarketType::Option))
                .unwrap_or(&endpoints.websocket.public)
                .to_string(),
        }
    }

    /// 获取 SAPI (Spot API) 端点
    ///
    /// 用于 Binance 特有的现货功能：
    /// - 杠杆交易
    /// - 储蓄和质押
    /// - 子账户管理
    /// - 资产划转
    pub fn sapi_endpoint(&self) -> String {
        self.endpoints()
            .rest
            .special
            .get("sapi")
            .unwrap_or(&self.endpoints().rest.spot)
            .to_string()
    }

    /// 获取 SAPI V2 端点
    pub fn sapi_v2_endpoint(&self) -> String {
        self.endpoints()
            .rest
            .special
            .get("sapi_v2")
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.sapi_endpoint())
    }

    /// 获取 PAPI (Portfolio Margin API) 端点
    ///
    /// 用于组合保证金交易，支持现货、期货和期权跨市场保证金。
    pub fn papi_endpoint(&self) -> String {
        self.endpoints()
            .rest
            .special
            .get("papi")
            .unwrap_or(&self.endpoints().rest.spot)
            .to_string()
    }

    /// 获取默认 WebSocket 端点
    ///
    /// 根据 default_type 和 default_sub_type 选项选择对应的端点。
    pub fn default_ws_endpoint_by_options(&self) -> String {
        use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};

        let endpoints = self.endpoints();
        let options = self.options();

        match options.default_type {
            DefaultType::Swap | DefaultType::Futures => match options.default_sub_type {
                Some(DefaultSubType::Inverse) => endpoints
                    .websocket
                    .by_market
                    .as_ref()
                    .and_then(|m| m.get(&WsMarketType::Future))
                    .unwrap_or(&endpoints.websocket.public)
                    .to_string(),
                _ => endpoints
                    .websocket
                    .by_market
                    .as_ref()
                    .and_then(|m| m.get(&WsMarketType::Swap))
                    .unwrap_or(&endpoints.websocket.public)
                    .to_string(),
            },
            DefaultType::Option => endpoints
                .websocket
                .by_market
                .as_ref()
                .and_then(|m| m.get(&WsMarketType::Option))
                .unwrap_or(&endpoints.websocket.public)
                .to_string(),
            _ => endpoints.websocket.public.to_string(),
        }
    }

    /// 获取默认 REST 端点
    ///
    /// 根据 default_type 和 default_sub_type 选项选择对应的端点。
    pub fn default_rest_endpoint_by_options(&self, _endpoint_type: EndpointType) -> String {
        use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};

        let endpoints = self.endpoints();
        let options = self.options();

        match options.default_type {
            DefaultType::Spot => endpoints.rest.spot.to_string(),
            DefaultType::Margin => self.sapi_endpoint(),
            DefaultType::Swap | DefaultType::Futures => match options.default_sub_type {
                Some(DefaultSubType::Inverse) => endpoints
                    .rest
                    .inverse_swap
                    .unwrap_or(endpoints.rest.spot)
                    .to_string(),
                _ => endpoints
                    .rest
                    .linear_swap
                    .unwrap_or(endpoints.rest.spot)
                    .to_string(),
            },
            DefaultType::Option => endpoints
                .rest
                .option
                .unwrap_or(endpoints.rest.spot)
                .to_string(),
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;
    use ccxt_core::prelude::Symbol;
    use ccxt_core::types::market::Market;
    use rust_decimal_macros::dec;

    fn create_test_binance() -> Binance {
        Binance::new(ExchangeConfig::default()).unwrap()
    }

    #[test]
    fn test_production_endpoints() {
        let binance = create_test_binance();
        let endpoints = binance.endpoints();

        // REST端点
        assert_eq!(endpoints.rest.spot, "https://api.binance.com/api/v3");
        assert_eq!(
            endpoints.rest.linear_swap.unwrap(),
            "https://fapi.binance.com/fapi/v1"
        );
        assert_eq!(
            endpoints.rest.inverse_swap.unwrap(),
            "https://dapi.binance.com/dapi/v1"
        );

        // WebSocket端点
        assert_eq!(
            endpoints.websocket.public,
            "wss://stream.binance.com:9443/ws"
        );
        assert_eq!(
            endpoints.websocket.private,
            "wss://stream.binance.com:9443/ws"
        );

        // 特殊端点
        assert_eq!(
            endpoints.rest.special.get("sapi"),
            Some(&"https://api.binance.com/sapi/v1")
        );
        assert_eq!(
            endpoints.rest.special.get("papi"),
            Some(&"https://papi.binance.com/papi/v1")
        );
    }

    #[test]
    fn test_rest_endpoint_for_spot_market() {
        let binance = create_test_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT"),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://api.binance.com/api/v3");
    }

    #[test]
    fn test_rest_endpoint_for_linear_swap() {
        let binance = create_test_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT:USDT"),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://fapi.binance.com/fapi/v1");
    }

    #[test]
    fn test_ws_endpoint_provider_auto_impl() {
        use ccxt_core::ws::WsContext;
        use ccxt_core::ws::WsEndpointProvider;

        let binance = create_test_binance();
        let context = WsContext::new();

        // 验证自动实现的 WsEndpointProvider
        let public_url = binance.ws_public_url(&context);
        assert_eq!(public_url, "wss://stream.binance.com:9443/ws");

        let private_url = binance.ws_private_url(&context);
        assert_eq!(private_url, "wss://stream.binance.com:9443/ws");

        // 验证 supports_multiple_urls
        assert!(binance.supports_multiple_urls());

        // 验证 all_urls
        let urls = binance.all_urls();
        assert!(!urls.is_empty());
        assert!(urls.contains(&"wss://stream.binance.com:9443/ws".to_string()));
        // Updated for 2026-03 WebSocket architecture upgrade
        assert!(urls.contains(&"wss://fstream.binance.com/market/ws".to_string()));
    }

    #[test]
    fn test_all_rest_endpoints() {
        let binance = create_test_binance();
        let endpoints = binance.all_rest_endpoints();

        assert_eq!(
            endpoints.get("spot"),
            Some(&"https://api.binance.com/api/v3")
        );
        assert_eq!(
            endpoints.get("linear_swap"),
            Some(&"https://fapi.binance.com/fapi/v1")
        );
        assert_eq!(
            endpoints.get("sapi"),
            Some(&"https://api.binance.com/sapi/v1")
        );
        assert_eq!(
            endpoints.get("papi"),
            Some(&"https://papi.binance.com/papi/v1")
        );
    }

    #[test]
    fn test_all_ws_endpoints() {
        let binance = create_test_binance();
        let endpoints = binance.all_ws_endpoints();

        assert_eq!(
            endpoints.get("public"),
            Some(&"wss://stream.binance.com:9443/ws")
        );
        assert_eq!(
            endpoints.get("ws_spot"),
            Some(&"wss://stream.binance.com:9443/ws")
        );
        assert_eq!(
            endpoints.get("ws_swap"),
            // Updated for 2026-03 WebSocket architecture upgrade
            Some(&"wss://fstream.binance.com/market/ws")
        );
    }

    // ==================== 便捷方法测试 ====================

    #[test]
    fn test_rest_endpoint_for_market_spot() {
        let binance = create_test_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT"),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://api.binance.com/api/v3");
    }

    #[test]
    fn test_rest_endpoint_for_market_linear_swap() {
        let binance = create_test_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT:USDT"),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://fapi.binance.com/fapi/v1");
    }

    #[test]
    fn test_rest_endpoint_for_market_inverse_swap() {
        let binance = create_test_binance();
        let mut market = Market::new_swap(
            "BTCUSD_PERP".to_string(),
            Symbol::new_unchecked("BTC/USD:BTC"),
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            dec!(100.0),
        );
        market.linear = Some(false);
        market.inverse = Some(true);

        let url = binance.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://dapi.binance.com/dapi/v1");
    }

    #[test]
    fn test_sapi_endpoint() {
        let binance = create_test_binance();
        let url = binance.sapi_endpoint();
        assert_eq!(url, "https://api.binance.com/sapi/v1");
    }

    #[test]
    fn test_sapi_v2_endpoint() {
        let binance = create_test_binance();
        let url = binance.sapi_v2_endpoint();
        assert_eq!(url, "https://api.binance.com/sapi/v2");
    }

    #[test]
    fn test_papi_endpoint() {
        let binance = create_test_binance();
        let url = binance.papi_endpoint();
        assert_eq!(url, "https://papi.binance.com/papi/v1");
    }

    #[test]
    fn test_default_ws_endpoint_spot() {
        let binance = create_test_binance();
        let url = binance.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://stream.binance.com:9443/ws");
    }
}

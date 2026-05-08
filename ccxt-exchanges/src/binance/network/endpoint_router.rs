//! Binance WebSocket endpoint provider.
//!
//! This module provides WebSocket endpoint routing for Binance.
//! REST endpoint routing is now handled by ExchangeEndpointManager and Binance methods.

use ccxt_core::ws::{WsContext, WsEndpointProvider, subscription::MarketType as WsMarketType};

/// Binance WebSocket endpoint provider.
///
/// **DEPRECATED**: Use `ExchangeEndpointManager` instead. The Binance struct now
/// implements this trait directly with automatic WsEndpointProvider bridging.
#[derive(Debug, Clone)]
pub struct BinanceWsEndpointProvider {
    /// 是否使用沙箱环境
    pub is_sandbox: bool,
}

impl BinanceWsEndpointProvider {
    /// Create a new Binance WebSocket endpoint provider.
    pub fn new(is_sandbox: bool) -> Self {
        Self { is_sandbox }
    }
}

impl Default for BinanceWsEndpointProvider {
    fn default() -> Self {
        Self { is_sandbox: false }
    }
}

impl WsEndpointProvider for BinanceWsEndpointProvider {
    fn ws_public_url(&self, context: &WsContext) -> String {
        // 复用静态配置
        use crate::binance::core::endpoints::{PRODUCTION_ENDPOINTS, TESTNET_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*TESTNET_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        let market_type = context.market_type.unwrap_or(WsMarketType::Spot);

        // Binance 2026 WebSocket 分流规则：
        // - /public: @depth, @bookTicker, @trade, @aggTrade (高频公共行情/盘口数据)
        // - /market: @ticker, @markPrice, @kline (常规公共市场数据)
        // - /private: 用户数据流 (需要listenKey)
        //
        // 根据频道类型选择正确的URL路径
        // 注意：channel_type 使用 snake_case（如 "bids_asks", "order_book"）
        let belongs_to_public = match context.channel_type.as_deref() {
            // bookTicker 属于 /public 类别
            Some("bids_asks") => true,
            // depth/orderbook 属于 /public 类别
            Some("orderbook") => true,
            // trade/aggTrade 属于 /public 类别
            Some("trades") => true,
            // 其他频道默认使用 by_market 配置
            _ => false,
        };

        // 如果是 Swap/Future 且属于 public 类别，使用 /public/ws
        if matches!(market_type, WsMarketType::Swap | WsMarketType::Future) && belongs_to_public {
            // 获取基础URL并替换为 /public/ws
            let base_url = endpoints
                .websocket
                .by_market
                .as_ref()
                .and_then(|m| m.get(&market_type))
                .unwrap_or(&endpoints.websocket.public);

            // 替换 /market/ws 或 /ws 为 /public/ws
            if base_url.contains("/market/ws") {
                return base_url.replace("/market/ws", "/public/ws");
            } else if base_url.ends_with("/ws") {
                let without_ws = base_url.strip_suffix("/ws").unwrap_or(base_url);
                return format!("{}/public/ws", without_ws);
            }
            // 如果无法替换，返回原始URL
            return base_url.to_string();
        }

        // 从 by_market 获取 URL，如果不存在则回退到 public
        endpoints
            .websocket
            .by_market
            .as_ref()
            .and_then(|m| m.get(&market_type))
            .unwrap_or(&endpoints.websocket.public)
            .to_string()
    }

    fn ws_private_url(&self, context: &WsContext) -> String {
        // Binance WebSocket private URL is the same as public
        self.ws_public_url(context)
    }
}

#[cfg(test)]
mod tests {
    use crate::binance::Binance;
    use ccxt_core::{EndpointType, ExchangeConfig, Market, MarketType, types::Symbol};
    use rust_decimal_macros::dec;

    fn create_test_binance() -> Binance {
        Binance::new(ExchangeConfig::default()).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_spot_public() {
        let binance = create_test_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT".to_string()),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.rest_endpoint_for_market(&market);
        assert!(url.contains("api.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_spot_private() {
        let binance = create_test_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT".to_string()),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.rest_endpoint_for_market(&market);
        assert!(url.contains("api.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_linear_swap_public() {
        let binance = create_test_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT:USDT".to_string()),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.rest_endpoint_for_market(&market);
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_linear_swap_private() {
        let binance = create_test_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT:USDT".to_string()),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.rest_endpoint_for_market(&market);
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_inverse_swap_public() {
        let binance = create_test_binance();
        let mut market = Market::new_swap(
            "BTCUSD_PERP".to_string(),
            Symbol::new_unchecked("BTC/USD:BTC".to_string()),
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            dec!(100.0),
        );
        // Ensure inverse is set correctly
        market.linear = Some(false);
        market.inverse = Some(true);

        let url = binance.rest_endpoint_for_market(&market);
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_inverse_swap_private() {
        let binance = create_test_binance();
        let mut market = Market::new_swap(
            "BTCUSD_PERP".to_string(),
            Symbol::new_unchecked("BTC/USD:BTC".to_string()),
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            dec!(100.0),
        );
        market.linear = Some(false);
        market.inverse = Some(true);

        let url = binance.rest_endpoint_for_market(&market);
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_option_public() {
        let binance = create_test_binance();
        let market = Market {
            id: "BTC-250328-100000-C".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT:USDT-250328-100000-C".to_string()),
            market_type: MarketType::Option,
            ..Default::default()
        };

        let url = binance.rest_endpoint_for_market(&market);
        assert!(url.contains("eapi.binance.com"));
    }

    #[test]
    fn test_rest_endpoint_option_private() {
        let binance = create_test_binance();
        let market = Market {
            id: "BTC-250328-100000-C".to_string(),
            symbol: Symbol::new_unchecked("BTC/USDT:USDT-250328-100000-C".to_string()),
            market_type: MarketType::Option,
            ..Default::default()
        };

        let url = binance.rest_endpoint_for_market(&market);
        assert!(url.contains("eapi.binance.com"));
    }

    // ==================== WebSocket Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_spot() {
        let binance = create_test_binance();
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT".to_string()),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = binance.ws_endpoint_for_market(&market);
        assert!(url.contains("stream.binance.com"));
    }

    #[test]
    fn test_ws_endpoint_linear_swap() {
        let binance = create_test_binance();
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT:USDT".to_string()),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = binance.ws_endpoint_for_market(&market);
        assert!(url.contains("fstream.binance.com"));
    }

    #[test]
    fn test_ws_endpoint_inverse_swap() {
        let binance = create_test_binance();
        let mut market = Market::new_swap(
            "BTCUSD_PERP".to_string(),
            Symbol::new_unchecked("BTC/USD:BTC".to_string()),
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            dec!(100.0),
        );
        market.linear = Some(false);
        market.inverse = Some(true);

        let url = binance.ws_endpoint_for_market(&market);
        assert!(url.contains("dstream.binance.com"));
    }

    #[test]
    fn test_ws_endpoint_option() {
        let binance = create_test_binance();
        let mut market = Market::default();
        market.market_type = MarketType::Option;

        let url = binance.ws_endpoint_for_market(&market);
        assert!(url.contains("nbstream.binance.com"));
    }

    // ==================== Default Endpoint Tests ====================

    #[test]
    fn test_default_rest_endpoint_spot() {
        let binance = create_test_binance();

        let url = binance.default_rest_endpoint_by_options(EndpointType::Public);
        assert!(url.contains("api.binance.com"));
    }

    #[test]
    fn test_default_ws_endpoint_spot() {
        let binance = create_test_binance();

        let url = binance.default_ws_endpoint_by_options();
        assert!(url.contains("stream.binance.com"));
    }

    // ==================== SAPI and PAPI Tests ====================

    #[test]
    fn test_sapi_endpoint() {
        let binance = create_test_binance();

        let url = binance.sapi_endpoint();
        assert!(url.contains("sapi"));
        assert!(url.contains("api.binance.com"));
    }

    #[test]
    fn test_papi_endpoint() {
        let binance = create_test_binance();

        let url = binance.papi_endpoint();
        assert!(url.contains("papi"));
    }

    // ==================== Sandbox Mode Tests ====================

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_swap_defaults_to_linear_when_not_specified() {
        let binance = create_test_binance();
        let market = Market {
            market_type: MarketType::Swap,
            ..Default::default()
        };
        // linear and inverse are None

        let url = binance.rest_endpoint_for_market(&market);
        // Should default to linear (fapi)
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_futures_defaults_to_linear_when_not_specified() {
        let binance = create_test_binance();
        let market = Market {
            market_type: MarketType::Futures,
            ..Default::default()
        };
        // linear and inverse are None

        let url = binance.rest_endpoint_for_market(&market);
        // Should default to linear (fapi)
        assert!(url.contains("fapi.binance.com"));
    }
}

//! OKX-specific端点路由
//!
//! This module provides OKX endpoint routing functionality.
//!
//! # OKX API Structure
//!
//! OKX uses a unified V5 API with the following endpoint structure:
//! - **REST**: `www.okx.com` - Single unified REST endpoint for all market types
//! - **WebSocket Public**: `ws.okx.com:8443/ws/v5/public` - Public market data
//! - **WebSocket Private**: `ws.okx.com:8443/ws/v5/private` - Account data
//! - **WebSocket Business**: `ws.okx.com:8443/ws/v5/business` - Trade execution
//!
//! # Demo Trading Mode
//!
//! OKX uses a unique approach for demo trading:
//! - REST API uses the **same production domain** (`www.okx.com`)
//! - Demo mode is indicated by the `x-simulated-trading: 1` header
//! - WebSocket URLs switch to demo domain (`wspap.okx.com:8443`)
//!
//! # Channel Types
//!
//! OKX WebSocket has three channel types:
//! - `Public` - Market data (tickers, orderbooks, trades)
//! - `Private` - Account data (positions, orders, balances)
//! - `Business` - Trade execution and advanced features
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_exchanges::okx::Okx;
//! use ccxt_core::ExchangeConfig;
//! use ccxt_core::network::endpoint_manager::ExchangeEndpointManager;
//!
//! let okx = Okx::new(ExchangeConfig::default()).unwrap();
//!
//! // Get REST endpoint (unified for all market types)
//! let rest_url = okx.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
//! assert!(rest_url.contains("okx.com"));
//!
//! // Get WebSocket endpoint for public channel
//! let endpoints = okx.endpoints();
//! let ws_public = endpoints.websocket.public;
//! assert!(ws_public.contains("/ws/v5/public"));
//!
//! // Get WebSocket endpoint for private channel
//! let ws_private = endpoints.websocket.private;
//! assert!(ws_private.contains("/ws/v5/private"));
//!
//! // Check if demo trading mode is enabled
//! let is_demo = okx.is_testnet_trading();
//! ```

use ccxt_core::network::endpoint_manager::WsChannel;
use ccxt_core::ws::{WsContext, WsEndpointProvider};

/// OKX WebSocket channel type.
///
/// OKX uses different WebSocket channels for different types of data:
/// - `Public` - Market data streams (no authentication required)
/// - `Private` - Account data streams (authentication required)
/// - `Business` - Trade execution streams (authentication required)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OkxChannelType {
    /// Public channel for market data (tickers, orderbooks, trades).
    ///
    /// No authentication required. Used for:
    /// - Real-time ticker updates
    /// - Order book snapshots and updates
    /// - Public trade streams
    /// - Candlestick/OHLCV data
    Public,

    /// Private channel for account data.
    ///
    /// Authentication required. Used for:
    /// - Account balance updates
    /// - Position updates
    /// - Order status updates
    Private,

    /// Business channel for trade execution.
    ///
    /// Authentication required. Used for:
    /// - Advanced order types
    /// - Algo orders
    /// - Grid trading
    /// - Copy trading
    Business,
}

impl std::fmt::Display for OkxChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OkxChannelType::Public => write!(f, "public"),
            OkxChannelType::Private => write!(f, "private"),
            OkxChannelType::Business => write!(f, "business"),
        }
    }
}

// ============================================================================
// WebSocket Endpoint Provider
// ============================================================================

/// OKX WebSocket 端点提供者
///
/// OKX 使用公共/私有/业务分离的 URL 策略
///
/// # URL 策略
///
/// - 公共频道: /ws/v5/public
/// - 私有频道: /ws/v5/private
/// - 业务频道 (包括 Kline): /ws/v5/business
///
/// 注意：Kline/Candle 频道从 2023 年 6 月已移动到 /business URL
///
/// **DEPRECATED**: Use `ExchangeEndpointManager` instead.
#[derive(Debug, Clone)]
pub struct OkxWsEndpointProvider {
    /// 是否使用沙箱环境
    is_sandbox: bool,
}

impl OkxWsEndpointProvider {
    /// 创建新的端点提供者
    pub fn new(is_sandbox: bool) -> Self {
        Self { is_sandbox }
    }
}

impl WsEndpointProvider for OkxWsEndpointProvider {
    /// 获取公共 WebSocket URL
    ///
    /// 复用静态配置，单一数据源
    fn ws_public_url(&self, _context: &WsContext) -> String {
        use crate::okx::core::endpoints::{DEMO_ENDPOINTS, PRODUCTION_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*DEMO_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        endpoints.websocket.public.to_string()
    }

    /// 获取私有 WebSocket URL
    ///
    /// 复用静态配置
    fn ws_private_url(&self, _context: &WsContext) -> String {
        use crate::okx::core::endpoints::{DEMO_ENDPOINTS, PRODUCTION_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*DEMO_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        endpoints.websocket.private.to_string()
    }

    /// 获取 WebSocket URL（根据上下文自动选择）
    ///
    /// 根据频道类型选择正确的 URL:
    /// - Kline 频道 -> /ws/v5/business
    /// - 私有频道 -> /ws/v5/private
    /// - 其他公共频道 -> /ws/v5/public
    fn ws_url(&self, context: &WsContext) -> String {
        use crate::okx::core::endpoints::{DEMO_ENDPOINTS, PRODUCTION_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*DEMO_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        // Kline 频道需要使用业务 URL
        if context.is_kline() {
            if let Some(by_channel) = &endpoints.websocket.by_channel {
                if let Some(url) = by_channel.get(&WsChannel::Business) {
                    return url.to_string();
                }
            }
        }

        if context.is_private {
            endpoints.websocket.private.to_string()
        } else {
            endpoints.websocket.public.to_string()
        }
    }

    /// OKX 支持按channel区分URL
    fn supports_multiple_urls(&self) -> bool {
        true
    }

    /// 获取所有 URL
    fn all_urls(&self) -> Vec<String> {
        use crate::okx::core::endpoints::{DEMO_ENDPOINTS, PRODUCTION_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*DEMO_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        let mut urls = vec![
            endpoints.websocket.public.to_string(),
            endpoints.websocket.private.to_string(),
        ];

        // 添加 by_channel 中的 URL
        if let Some(by_channel) = &endpoints.websocket.by_channel {
            for url in by_channel.values() {
                urls.push(url.to_string());
            }
        }

        urls
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::okx::{Okx, OkxOptions};
    use ccxt_core::ExchangeConfig;

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

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_production() {
        let okx = create_test_okx();
        let url = okx.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        assert_eq!(url, "https://www.okx.com");
    }

    #[test]
    fn test_rest_endpoint_demo() {
        let okx = create_demo_okx();
        let url = okx.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        // OKX uses the same REST domain for demo trading
        // Demo mode is indicated by header, not URL
        assert_eq!(url, "https://www.okx.com");
    }

    // ==================== WebSocket Public Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_public_production() {
        let okx = create_test_okx();
        let url = okx.special_endpoint("public");
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/public");
        assert!(!url.contains("brokerId"));
    }

    #[test]
    fn test_ws_endpoint_public_demo() {
        let okx = create_demo_okx();
        let url = okx.special_endpoint("public");
        assert!(url.contains("wspap.okx.com"));
        assert!(url.contains("/ws/v5/public"));
        assert!(url.contains("brokerId=9999"));
    }

    // ==================== WebSocket Private Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_private_production() {
        let okx = create_test_okx();
        let url = okx.special_endpoint("private");
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/private");
        assert!(!url.contains("brokerId"));
    }

    #[test]
    fn test_ws_endpoint_private_demo() {
        let okx = create_demo_okx();
        let url = okx.special_endpoint("private");
        assert!(url.contains("wspap.okx.com"));
        assert!(url.contains("/ws/v5/private"));
        assert!(url.contains("brokerId=9999"));
    }

    // ==================== WebSocket Business Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_business_production() {
        let okx = create_test_okx();
        let url = okx.special_endpoint("business");
        assert_eq!(url, "wss://ws.okx.com:8443/ws/v5/business");
        assert!(!url.contains("brokerId"));
    }

    #[test]
    fn test_ws_endpoint_business_demo() {
        let okx = create_demo_okx();
        let url = okx.special_endpoint("business");
        assert!(url.contains("wspap.okx.com"));
        assert!(url.contains("/ws/v5/business"));
        assert!(url.contains("brokerId=9999"));
    }

    // ==================== Demo Trading Mode Tests ====================

    #[test]
    fn test_is_demo_trading_false_by_default() {
        let okx = create_test_okx();
        assert!(!okx.is_testnet_trading());
    }

    #[test]
    fn test_is_demo_trading_with_sandbox_config() {
        let okx = create_demo_okx();
        assert!(okx.is_testnet_trading());
    }

    #[test]
    fn test_is_demo_trading_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = OkxOptions {
            testnet: true,
            ..Default::default()
        };
        let okx = Okx::new_with_options(config, options).unwrap();
        assert!(okx.is_testnet_trading());
    }

    // ==================== Channel Type Display Tests ====================

    #[test]
    fn test_channel_type_display() {
        assert_eq!(format!("{}", OkxChannelType::Public), "public");
        assert_eq!(format!("{}", OkxChannelType::Private), "private");
        assert_eq!(format!("{}", OkxChannelType::Business), "business");
    }

    // ==================== Channel Type Equality Tests ====================

    #[test]
    fn test_channel_type_equality() {
        assert_eq!(OkxChannelType::Public, OkxChannelType::Public);
        assert_eq!(OkxChannelType::Private, OkxChannelType::Private);
        assert_eq!(OkxChannelType::Business, OkxChannelType::Business);
        assert_ne!(OkxChannelType::Public, OkxChannelType::Private);
        assert_ne!(OkxChannelType::Private, OkxChannelType::Business);
    }

    // ==================== All Channel Types Tests ====================

    #[test]
    fn test_all_channel_types_production() {
        let okx = create_test_okx();

        let channels = [
            (OkxChannelType::Public, "/ws/v5/public"),
            (OkxChannelType::Private, "/ws/v5/private"),
            (OkxChannelType::Business, "/ws/v5/business"),
        ];

        for (channel_type, expected_path) in channels {
            let url = okx.special_endpoint(&channel_type.to_string());
            assert!(
                url.contains(expected_path),
                "URL {} should contain {}",
                url,
                expected_path
            );
            assert!(
                url.contains("ws.okx.com"),
                "Production URL {} should contain ws.okx.com",
                url
            );
        }
    }

    #[test]
    fn test_all_channel_types_demo() {
        let okx = create_demo_okx();

        let channels = [
            (OkxChannelType::Public, "/ws/v5/public"),
            (OkxChannelType::Private, "/ws/v5/private"),
            (OkxChannelType::Business, "/ws/v5/business"),
        ];

        for (channel_type, expected_path) in channels {
            let url = okx.special_endpoint(&channel_type.to_string());
            assert!(
                url.contains(expected_path),
                "URL {} should contain {}",
                url,
                expected_path
            );
            assert!(
                url.contains("wspap.okx.com"),
                "Demo URL {} should contain wspap.okx.com",
                url
            );
            assert!(
                url.contains("brokerId=9999"),
                "Demo URL {} should contain brokerId=9999",
                url
            );
        }
    }

    // ==================== Consistency Tests ====================

    #[test]
    fn test_rest_endpoint_same_for_production_and_demo() {
        let okx_prod = create_test_okx();
        let okx_demo = create_demo_okx();

        // OKX uses the same REST domain for both modes
        assert_eq!(
            okx_prod.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public),
            okx_demo.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public)
        );
    }

    #[test]
    fn test_ws_endpoints_differ_for_production_and_demo() {
        let okx_prod = create_test_okx();
        let okx_demo = create_demo_okx();

        // WebSocket URLs should be different
        assert_ne!(
            okx_prod.special_endpoint("public"),
            okx_demo.special_endpoint("public")
        );
        assert_ne!(
            okx_prod.special_endpoint("private"),
            okx_demo.special_endpoint("private")
        );
        assert_ne!(
            okx_prod.special_endpoint("business"),
            okx_demo.special_endpoint("business")
        );
    }
}

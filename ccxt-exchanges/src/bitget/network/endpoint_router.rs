//! Bitget端点路由模块
//!
//! 提供Bitget的端点管理和WebSocket端点提供功能。
//!
//! # Bitget UTA V3 API结构
//!
//! Bitget使用简单的双端点结构：
//! - **REST**: `api.bitget.com` - 单一REST端点
//! - **WebSocket Public**: `ws.bitget.com/v3/ws/public` - 公共市场数据 (V3 UTA)
//! - **WebSocket Private**: `ws.bitget.com/v3/ws/private` - 账户数据 (V3 UTA)
//!
//! # 测试网/沙箱模式
//!
//! Bitget提供独立的测试网环境：
//! - REST: `api.bitget.com` (相同URL，通过header区分)
//! - WS Public: `wspap.bitget.com/v3/ws/public`
//! - WS Private: `wspap.bitget.com/v3/ws/private`
//!
//! # 示例
//!
//! ```rust,no_run
//! use ccxt_exchanges::bitget::Bitget;
//! use ccxt_core::ExchangeConfig;
//!
//! let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
//!
//! // 获取REST端点
//! let rest_url = bitget.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
//! assert!(rest_url.contains("api.bitget.com"));
//!
//! // 获取WebSocket公共端点
//! let ws_public = bitget.default_ws_endpoint_by_options();
//! assert!(ws_public.contains("/v3/ws/public"));
//! ```

use ccxt_core::ws::{WsContext, WsEndpointProvider};

// ============================================================================
// WebSocket Endpoint Provider
// ============================================================================

/// Bitget WebSocket 端点提供者
///
/// Bitget 使用公共/私有分离的 URL 策略
/// **DEPRECATED**: Use `ExchangeEndpointManager` instead.
#[derive(Debug, Clone)]
pub struct BitgetWsEndpointProvider {
    /// 是否使用沙箱环境
    is_sandbox: bool,
}

impl BitgetWsEndpointProvider {
    /// 创建新的端点提供者
    pub fn new(is_sandbox: bool) -> Self {
        Self { is_sandbox }
    }
}

impl WsEndpointProvider for BitgetWsEndpointProvider {
    /// 获取公共 WebSocket URL
    ///
    /// 复用静态配置，单一数据源
    fn ws_public_url(&self, _context: &WsContext) -> String {
        use crate::bitget::core::endpoints::{PRODUCTION_ENDPOINTS, TESTNET_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*TESTNET_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        endpoints.websocket.public.to_string()
    }

    /// 获取私有 WebSocket URL
    ///
    /// 复用静态配置，单一数据源
    fn ws_private_url(&self, _context: &WsContext) -> String {
        use crate::bitget::core::endpoints::{PRODUCTION_ENDPOINTS, TESTNET_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*TESTNET_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        endpoints.websocket.private.to_string()
    }

    /// Bitget 不按市场类型分 URL
    fn supports_multiple_urls(&self) -> bool {
        false
    }

    /// 获取所有 URL
    fn all_urls(&self) -> Vec<String> {
        vec![
            self.ws_public_url(&WsContext::new()),
            self.ws_private_url(&WsContext::new()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::bitget::{Bitget, BitgetOptions};
    use ccxt_core::{EndpointType, ExchangeConfig};

    fn create_test_bitget() -> Bitget {
        Bitget::new(ExchangeConfig::default()).unwrap()
    }

    fn create_sandbox_bitget() -> Bitget {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        Bitget::new(config).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_production() {
        let bitget = create_test_bitget();
        let url = bitget.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        assert_eq!(url, "https://api.bitget.com");
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_rest_endpoint_sandbox() {
        let bitget = create_sandbox_bitget();
        let url = bitget.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        // Bitget测试网REST使用相同URL，通过header区分
        assert_eq!(url, "https://api.bitget.com");
    }

    // ==================== WebSocket Public Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_public_production() {
        let bitget = create_test_bitget();
        let url = bitget.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://ws.bitget.com/v3/ws/public"); // V3 UTA
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_ws_endpoint_public_sandbox() {
        let bitget = create_sandbox_bitget();
        let url = bitget.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://wspap.bitget.com/v3/ws/public"); // V3 UTA
    }

    // ==================== WebSocket Private Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_private_production() {
        let bitget = create_test_bitget();
        let url = {
            use ccxt_core::ws::WsContext;
            use ccxt_core::ws::WsEndpointProvider;
            let mut ctx = WsContext::new();
            ctx.is_private = true;
            bitget.ws_private_url(&ctx)
        };
        assert_eq!(url, "wss://ws.bitget.com/v3/ws/private"); // V3 UTA
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_ws_endpoint_private_sandbox() {
        let bitget = create_sandbox_bitget();
        let url = {
            use ccxt_core::ws::WsContext;
            use ccxt_core::ws::WsEndpointProvider;
            let mut ctx = WsContext::new();
            ctx.is_private = true;
            bitget.ws_private_url(&ctx)
        };
        assert_eq!(url, "wss://wspap.bitget.com/v3/ws/private"); // V3 UTA
    }

    // ==================== Testnet Option Tests ====================

    #[test]
    fn test_rest_endpoint_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BitgetOptions {
            testnet: true,
            ..Default::default()
        };
        let bitget = Bitget::new_with_options(config, options).unwrap();

        let url = bitget.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        // Bitget测试网REST使用相同URL，通过header区分
        assert_eq!(url, "https://api.bitget.com");
    }

    #[test]
    fn test_ws_endpoint_public_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BitgetOptions {
            testnet: true,
            ..Default::default()
        };
        let bitget = Bitget::new_with_options(config, options).unwrap();

        let url = bitget.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://wspap.bitget.com/v3/ws/public"); // V3 UTA
    }

    #[test]
    fn test_ws_endpoint_private_with_testnet_option() {
        let config = ExchangeConfig::default();
        let options = BitgetOptions {
            testnet: true,
            ..Default::default()
        };
        let bitget = Bitget::new_with_options(config, options).unwrap();

        let url = {
            use ccxt_core::ws::WsContext;
            use ccxt_core::ws::WsEndpointProvider;
            let mut ctx = WsContext::new();
            ctx.is_private = true;
            bitget.ws_private_url(&ctx)
        };
        assert_eq!(url, "wss://wspap.bitget.com/v3/ws/private"); // V3 UTA
    }

    // ==================== Endpoint Type Tests ====================

    #[test]
    fn test_endpoint_type_public_is_public() {
        assert!(EndpointType::Public.is_public());
        assert!(!EndpointType::Public.is_private());
    }

    #[test]
    fn test_endpoint_type_private_is_private() {
        assert!(EndpointType::Private.is_private());
        assert!(!EndpointType::Private.is_public());
    }

    // ==================== URL Format Tests ====================

    #[test]
    fn test_rest_endpoint_uses_https() {
        let bitget = create_test_bitget();
        let url = bitget.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_ws_endpoint_uses_wss() {
        let bitget = create_test_bitget();

        let ws_public = bitget.default_ws_endpoint_by_options();
        assert!(ws_public.starts_with("wss://"));

        let ws_private = {
            use ccxt_core::ws::WsContext;
            use ccxt_core::ws::WsEndpointProvider;
            let mut ctx = WsContext::new();
            ctx.is_private = true;
            bitget.ws_private_url(&ctx)
        };
        assert!(ws_private.starts_with("wss://"));
    }

    #[test]
    fn test_ws_endpoint_contains_v3_path() {
        let bitget = create_test_bitget();

        let ws_public = bitget.default_ws_endpoint_by_options();
        assert!(ws_public.contains("/v3/ws/")); // V3 UTA

        let ws_private = {
            use ccxt_core::ws::WsContext;
            use ccxt_core::ws::WsEndpointProvider;
            let mut ctx = WsContext::new();
            ctx.is_private = true;
            bitget.ws_private_url(&ctx)
        };
        assert!(ws_private.contains("/v3/ws/")); // V3 UTA
    }
}

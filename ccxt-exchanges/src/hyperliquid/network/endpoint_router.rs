//! HyperLiquid WebSocket endpoint provider.
//!
//! This module provides WebSocket endpoint routing for HyperLiquid.
//! REST endpoint routing is now handled by ExchangeEndpointManager.

use ccxt_core::ws::{WsContext, WsEndpointProvider};

/// HyperLiquid WebSocket endpoint provider.
///
/// **DEPRECATED**: Use `ExchangeEndpointManager` instead.
#[derive(Debug, Clone)]
pub struct HyperliquidWsEndpointProvider {
    /// 是否使用沙箱环境
    pub is_sandbox: bool,
}

impl HyperliquidWsEndpointProvider {
    /// 创建新的端点提供者
    pub fn new(is_sandbox: bool) -> Self {
        Self { is_sandbox }
    }
}

impl Default for HyperliquidWsEndpointProvider {
    fn default() -> Self {
        Self { is_sandbox: false }
    }
}

impl WsEndpointProvider for HyperliquidWsEndpointProvider {
    /// 获取 WebSocket URL
    ///
    /// 复用静态配置，单一数据源
    fn ws_public_url(&self, _context: &WsContext) -> String {
        use crate::hyperliquid::core::endpoints::{PRODUCTION_ENDPOINTS, TESTNET_ENDPOINTS};

        let endpoints = if self.is_sandbox {
            &*TESTNET_ENDPOINTS
        } else {
            &*PRODUCTION_ENDPOINTS
        };

        endpoints.websocket.public.to_string()
    }

    /// 获取私有 WebSocket URL
    ///
    /// Hyperliquid 公共和私有使用相同 URL
    fn ws_private_url(&self, context: &WsContext) -> String {
        self.ws_public_url(context)
    }

    /// Hyperliquid 不支持多个 URL
    fn supports_multiple_urls(&self) -> bool {
        false
    }

    /// 获取所有 URL
    fn all_urls(&self) -> Vec<String> {
        vec![self.ws_public_url(&WsContext::new())]
    }
}

#[cfg(test)]
mod tests {
    use crate::hyperliquid::HyperLiquidOptions;
    use ccxt_core::ExchangeConfig;
    use ccxt_core::types::EndpointType;

    fn create_test_hyperliquid() -> crate::hyperliquid::HyperLiquid {
        crate::hyperliquid::HyperLiquid::builder().build().unwrap()
    }

    fn create_testnet_hyperliquid() -> crate::hyperliquid::HyperLiquid {
        crate::hyperliquid::HyperLiquid::builder()
            .testnet(true)
            .build()
            .unwrap()
    }

    fn create_sandbox_hyperliquid() -> crate::hyperliquid::HyperLiquid {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = HyperLiquidOptions::default();
        crate::hyperliquid::HyperLiquid::new_with_options(config, options, None).unwrap()
    }

    // ==================== REST Endpoint Tests ====================

    #[test]
    fn test_rest_endpoint_production() {
        let exchange = create_test_hyperliquid();
        let url = exchange.default_rest_endpoint_by_options(EndpointType::Public);
        assert!(url.contains("api.hyperliquid.xyz"));
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_rest_endpoint_testnet() {
        let exchange = create_testnet_hyperliquid();
        let url = exchange.default_rest_endpoint_by_options(EndpointType::Public);
        assert!(url.contains("api.hyperliquid-testnet.xyz"));
    }

    #[test]
    fn test_rest_endpoint_sandbox() {
        let exchange = create_sandbox_hyperliquid();
        let url = exchange.default_rest_endpoint_by_options(EndpointType::Public);
        assert!(url.contains("api.hyperliquid-testnet.xyz"));
    }

    // ==================== WebSocket Endpoint Tests ====================

    #[test]
    fn test_ws_endpoint_production() {
        let exchange = create_test_hyperliquid();
        let url = exchange.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://api.hyperliquid.xyz/ws");
        assert!(!url.contains("testnet"));
    }

    #[test]
    fn test_ws_endpoint_testnet() {
        let exchange = create_testnet_hyperliquid();
        let url = exchange.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://api.hyperliquid-testnet.xyz/ws");
    }

    #[test]
    fn test_ws_endpoint_sandbox() {
        let exchange = create_sandbox_hyperliquid();
        let url = exchange.default_ws_endpoint_by_options();
        assert_eq!(url, "wss://api.hyperliquid-testnet.xyz/ws");
    }

    // ==================== URL Format Tests ====================

    #[test]
    fn test_rest_endpoint_uses_https() {
        let exchange = create_test_hyperliquid();
        let url = exchange.default_rest_endpoint_by_options(EndpointType::Public);
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_ws_endpoint_uses_wss() {
        let exchange = create_test_hyperliquid();
        let url = exchange.default_ws_endpoint_by_options();
        assert!(url.starts_with("wss://"));
    }

    #[test]
    fn test_ws_endpoint_contains_ws_path() {
        let exchange = create_test_hyperliquid();
        let url = exchange.default_ws_endpoint_by_options();
        assert!(url.ends_with("/ws"));
    }

    // ==================== Sandbox Mode Tests ====================

    #[test]
    fn test_is_sandbox_with_testnet_option() {
        let exchange = create_testnet_hyperliquid();
        assert!(exchange.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let exchange = create_sandbox_hyperliquid();
        assert!(exchange.is_sandbox());
    }

    #[test]
    fn test_is_not_sandbox_by_default() {
        let exchange = create_test_hyperliquid();
        assert!(!exchange.is_sandbox());
    }
}

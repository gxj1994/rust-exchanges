//! Gate.io endpoint router
//!
//! This module provides endpoint routing for different market types (spot/swap/futures)
//! and handles testnet/sandbox mode switching.

use crate::gate::core::options::GateOptions;
use ccxt_core::types::market::MarketType;
use ccxt_core::ws::subscription::MarketType as WsMarketType;
use ccxt_core::ws::{WsContext, WsEndpointProvider};

/// Gate.io REST endpoint router
///
/// Routes requests to the correct API endpoint based on market type and configuration.
pub struct GateEndpointRouter;

impl GateEndpointRouter {
    /// Get the REST API base URL for a given market type
    ///
    /// # Arguments
    ///
    /// * `options` - Gate exchange options
    /// * `market_type` - The market type (spot/swap/futures)
    ///
    /// # Returns
    ///
    /// The base URL for the API endpoint
    ///
    /// # Endpoints
    ///
    /// | Market Type | Live URL | Testnet URL |
    /// |-------------|----------|-------------|
    /// | Spot | https://api.gateio.ws/api/v4 | https://api-testnet.gateapi.io/api/v4 |
    /// | Swap (USDT) | https://fx-api.gateio.ws/api/v4 | https://api-testnet.gateapi.io/api/v4 |
    /// | Swap (USDC) | https://fx-api.gateio.ws/api/v4 | https://api-testnet.gateapi.io/api/v4 |
    pub fn rest_endpoint(options: &GateOptions, market_type: MarketType) -> &'static str {
        #[cfg(test)]
        println!(
            "[DEBUG Gate Endpoint] Market type: {:?}, is_testnet: {}",
            market_type,
            options.is_testnet()
        );

        match market_type {
            MarketType::Spot => {
                // Spot trading testnet support
                if options.is_testnet() {
                    #[cfg(test)]
                    println!(
                        "[DEBUG Gate Endpoint] Using testnet URL for spot: https://api-testnet.gateapi.io"
                    );
                    "https://api-testnet.gateapi.io"
                } else {
                    #[cfg(test)]
                    println!(
                        "[DEBUG Gate Endpoint] Using live URL for spot: https://api.gateio.ws"
                    );
                    "https://api.gateio.ws"
                }
            }
            MarketType::Swap | MarketType::Futures => {
                // Contract trading endpoints
                // Both spot and contract share the same testnet domain: api-testnet.gateapi.io
                if options.is_testnet() {
                    "https://api-testnet.gateapi.io"
                } else {
                    "https://fx-api.gateio.ws"
                }
            }
            _ => "https://api.gateio.ws",
        }
    }

    /// Get the WebSocket endpoint for a given market type
    ///
    /// # Arguments
    ///
    /// * `options` - Gate exchange options
    /// * `market_type` - The market type (spot/swap)
    ///
    /// # Returns
    ///
    /// The WebSocket URL (settle currency is read from options, defaults to "usdt")
    ///
    /// # WebSocket Endpoints
    ///
    /// | Market Type | Live URL | Testnet URL |
    /// |-------------|----------|-------------|
    /// | Spot | wss://api.gateio.ws/ws/v4/ | N/A |
    /// | Swap | wss://fx-ws.gateio.ws/v4/ws/{settle} | wss://fx-ws-testnet.gateio.ws/v4/ws/{settle} |
    pub fn ws_endpoint(options: &GateOptions, market_type: MarketType) -> String {
        match market_type {
            MarketType::Spot => {
                // Spot WebSocket
                "wss://api.gateio.ws/ws/v4/".to_string()
            }
            MarketType::Swap | MarketType::Futures => {
                // Contract WebSocket - settle currency from options
                let settle = options.default_settle();
                if options.is_testnet() {
                    format!("wss://fx-ws-testnet.gateio.ws/v4/ws/{}", settle)
                } else {
                    format!("wss://fx-ws.gateio.ws/v4/ws/{}", settle)
                }
            }
            _ => "wss://api.gateio.ws/ws/v4/".to_string(),
        }
    }

    /// Build the full API path for a request
    ///
    /// # Arguments
    ///
    /// * `market_type` - The market type
    /// * `path` - The API path (e.g., "/spot/tickers")
    ///
    /// # Returns
    ///
    /// The full URL (base + path)
    pub fn build_url(market_type: MarketType, options: &GateOptions, path: &str) -> String {
        let base = Self::rest_endpoint(options, market_type);
        format!("{}{}", base, path)
    }
}

/// Gate.io WebSocket endpoint provider for unified WebSocket architecture
#[derive(Clone)]
pub struct GateWsEndpointProvider {
    is_testnet: bool,
    settle: String,
}

impl GateWsEndpointProvider {
    /// Create a new endpoint provider
    ///
    /// # Arguments
    ///
    /// * `is_testnet` - Whether to use testnet environment
    /// * `settle` - Settlement currency (e.g., "usdt", "usdc", "btc")
    pub fn new(is_testnet: bool, settle: impl Into<String>) -> Self {
        Self {
            is_testnet,
            settle: settle.into(),
        }
    }
}

impl WsEndpointProvider for GateWsEndpointProvider {
    fn ws_public_url(&self, context: &WsContext) -> String {
        // Use market_type from context to select the correct WebSocket endpoint
        // Context is populated from WsContext when connecting
        match context.market_type {
            Some(WsMarketType::Swap) | Some(WsMarketType::Future) => {
                // Futures/Swap WebSocket endpoint
                if self.is_testnet {
                    // Testnet: wss://ws-testnet.gate.com/v4/ws/futures/{settle}
                    format!("wss://ws-testnet.gate.com/v4/ws/futures/{}", self.settle)
                } else {
                    // Production: wss://fx-ws.gateio.ws/v4/ws/{settle}
                    format!("wss://fx-ws.gateio.ws/v4/ws/{}", self.settle)
                }
            }
            _ => {
                // Default to Spot WebSocket endpoint
                if self.is_testnet {
                    // Spot has no dedicated testnet, use contract testnet
                    format!("wss://fx-ws-testnet.gateio.ws/v4/ws/{}", self.settle)
                } else {
                    // Production spot: wss://api.gateio.ws/ws/v4/
                    GateEndpointRouter::ws_endpoint(&GateOptions::spot(), MarketType::Spot)
                }
            }
        }
    }

    fn ws_private_url(&self, context: &WsContext) -> String {
        // Gate.io uses same endpoint for public and private
        self.ws_public_url(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spot_endpoint() {
        let options = GateOptions::spot();
        assert_eq!(
            GateEndpointRouter::rest_endpoint(&options, MarketType::Spot),
            "https://api.gateio.ws"
        );
    }

    #[test]
    fn test_swap_endpoint_live() {
        let options = GateOptions::swap_usdt();
        assert_eq!(
            GateEndpointRouter::rest_endpoint(&options, MarketType::Swap),
            "https://fx-api.gateio.ws"
        );
    }

    #[test]
    fn test_swap_endpoint_testnet() {
        let options = GateOptions {
            testnet: true,
            ..GateOptions::swap_usdt()
        };
        // Gate spot and futures share the same testnet domain
        assert_eq!(
            GateEndpointRouter::rest_endpoint(&options, MarketType::Swap),
            "https://api-testnet.gateapi.io"
        );
    }

    #[test]
    fn test_ws_endpoint_spot() {
        let options = GateOptions::spot();
        assert_eq!(
            GateEndpointRouter::ws_endpoint(&options, MarketType::Spot),
            "wss://api.gateio.ws/ws/v4/"
        );
    }

    #[test]
    fn test_ws_endpoint_swap_usdt_live() {
        let options = GateOptions::swap_usdt();
        assert_eq!(
            GateEndpointRouter::ws_endpoint(&options, MarketType::Swap),
            "wss://fx-ws.gateio.ws/v4/ws/usdt"
        );
    }

    #[test]
    fn test_ws_endpoint_swap_usdt_testnet() {
        let options = GateOptions {
            testnet: true,
            ..GateOptions::swap_usdt()
        };
        assert_eq!(
            GateEndpointRouter::ws_endpoint(&options, MarketType::Swap),
            "wss://fx-ws-testnet.gateio.ws/v4/ws/usdt"
        );
    }

    #[test]
    fn test_ws_endpoint_swap_usdc_live() {
        let options = GateOptions::swap_usdc();
        assert_eq!(
            GateEndpointRouter::ws_endpoint(&options, MarketType::Swap),
            "wss://fx-ws.gateio.ws/v4/ws/usdc"
        );
    }

    #[test]
    fn test_ws_endpoint_swap_btc_testnet() {
        let options = GateOptions {
            testnet: true,
            ..GateOptions::swap_inverse()
        };
        assert_eq!(
            GateEndpointRouter::ws_endpoint(&options, MarketType::Swap),
            "wss://fx-ws-testnet.gateio.ws/v4/ws/btc"
        );
    }

    #[test]
    fn test_build_url() {
        let options = GateOptions::spot();
        let url = GateEndpointRouter::build_url(MarketType::Spot, &options, "/api/v4/spot/tickers");
        assert_eq!(url, "https://api.gateio.ws/api/v4/spot/tickers");
    }
}

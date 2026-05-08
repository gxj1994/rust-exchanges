//! Gate.io Exchange Implementation
//!
//! This module provides the complete implementation for the Gate.io exchange,
//! supporting both spot and contract (swap/futures) trading.
//!
//! # Features
//!
//! - **Spot Trading**: Full spot market support with limit/market orders
//! - **Contract Trading**: USDT/USDC/BTC margined perpetual and futures contracts
//! - **Multi-Settlement**: Configurable settlement currency via `default_sub_type`
//! - **Testnet Support**: Contract trading testnet environment
//! - **Unified Symbol**: CCXT standard symbol format with automatic conversion
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use ccxt_exchanges::gate::Gate;
//! use ccxt_core::types::common::default_type::{DefaultType, DefaultSubType};
//!
//! // Create spot exchange
//! let gate_spot = Gate::builder()
//!     .default_type(DefaultType::Spot)
//!     .build()?;
//!
//! // Create USDT-margined swap exchange
//! let gate_swap = Gate::builder()
//!     .default_type(DefaultType::Swap)
//!     .default_sub_type(DefaultSubType::Linear)
//!     .build()?;
//!
//! // Create testnet exchange
//! let gate_testnet = Gate::builder()
//!     .default_type(DefaultType::Swap)
//!     .testnet(true)
//!     .build()?;
//! ```
//!
//! # API Documentation
//!
//! - REST API: https://www.gate.com/docs/developers/apiv4/zh_CN/
//! - WebSocket: https://www.gate.com/docs/developers/apiv4/ws/zh_CN/

pub mod auth;
pub mod core;
pub mod impls;
pub mod network;
pub mod parser;
pub mod rest;
pub mod swap;
pub mod ws;

use crate::gate::core::options::GateOptions;
use crate::gate::core::symbol::GateSymbolConverter;
use crate::gate::network::rate_limiter::GateRateLimiter;
use ccxt_core::base_exchange::BaseExchange;
use ccxt_core::{ExchangeConfig, Result, SecretString};

/// Gate.io Exchange
///
/// Main exchange struct that provides access to all Gate.io API functionality.
///
/// # Example
///
/// ```rust,ignore
/// use ccxt_exchanges::gate::Gate;
///
/// let gate = Gate::builder()
///     .api_key("your_api_key")
///     .secret("your_secret_key")
///     .build()?;
///
/// // Fetch markets
/// let markets = gate.fetch_markets(None).await?;
///
/// // Create order
/// let order = gate.create_order(&order_request).await?;
/// ```
pub struct Gate {
    /// Base exchange (provides HTTP client, config, etc.)
    pub base: BaseExchange,

    /// Exchange options
    pub options: GateOptions,

    /// Symbol converter
    pub symbol_converter: GateSymbolConverter,

    /// Rate limiter for API requests
    pub rate_limiter: GateRateLimiter,

    /// WebSocket client for public channels (lazily initialized)
    ws_client: std::sync::OnceLock<ws::GateWsClient>,

    /// WebSocket client for private channels (lazily initialized)
    ws_client_auth: std::sync::OnceLock<ws::GateWsClientAuth>,
}

impl Gate {
    /// Creates a new Gate instance with the given exchange configuration.
    ///
    /// This is a convenience constructor for use in tests and conformance suites.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::gate::Gate;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let gate = Gate::new(ExchangeConfig::default()).unwrap();
    /// ```
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        Ok(Self {
            base,
            options: GateOptions::default(),
            symbol_converter: GateSymbolConverter,
            rate_limiter: GateRateLimiter::default(),
            ws_client: std::sync::OnceLock::new(),
            ws_client_auth: std::sync::OnceLock::new(),
        })
    }

    /// Create a new Gate exchange builder
    pub fn builder() -> GateBuilder {
        GateBuilder::default()
    }

    /// Get the default settle currency
    pub fn default_settle(&self) -> &'static str {
        self.options.default_settle()
    }

    /// Check if testnet mode is enabled
    pub fn is_testnet(&self) -> bool {
        self.options.is_testnet()
    }

    /// Get WebSocket client (lazily initialized)
    pub fn ws_client(&self) -> &ws::GateWsClient {
        self.ws_client.get_or_init(|| {
            let settle = self.default_settle().to_string();
            ws::create_gate_ws_client(self.is_testnet(), settle)
        })
    }

    /// Get authenticated WebSocket client for private channels (lazily initialized)
    ///
    /// # Errors
    ///
    /// Returns an error if API credentials are not configured.
    pub fn ws_client_auth(&self) -> Result<&ws::GateWsClientAuth> {
        // Check if already initialized
        if let Some(client) = self.ws_client_auth.get() {
            return Ok(client);
        }

        // Get credentials from config
        let api_key = self
            .base
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| {
                ccxt_core::Error::authentication(
                    "API key is required for private WebSocket channels",
                )
            })?
            .expose_secret();

        let api_secret = self
            .base
            .config
            .secret
            .as_ref()
            .ok_or_else(|| {
                ccxt_core::Error::authentication(
                    "API secret is required for private WebSocket channels",
                )
            })?
            .expose_secret();

        let settle = self.default_settle().to_string();
        let client = ws::create_gate_ws_client_auth(self.is_testnet(), settle, api_key, api_secret);

        // Initialize (ignore if another thread already initialized)
        let _ = self.ws_client_auth.set(client);

        Ok(self.ws_client_auth.get().unwrap())
    }

    /// Make a public GET request to the API.
    ///
    /// This method wraps `http_client.get()` to provide:
    /// - Base URL prepending (e.g., `https://api.gateio.ws/api/v4` + path)
    /// - Rate limiter integration
    pub(crate) async fn public_get(
        &self,
        url: &str,
        _headers: Option<reqwest::header::HeaderMap>,
    ) -> ccxt_core::Result<serde_json::Value> {
        self.rate_limiter.wait().await;
        let base_url = self.get_rest_url();
        let full_url = format!("{}{}", base_url, url);

        #[cfg(test)]
        println!("[DEBUG Gate public_get] Base URL: {}", base_url);
        #[cfg(test)]
        println!("[DEBUG Gate public_get] Full URL: {}", full_url);

        self.base.http_client.get(&full_url, _headers).await
    }
}

/// Gate.io Exchange Builder
///
/// Builder pattern for constructing Gate exchange instances with custom configuration.
///
/// # Example
///
/// ```rust,ignore
/// use ccxt_exchanges::gate::GateBuilder;
/// use ccxt_core::types::common::default_type::{DefaultType, DefaultSubType};
///
/// let gate = GateBuilder::new()
///     .default_type(DefaultType::Swap)
///     .default_sub_type(DefaultSubType::Linear)
///     .api_key("your-api-key")
///     .secret("your-secret")
///     .build()?;
/// ```
pub struct GateBuilder {
    config: ExchangeConfig,
    options: GateOptions,
}

impl Default for GateBuilder {
    fn default() -> Self {
        Self {
            config: ExchangeConfig {
                id: "gate".to_string(),
                name: "Gate.io".to_string(),
                ..Default::default()
            },
            options: GateOptions::default(),
        }
    }
}

impl GateBuilder {
    /// Create a new builder with default options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(SecretString::new(key.into()));
        self
    }

    /// Set the API secret
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.config.secret = Some(SecretString::new(secret.into()));
        self
    }

    /// Set the default trading type
    pub fn default_type(
        mut self,
        default_type: ccxt_core::types::common::default_type::DefaultType,
    ) -> Self {
        self.options.default_type = default_type;
        self
    }

    /// Set the default sub-type for contract settlement
    pub fn default_sub_type(
        mut self,
        default_sub_type: ccxt_core::types::common::default_type::DefaultSubType,
    ) -> Self {
        self.options.default_sub_type = Some(default_sub_type);
        self
    }

    /// Enable testnet mode
    pub fn testnet(mut self, testnet: bool) -> Self {
        self.options.testnet = testnet;
        self
    }

    /// Enable sandbox mode (alias for testnet)
    pub fn sandbox(mut self, sandbox: bool) -> Self {
        self.options.sandbox = sandbox;
        self
    }

    /// Set custom options
    pub fn options(mut self, options: GateOptions) -> Self {
        self.options = options;
        self
    }

    /// Build the Gate exchange instance
    pub fn build(self) -> Result<Gate> {
        let base = BaseExchange::new(self.config)?;

        Ok(Gate {
            base,
            options: self.options,
            symbol_converter: GateSymbolConverter,
            rate_limiter: GateRateLimiter::default(),
            ws_client: std::sync::OnceLock::new(),
            ws_client_auth: std::sync::OnceLock::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};

    #[test]
    fn test_builder_default() {
        let gate = Gate::builder().build().unwrap();
        assert_eq!(gate.options.default_type, DefaultType::Spot);
        assert!(gate.options.default_sub_type.is_none());
        assert!(!gate.is_testnet());
    }

    #[test]
    fn test_builder_spot() {
        let gate = Gate::builder()
            .default_type(DefaultType::Spot)
            .build()
            .unwrap();
        assert_eq!(gate.options.default_type, DefaultType::Spot);
    }

    #[test]
    fn test_builder_swap_usdt() {
        let gate = Gate::builder()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear)
            .build()
            .unwrap();
        assert_eq!(gate.options.default_type, DefaultType::Swap);
        assert_eq!(gate.options.default_sub_type, Some(DefaultSubType::Linear));
        assert_eq!(gate.default_settle(), "usdt");
    }

    #[test]
    fn test_builder_swap_usdc() {
        let gate = Gate::builder()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Usdc)
            .build()
            .unwrap();
        assert_eq!(gate.default_settle(), "usdc");
    }

    #[test]
    fn test_builder_testnet() {
        let gate = Gate::builder().testnet(true).build().unwrap();
        assert!(gate.is_testnet());
    }
}

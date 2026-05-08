//! HyperLiquid exchange implementation.
//!
//! HyperLiquid is a decentralized perpetual futures exchange built on its own L1 blockchain.
//! Unlike centralized exchanges (CEX) like Binance or Bitget, HyperLiquid uses:
//! - Ethereum wallet private keys for authentication (EIP-712 typed data signatures)
//! - Wallet addresses as account identifiers (no registration required)
//! - USDC as the sole settlement currency
//!
//! # Features
//!
//! - Perpetual futures trading with up to 50x leverage
//! - Cross-margin and isolated margin modes
//! - Real-time WebSocket data streaming
//! - EIP-712 compliant transaction signing
//!
//! # Note on Market Types
//!
//! HyperLiquid only supports perpetual futures (Swap). Attempting to configure
//! other market types (Spot, Futures, Margin, Option) will result in an error.
//!
//! # Example
//!
//! ```no_run
//! use ccxt_exchanges::hyperliquid::HyperLiquid;
//!
//! # async fn example() -> ccxt_core::Result<()> {
//! // Create a public-only instance (no authentication)
//! let exchange = HyperLiquid::builder()
//!     .testnet(true)
//!     .build()?;
//!
//! // Fetch markets
//! let markets = exchange.fetch_markets().await?;
//! println!("Found {} markets", markets.len());
//!
//! // Create an authenticated instance
//! let exchange = HyperLiquid::builder()
//!     .private_key("0x...")
//!     .testnet(true)
//!     .build()?;
//!
//! // Fetch balance
//! let balance = exchange.fetch_balance().await?;
//! # Ok(())
//! # }
//! ```

use ccxt_core::types::common::default_type::DefaultType;
use ccxt_core::{BaseExchange, ExchangeConfig, Result};

pub mod auth;
pub mod core;
pub(crate) mod impls;

pub mod network;
pub mod parser;
pub mod rest;
pub mod ws;

pub use auth::{HyperLiquidAuth, HyperliquidWsAuth};
pub use core::error::{HyperLiquidErrorCode, is_error_response, parse_error};
pub use core::{HyperLiquidBuilder, validate_default_type};

/// HyperLiquid exchange structure.
#[derive(Debug, Clone)]
pub struct HyperLiquid {
    /// Base exchange instance.
    base: BaseExchange,
    /// HyperLiquid-specific options.
    options: HyperLiquidOptions,
    /// Authentication instance (optional, for private API).
    auth: Option<HyperLiquidAuth>,
    /// WebSocket client (lazily initialized).
    ws_client: std::sync::OnceLock<ws::HyperliquidWsClient>,
}

/// HyperLiquid-specific options.
///
/// Note: HyperLiquid only supports perpetual futures (Swap). The `default_type`
/// field defaults to `Swap` and attempting to set it to any other value will
/// result in a validation error.
#[derive(Debug, Clone)]
pub struct HyperLiquidOptions {
    /// Whether to use testnet.
    pub testnet: bool,
    /// Vault address for vault trading (optional).
    pub vault_address: Option<String>,
    /// Default leverage multiplier.
    pub default_leverage: u32,
    /// Default market type for trading.
    ///
    /// HyperLiquid only supports perpetual futures, so this must be `Swap`.
    /// Attempting to set any other value will result in a validation error.
    pub default_type: DefaultType,
}

impl Default for HyperLiquidOptions {
    fn default() -> Self {
        Self {
            testnet: false,
            vault_address: None,
            default_leverage: 1,
            // HyperLiquid only supports perpetual futures (Swap)
            default_type: DefaultType::Swap,
        }
    }
}

impl HyperLiquid {
    /// Creates a new HyperLiquid instance using the builder pattern.
    ///
    /// This is the recommended way to create a HyperLiquid instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquid;
    ///
    /// let exchange = HyperLiquid::builder()
    ///     .private_key("0x...")
    ///     .testnet(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> HyperLiquidBuilder {
        HyperLiquidBuilder::new()
    }

    /// Creates a new HyperLiquid instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - HyperLiquid-specific options.
    /// * `auth` - Optional authentication instance.
    pub fn new_with_options(
        config: ExchangeConfig,
        options: HyperLiquidOptions,
        auth: Option<HyperLiquidAuth>,
    ) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        Ok(Self {
            base,
            options,
            auth,
            ws_client: std::sync::OnceLock::new(),
        })
    }

    /// Returns a reference to the base exchange.
    pub fn base(&self) -> &BaseExchange {
        &self.base
    }

    /// Returns a mutable reference to the base exchange.
    pub fn base_mut(&mut self) -> &mut BaseExchange {
        &mut self.base
    }

    /// Returns the HyperLiquid options.
    pub fn options(&self) -> &HyperLiquidOptions {
        &self.options
    }

    /// Returns a reference to the authentication instance.
    pub fn auth(&self) -> Option<&HyperLiquidAuth> {
        self.auth.as_ref()
    }

    /// Creates a signed action builder for authenticated exchange requests.
    ///
    /// This method provides a fluent API for constructing and executing
    /// authenticated Hyperliquid exchange actions using EIP-712 signing.
    ///
    /// # Arguments
    ///
    /// * `action` - The action JSON to be signed and executed
    ///
    /// # Returns
    ///
    /// A `HyperliquidSignedRequestBuilder` that can be configured and executed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquid;
    /// use serde_json::json;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let exchange = HyperLiquid::builder()
    ///     .private_key("0x...")
    ///     .testnet(true)
    ///     .build()?;
    ///
    /// // Create an order
    /// let action = json!({
    ///     "type": "order",
    ///     "orders": [{"a": 0, "b": true, "p": "50000", "s": "0.001", "r": false, "t": {"limit": {"tif": "Gtc"}}}],
    ///     "grouping": "na"
    /// });
    ///
    /// let response = exchange.signed_action(action)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn signed_action(
        &self,
        action: serde_json::Value,
    ) -> auth::signed_request::HyperliquidSignedRequestBuilder<'_> {
        auth::signed_request::HyperliquidSignedRequestBuilder::new(self, action)
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &'static str {
        "hyperliquid"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &'static str {
        "HyperLiquid"
    }

    /// Returns the API version.
    pub fn version(&self) -> &'static str {
        "1"
    }

    /// Returns `true` if the exchange is CCXT-certified.
    pub fn certified(&self) -> bool {
        false
    }

    /// Returns `true` if Pro version (WebSocket) is supported.
    pub fn pro(&self) -> bool {
        true
    }

    /// Returns the rate limit in requests per second.
    pub fn rate_limit(&self) -> u32 {
        // HyperLiquid has generous rate limits
        100
    }

    /// Returns `true` if sandbox/testnet mode is enabled.
    ///
    /// Sandbox mode is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.testnet` is set to `true`
    ///
    /// # Returns
    ///
    /// `true` if sandbox mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquid;
    ///
    /// let exchange = HyperLiquid::builder()
    ///     .testnet(true)
    ///     .build()
    ///     .unwrap();
    /// assert!(exchange.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns the wallet address if authenticated.
    pub fn wallet_address(&self) -> Option<&str> {
        self.auth
            .as_ref()
            .map(auth::HyperLiquidAuth::wallet_address)
    }

    /// Returns a reference to the WebSocket client.
    ///
    /// The client is lazily initialized on first access and reused for subsequent calls.
    /// Connection state is queried directly from the underlying client.
    pub fn ws_client(&self) -> &ws::HyperliquidWsClient {
        self.ws_client
            .get_or_init(|| ws::create_hyperliquid_ws_client(self.is_sandbox()))
    }

    /// Subscribe to real-time best bid/ask (book_ticker) updates.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDC:USDC")
    ///
    /// # Returns
    ///
    /// A stream of `BidAsk` updates
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let hyperliquid = HyperLiquid::builder().testnet(true).build()?;
    /// let mut stream = hyperliquid.watch_bids_asks("BTC/USDC:USDC").await?;
    /// while let Some(ba) = stream.recv().await {
    ///     println!("Best bid: {} @ {}, Best ask: {} @ {}",
    ///              ba.bid_quantity, ba.bid_price, ba.ask_quantity, ba.ask_price);
    /// }
    /// ```
    pub async fn watch_bids_asks(
        &self,
        symbol: &str,
    ) -> ccxt_core::error::Result<ccxt_core::ws_exchange::MessageStream<ccxt_core::types::BidAsk>>
    {
        self.ws_client()
            .watch::<ccxt_core::types::BidAsk>(symbol, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = HyperLiquidOptions::default();
        assert!(!options.testnet);
        assert!(options.vault_address.is_none());
        assert_eq!(options.default_leverage, 1);
        // HyperLiquid only supports perpetual futures, so default_type must be Swap
        assert_eq!(options.default_type, DefaultType::Swap);
    }

    #[test]
    fn test_is_sandbox_with_options_testnet() {
        let config = ExchangeConfig::default();
        let options = HyperLiquidOptions {
            testnet: true,
            ..Default::default()
        };
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        assert!(exchange.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let options = HyperLiquidOptions::default();
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        assert!(exchange.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let options = HyperLiquidOptions::default();
        let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();
        assert!(!exchange.is_sandbox());
    }
}

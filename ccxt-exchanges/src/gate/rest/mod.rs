//! Gate.io REST API implementation organized by functionality.
//!
//! This module provides a modularized structure for Gate.io REST API operations,
//! organized into logical sub-modules by functionality:
//!
//! - `market_data`: Public market data operations (ticker, orderbook, markets)
//! - `trading`: Trading operations (orders, cancellations)
//! - `account`: Account-related operations (balance)
//!
//! # Usage
//!
//! All methods are implemented directly on the `Gate` struct, so you can call them
//! directly on any `Gate` instance:
//!
//! ```ignore
//! # use ccxt_exchanges::gate::Gate;
//! # use ccxt_core::ExchangeConfig;
//! # async fn example() -> ccxt_core::Result<()> {
//! let gate = Gate::builder().build()?;
//!
//! // Market data methods (from market_data.rs)
//! let markets = gate.fetch_markets().await?;
//! let ticker = gate.fetch_ticker("BTC/USDT").await?;
//!
//! // Trading methods (from trading.rs)
//! // let order = gate.create_order(request).await?;
//!
//! // Account methods (from account.rs)
//! // let balance = gate.fetch_balance().await?;
//! # Ok(())
//! # }
//! ```

pub mod account;
pub mod builder;
pub mod market_data;
pub mod trading;

use super::Gate;
// Re-export from auth module for convenience
pub use super::auth::signed_request::{GateSignedRequestBuilder, HttpMethod};

impl Gate {
    /// Get the REST API base URL for the current market type.
    pub fn get_rest_url(&self) -> &str {
        use crate::gate::network::endpoint_router::GateEndpointRouter;
        use ccxt_core::types::market::MarketType;
        GateEndpointRouter::rest_endpoint(&self.options, MarketType::Spot)
    }

    /// Get the contract REST API base URL.
    pub fn get_contract_rest_url(&self) -> &str {
        use crate::gate::network::endpoint_router::GateEndpointRouter;
        use ccxt_core::types::market::MarketType;

        let market_type = match self.options.default_type {
            ccxt_core::types::common::default_type::DefaultType::Swap => MarketType::Swap,
            ccxt_core::types::common::default_type::DefaultType::Futures => MarketType::Futures,
            _ => MarketType::Swap,
        };

        GateEndpointRouter::rest_endpoint(&self.options, market_type)
    }

    /// Create a signed request builder for authenticated API calls.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::gate::Gate;
    /// # use ccxt_exchanges::gate::rest::HttpMethod;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let gate = Gate::builder().build()?;
    ///
    /// let data = gate.signed_request("/api/v4/spot/orders")
    ///     .method(HttpMethod::Post)
    ///     .body(serde_json::json!({}))
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn signed_request(&self, path: &str) -> GateSignedRequestBuilder<'_> {
        GateSignedRequestBuilder::new(self, path)
    }

    /// Convert unified symbol format (BTC/USDT) to Gate format (BTC_USDT).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol in format "BASE/QUOTE"
    ///
    /// # Returns
    ///
    /// Gate-specific symbol in format "BASE_QUOTE"
    #[inline]
    pub fn to_exchange_symbol(symbol: &str) -> String {
        symbol.replace('/', "_")
    }

    /// Get the settle currency string for contract API calls.
    ///
    /// # Returns
    ///
    /// Lowercase settle currency string: "usdt", "usdc", or "btc"
    #[inline]
    pub fn get_contract_settle(&self) -> &'static str {
        use ccxt_core::types::common::default_type::DefaultSubType;

        match self.options.default_sub_type {
            Some(DefaultSubType::Usdc) => "usdc",
            Some(DefaultSubType::Inverse) => "btc",
            _ => "usdt",
        }
    }
}

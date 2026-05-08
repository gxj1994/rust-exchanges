//! Bybit account operations.
//!
//! This module contains account-related methods including balance queries
//! and trade history.

use crate::bybit::{Bybit, parser};
use ccxt_core::{Error, ParseError, Result, types::Balance};

impl Bybit {
    /// Determine Bybit API category from unified symbol.
    ///
    /// Maps unified market types to Bybit V5 API category parameter:
    /// - Linear contracts (USDT/USDC-settled) → `"linear"`
    /// - Inverse contracts (coin-margined) → `"inverse"`
    /// - Spot markets → `"spot"`
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol (e.g., "BTC/USDT:USDT" for linear, "BTC/USD:BTC" for inverse)
    ///
    /// # Returns
    ///
    /// Returns Bybit API category string: `"linear"`, `"inverse"`, or `"spot"`
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The symbol is not found in markets
    /// - The market is a contract but has unknown linear/inverse type
    ///
    /// # Example
    ///
    /// ```no_run
    /// // BTC/USDT:USDT → "linear"
    /// // BTC/USD:BTC → "inverse"
    /// // BTC/USDT → "spot"
    /// ```
    ///
    /// # API Reference
    ///
    /// Bybit V5 API uses category parameter for all endpoints:
    /// <https://bybit-exchange.github.io/docs/v5/introduction>
    pub async fn category_from_symbol(&self, symbol: &str) -> Result<String> {
        let market = self.base().market(symbol).await?;

        // Map MarketType to Bybit V5 API category
        // Reference: https://bybit-exchange.github.io/docs/v5/introduction
        match market.market_type {
            ccxt_core::types::market::MarketType::Spot => Ok("spot".to_string()),
            ccxt_core::types::market::MarketType::Swap
            | ccxt_core::types::market::MarketType::Futures => {
                // Derivatives: determine linear vs inverse
                if market.is_linear() {
                    Ok("linear".to_string())
                } else if market.is_inverse() {
                    Ok("inverse".to_string())
                } else {
                    Err(Error::invalid_request(format!(
                        "Unknown contract type for symbol: {}. Expected linear or inverse.",
                        symbol
                    )))
                }
            }
            ccxt_core::types::market::MarketType::Option => Ok("option".to_string()),
        }
    }

    /// Fetch account balances.
    ///
    /// # Returns
    ///
    /// Returns a [`Balance`] structure with all currency balances.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_balance(&self) -> Result<Balance> {
        let path = Self::build_api_path("/account/wallet-balance");

        let response = self
            .signed_request(&path)
            .param("accountType", &self.options().account_type)
            .execute()
            .await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        parser::parse_balance(result)
    }
}

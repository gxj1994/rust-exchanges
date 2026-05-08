//! Account operations for Bitget REST API.

use super::super::{Bitget, parser};
use ccxt_core::{Error, ParseError, Result, types::Balance};

impl Bitget {
    /// Fetch account balances (V3 API).
    pub async fn fetch_balance(&self) -> Result<Balance> {
        let response = self
            .signed_request("/api/v3/account/assets")
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_balance(data)
    }

    /// Get account settings including position mode (holdMode).
    ///
    /// Uses Bitget V3 GET `/api/v3/account/settings` endpoint.
    ///
    /// # Returns
    /// JSON Value containing account settings:
    /// - `holdMode`: "one_way_mode" or "hedge_mode"
    /// - `accountMode`: "unified" or "hybrid"
    /// - `symbolConfigList`: Array of symbol configurations
    /// - `coinConfigList`: Array of coin configurations
    ///
    /// # Example
    /// ```ignore
    /// let settings = exchange.get_account_settings().await?;
    /// let hold_mode = settings["holdMode"].as_str().unwrap_or("one_way_mode");
    /// ```
    pub async fn get_account_settings(&self) -> Result<serde_json::Value> {
        let response = self
            .signed_request("/api/v3/account/settings")
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        Ok(data.clone())
    }
}

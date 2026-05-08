//! HyperLiquid account operations.
//!
//! This module contains account-related methods including balance queries
//! and position management.

use crate::hyperliquid::{HyperLiquid, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Balance, Trade},
};
use serde_json::Map;

impl HyperLiquid {
    /// Fetch account balance.
    pub async fn fetch_balance(&self) -> Result<Balance> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch balance"))?;

        let response = self
            .info_request("clearinghouseState", {
                let mut map = Map::new();
                map.insert(
                    "user".to_string(),
                    serde_json::Value::String(address.to_string()),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        parser::parse_balance(&response)
    }

    /// Fetch account trades (fills) with pagination.
    ///
    /// Uses userFills endpoint to query historical fills.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol (CCXT standard format, e.g., "BTC/USDC:USDC").
    /// * `since` - Optional timestamp in milliseconds to filter trades from.
    /// * `limit` - Optional maximum number of trades to return.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures.
    pub async fn fetch_account_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch account trades"))?;

        let response = self
            .info_request("userFills", {
                let mut map = Map::new();
                map.insert(
                    "user".to_string(),
                    serde_json::Value::String(address.to_string()),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        let fills_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut trades = Vec::new();

        for fill_data in fills_array {
            // Parse the fill timestamp for filtering
            let fill_timestamp = fill_data["time"].as_i64();

            // Filter by time range if specified
            if let Some(since_ts) = since {
                if let Some(ts) = fill_timestamp {
                    if ts < since_ts {
                        continue;
                    }
                }
            }

            // Parse trade from fill data
            if let Ok(trade) = crate::hyperliquid::parser::parse_trade_from_fill(fill_data, symbol)
            {
                trades.push(trade);
            }
        }

        // Sort by timestamp (newest first)
        trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        if let Some(limit) = limit {
            trades.truncate(limit as usize);
        }

        Ok(trades)
    }
}

//! Bybit position management operations.
//!
//! This module contains position-related methods including
//! setting take profit, stop loss, and trailing stop for positions.

use crate::bybit::Bybit;
use ccxt_core::{Error, ParseError, Result, types::Position};
use serde_json::Value;
use tracing::warn;

impl Bybit {
    // ========================================================================
    // Position Management
    // ========================================================================

    /// Fetch positions for specific symbols.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional unified symbol to query (e.g., "BTC/USDT:USDT")
    ///   - If provided, queries position for this specific symbol
    ///   - API returns data even if position is empty when symbol is specified
    ///
    /// * `filter` - Optional list of unified symbols to filter from all positions
    ///   - If symbol is None and filter is provided, fetches all positions then filters
    ///   - Useful for querying multiple symbols at once
    ///
    /// # API Endpoint
    ///
    /// `GET /v5/position/list`
    ///
    /// # Bybit V5 API Notes
    ///
    /// - category is required: linear, inverse, option
    /// - symbol: If passed, returns data for this symbol regardless of position size
    /// - If symbol is not passed, only returns positions with size > 0
    /// - Does not support multiple symbols in one request
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Query single symbol
    /// let positions = bybit.fetch_positions(Some("BTC/USDT:USDT".to_string()), None).await?;
    ///
    /// // Query multiple symbols (fetches all, then filters)
    /// let positions = bybit.fetch_positions(
    ///     None,
    ///     Some(vec!["BTC/USDT:USDT".to_string(), "ETH/USDT:USDT".to_string()])
    /// ).await?;
    ///
    /// // Query all positions for linear category
    /// let positions = bybit.fetch_positions(None, None).await?;
    /// ```
    pub async fn fetch_positions(
        &self,
        symbol: Option<String>,
        filter: Option<Vec<String>>,
    ) -> Result<Vec<Position>> {
        // Determine category based on symbol or default to linear
        let category = if let Some(ref sym) = symbol {
            self.category_from_symbol(sym).await?
        } else if let Some(ref syms) = filter {
            if !syms.is_empty() {
                self.category_from_symbol(&syms[0]).await?
            } else {
                self.category().to_string()
            }
        } else {
            self.category().to_string()
        };

        let mut builder = self
            .signed_request("/v5/position/list")
            .param("category", &category);

        // Add symbol parameter if provided (higher priority)
        if let Some(ref sym) = symbol {
            let market = self.base().market(sym).await?;
            builder = builder.param("symbol", market.id.clone());
        }

        let response = builder.execute().await?;

        // Check response
        let ret_code = response["retCode"].as_i64().unwrap_or(-1);
        if ret_code != 0 {
            let ret_msg = response["retMsg"].as_str().unwrap_or("Unknown error");
            return Err(Error::invalid_request(format!(
                "Bybit fetch_positions failed: retCode={}, retMsg={}",
                ret_code, ret_msg
            )));
        }

        // Parse positions from response
        let positions_array = response["result"]["list"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of positions",
            ))
        })?;

        let mut positions = Vec::new();
        for pos_data in positions_array {
            match self.parse_position(pos_data).await {
                Ok(pos) => {
                    // If symbol was specified, include all positions (even empty ones)
                    // Otherwise, only include non-zero positions
                    if symbol.is_some() || pos.contracts.unwrap_or(0.0) > 0.0 {
                        positions.push(pos);
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse position");
                }
            }
        }

        // Apply filter if provided and symbol was not specified
        if symbol.is_none() && filter.is_some() {
            let filter_symbols = filter.as_ref().unwrap();
            positions.retain(|pos| filter_symbols.contains(&pos.symbol));
        }

        Ok(positions)
    }

    /// Parse a single position from Bybit API response.
    async fn parse_position(&self, data: &Value) -> Result<Position> {
        let bybit_symbol = data["symbol"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?;

        // Get market info to convert symbol
        let market = self.base().market_by_id(bybit_symbol).await?;
        let symbol = market.symbol.as_str().to_string();

        let szi = data["size"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let side = match data["side"].as_str() {
            Some("Buy") => Some("long".to_string()),
            Some("Sell") => Some("short".to_string()),
            _ => None,
        };

        let entry_price = data["avgPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());

        let leverage = data["leverage"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());

        let unrealized_pnl = data["unrealisedPnl"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());

        let liquidation_price = data["liqPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());

        let margin_mode = match data["isIsolated"].as_str() {
            Some("1") => Some("isolated".to_string()),
            Some("0") => Some("cross".to_string()),
            _ => None,
        };

        let mark_price = data["markPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());

        // Parse positionIdx to determine position mode
        // 0: One-Way Mode
        // 1: Buy side of hedge mode
        // 2: Sell side of hedge mode
        let position_idx = data["positionIdx"].as_i64().unwrap_or(0);
        let hedged = match position_idx {
            0 => Some(false),    // One-way mode
            1 | 2 => Some(true), // Hedge mode
            _ => None,
        };

        let position = Position {
            info: data.clone(),
            id: None,
            symbol,
            side,
            position_side: None,
            dual_side_position: hedged,
            contracts: Some(szi),
            contract_size: Some(1.0),
            entry_price,
            mark_price,
            notional: None,
            leverage,
            collateral: None,
            initial_margin: None,
            initial_margin_percentage: None,
            maintenance_margin: None,
            maintenance_margin_percentage: None,
            unrealized_pnl,
            realized_pnl: None,
            liquidation_price,
            margin_ratio: None,
            margin_mode,
            hedged,
            percentage: None,
            timestamp: None,
            datetime: None,
        };

        Ok(position)
    }
}

//! HyperLiquid Margin API Implementation.
//!
//! This module provides margin-related API methods for HyperLiquid.
//!
//! # Key Features
//!
//! - **Unified Leverage & Margin Mode**: HyperLiquid uses `updateLeverage` to set
//!   both leverage and margin mode (cross/isolated) in a single API call
//! - **DEX Architecture**: All margin operations use signed actions via HyperLiquid's
//!   L1 consensus mechanism
//! - **Position-based Leverage Query**: Leverage info is retrieved from position data
//!
//! # API Design Notes
//!
//! HyperLiquid's approach differs from CEXs:
//! - `set_leverage(leverage, is_cross)` - Sets both leverage AND margin mode together
//! - `get_leverage()` - Must query position data to get current leverage
//! - `set_margin_mode()` - Must read current leverage first, then re-set with new mode

use ccxt_core::Error;
use ccxt_core::Result;
use ccxt_core::error::ParseError;
use ccxt_core::types::trading::params::MarginMode;
use ccxt_core::types::{FundingRate, FundingRateHistory};
use serde_json::{Map, Value};
use tracing::info;

use crate::hyperliquid::HyperLiquid;

impl HyperLiquid {
    /// Get current leverage for a symbol.
    ///
    /// # Note
    ///
    /// HyperLiquid does not have a dedicated leverage query endpoint.
    /// This method fetches position data to extract leverage information.
    /// If no position exists, returns default leverage of 1x.
    pub async fn get_leverage(&self, symbol: &str) -> Result<u32> {
        // Try to get leverage from position
        let positions = self.fetch_positions(Some(vec![symbol.to_string()])).await?;

        if let Some(position) = positions.into_iter().next() {
            if let Some(leverage) = position.leverage {
                return Ok(leverage as u32);
            }
        }

        // Default leverage if no position exists
        Ok(1)
    }

    /// Set leverage for a symbol.
    ///
    /// # Note
    ///
    /// Leverage is only supported for perpetual swaps (futures), not for spot markets.
    /// Spot markets do not support margin trading or leverage.
    pub async fn set_leverage(&self, symbol: &str, leverage: u32, is_cross: bool) -> Result<()> {
        let market = self.base().market(symbol).await?;

        // Check if market is a perpetual swap (supports leverage)
        if !market.contract.unwrap_or(false) {
            return Err(Error::invalid_request(
                "Leverage is only supported for perpetual swap markets, not spot markets",
            ));
        }

        let asset_index: u32 = market.id.parse().unwrap_or(0);

        let action = {
            let mut map = Map::new();
            map.insert(
                "type".to_string(),
                serde_json::Value::String("updateLeverage".to_string()),
            );
            map.insert(
                "asset".to_string(),
                serde_json::Value::Number(asset_index.into()),
            );
            map.insert("isCross".to_string(), serde_json::Value::Bool(is_cross));
            map.insert(
                "leverage".to_string(),
                serde_json::Value::Number(leverage.into()),
            );
            serde_json::Value::Object(map)
        };

        let response = self.signed_action(action).execute().await?;

        // Check for success
        if crate::hyperliquid::core::error::is_error_response(&response) {
            return Err(crate::hyperliquid::core::error::parse_error(&response));
        }

        info!("Set leverage for {} to {}x", symbol, leverage);

        Ok(())
    }

    /// Switch margin mode (cross/isolated).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDC:USDC")
    /// * `mode` - Target margin mode (Cross or Isolated)
    ///
    /// # Implementation Strategy
    ///
    /// HyperLiquid does not have a dedicated margin mode switch endpoint.
    /// This method:
    /// 1. Fetches current leverage from position (or uses default 1x)
    /// 2. Re-applies the same leverage with the new margin mode
    ///
    /// This is necessary because HyperLiquid's `updateLeverage` action
    /// sets BOTH leverage and margin mode together.
    pub async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()> {
        let market = self.base().market(symbol).await?;

        // Check if market is a perpetual swap (supports margin mode)
        if !market.contract.unwrap_or(false) {
            return Err(Error::invalid_request(
                "Margin mode switching is only supported for perpetual swap markets",
            ));
        }

        // Get current leverage (from position or default)
        let current_leverage = self.get_leverage(symbol).await?;

        let is_cross = match mode {
            MarginMode::Cross => true,
            MarginMode::Isolated => false,
        };

        // Re-apply leverage with new margin mode
        self.set_leverage(symbol, current_leverage, is_cross)
            .await?;

        info!("Switched margin mode for {} to {:?}", symbol, mode);

        Ok(())
    }

    /// Fetch funding rates for multiple symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of symbols to fetch (None = all symbols)
    ///
    /// # Returns
    ///
    /// A map of symbol to `FundingRate`.
    pub async fn fetch_funding_rates(
        &self,
        symbols: Option<&[String]>,
    ) -> Result<std::collections::HashMap<String, FundingRate>> {
        let response = self
            .info_request("metaAndAssetCtxs", Value::Array(vec![]))
            .await?;

        // Parse response: [meta, asset_contexts]
        let meta = response
            .as_array()
            .and_then(|arr| arr.get(0))
            .ok_or_else(|| Error::from(ParseError::missing_field("meta")))?;

        let asset_contexts = response
            .as_array()
            .and_then(|arr| arr.get(1))
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::from(ParseError::missing_field("asset contexts")))?;

        let universe = meta["universe"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("universe")))?;

        let mut rates = std::collections::HashMap::new();

        for (index, ctx) in asset_contexts.iter().enumerate() {
            let coin = universe[index]["name"].as_str().unwrap_or("");

            let symbol = format!("{}/USDC:USDC", coin);

            // Filter by symbols if provided
            if let Some(syms) = symbols {
                if !syms.contains(&symbol) {
                    continue;
                }
            }

            let funding_rate = ctx["funding"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            let mark_price = ctx["markPx"].as_str().and_then(|s| s.parse::<f64>().ok());

            let timestamp = ctx["time"]
                .as_i64()
                .or_else(|| ctx.get("timeMs").and_then(|v| v.as_i64()));

            rates.insert(
                symbol.clone(),
                FundingRate {
                    info: ctx.clone(),
                    symbol,
                    mark_price,
                    index_price: None,
                    interest_rate: None,
                    estimated_settle_price: None,
                    funding_rate: Some(funding_rate),
                    funding_timestamp: timestamp,
                    funding_datetime: None,
                    previous_funding_rate: None,
                    previous_funding_timestamp: None,
                    previous_funding_datetime: None,
                    timestamp: None,
                    datetime: None,
                },
            );
        }

        Ok(rates)
    }

    /// Fetch funding rate history.
    ///
    /// # Note
    ///
    /// HyperLiquid does not provide a dedicated funding rate history endpoint
    /// in the same way as CEXs. This method is a placeholder that returns
    /// an empty vector.
    ///
    /// For historical funding rates, you would need to query HyperLiquid's
    /// archival data or use external data providers.
    pub async fn fetch_funding_rate_history(
        &self,
        _symbol: &str,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        // HyperLiquid does not provide funding rate history via REST API
        Ok(Vec::new())
    }
}

//! Bybit margin and leverage operations.
//!
//! This module implements:
//! - Leverage management (set/get leverage)
//! - Margin mode switching (cross/isolated)
//! - Position queries
//! - Funding rate queries
//!
//! # API Endpoints
//!
//! - Set Leverage: `POST /v5/position/set-leverage`
//! - Switch Margin Mode: `POST /v5/position/switch-isolated`
//! - Fetch Positions: `GET /v5/position/list`
//! - Fetch Funding Rate: `GET /v5/market/tickers`
//! - Fetch Funding Rate History: `GET /v5/market/history-funding-rate`

use crate::bybit::Bybit;
use ccxt_core::types::trading::params::MarginMode;
use ccxt_core::{Error, Result};
use serde_json::{Map, Value};
use tracing::{info, warn};

impl Bybit {
    // ========================================================================
    // Leverage Management
    // ========================================================================

    /// Set leverage for a symbol.
    ///
    /// According to Bybit API:
    /// - One-way mode: buyLeverage must equal sellLeverage
    /// - Hedge mode (isolated): buyLeverage and sellLeverage can differ
    /// - Hedge mode (cross): buyLeverage must equal sellLeverage
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol (e.g., "BTC/USDT:USDT")
    /// * `leverage` - Leverage multiplier (1-100)
    ///
    /// # API Endpoint
    ///
    /// `POST /v5/position/set-leverage`
    pub async fn set_leverage(&self, symbol: &str, leverage: u32) -> Result<()> {
        let category = self.category_from_symbol(symbol).await?;

        if category == "spot" {
            return Err(Error::invalid_request(
                "Leverage is not supported for spot markets",
            ));
        }

        let market = self.base().market(symbol).await?;
        let leverage_str = leverage.to_string();

        let body = {
            let mut map = Map::new();
            map.insert("category".to_string(), Value::String(category));
            map.insert("symbol".to_string(), Value::String(market.id.clone()));
            map.insert(
                "buyLeverage".to_string(),
                Value::String(leverage_str.clone()),
            );
            map.insert("sellLeverage".to_string(), Value::String(leverage_str));
            Value::Object(map)
        };

        let response = self
            .signed_request("/v5/position/set-leverage")
            .method(crate::bybit::signed_request::HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        // Check response
        let ret_code = response["retCode"].as_i64().unwrap_or(-1);
        if ret_code != 0 {
            let ret_msg = response["retMsg"].as_str().unwrap_or("Unknown error");
            return Err(Error::invalid_request(format!(
                "Bybit set_leverage failed: retCode={}, retMsg={}",
                ret_code, ret_msg
            )));
        }

        info!("Set leverage for {} to {}x", symbol, leverage);
        Ok(())
    }

    /// Get current leverage for a symbol.
    ///
    /// Fetches position information and extracts leverage from it.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol (e.g., "BTC/USDT:USDT")
    ///
    /// # Returns
    ///
    /// Returns current leverage multiplier
    pub async fn get_leverage(&self, symbol: &str) -> Result<u32> {
        let positions = self.fetch_positions(Some(symbol.to_string()), None).await?;

        if positions.is_empty() {
            // No position found, return default leverage
            warn!(
                "No position found for {}, returning default leverage 1x",
                symbol
            );
            return Ok(1);
        }

        // Get leverage from the first position
        positions[0].leverage.map(|l| l as u32).ok_or_else(|| {
            Error::invalid_request(format!("No leverage information for {}", symbol))
        })
    }

    // ========================================================================
    // Margin Mode Management
    // ========================================================================

    /// Switch margin mode (cross/isolated).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol (e.g., "BTC/USDT:USDT")
    /// * `mode` - Margin mode (Cross or Isolated)
    ///
    /// # API Endpoint
    ///
    /// `POST /v5/position/switch-isolated`
    ///
    /// # Trade Mode Values
    ///
    /// - 0: Cross margin
    /// - 1: Isolated margin
    pub async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()> {
        let category = self.category_from_symbol(symbol).await?;

        if category == "spot" {
            return Err(Error::invalid_request(
                "Margin mode switching is not supported for spot markets",
            ));
        }

        let market = self.base().market(symbol).await?;
        let trade_mode = match mode {
            MarginMode::Cross => 0,
            MarginMode::Isolated => 1,
        };

        let body = {
            let mut map = Map::new();
            map.insert("category".to_string(), Value::String(category));
            map.insert("symbol".to_string(), Value::String(market.id.clone()));
            map.insert("tradeMode".to_string(), Value::Number(trade_mode.into()));
            Value::Object(map)
        };

        let response = self
            .signed_request("/v5/position/switch-isolated")
            .method(crate::bybit::signed_request::HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        // Check response
        let ret_code = response["retCode"].as_i64().unwrap_or(-1);
        if ret_code != 0 {
            let ret_msg = response["retMsg"].as_str().unwrap_or("Unknown error");
            return Err(Error::invalid_request(format!(
                "Bybit set_margin_mode failed: retCode={}, retMsg={}",
                ret_code, ret_msg
            )));
        }

        info!("Switched margin mode for {} to {:?}", symbol, mode);
        Ok(())
    }
}

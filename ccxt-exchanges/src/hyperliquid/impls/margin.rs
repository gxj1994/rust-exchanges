//! Margin Trait implementation for HyperLiquid.
//!
//! This module implements the Margin trait for HyperLiquid DEX,
//! providing leverage and margin mode management.
//!
//! # Architecture Notes
//!
//! HyperLiquid is a DEX (Decentralized Exchange) with a unique approach:
//! - Uses L1 consensus for all margin operations
//! - Combines leverage and margin mode setting in a single `updateLeverage` action
//! - No separate margin mode switch endpoint - must re-apply leverage with new mode
//!
//! # Implementation Details
//!
//! The trait implementation maps HyperLiquid's unified API to the standard
//! Margin trait interface:
//!
//! - `set_leverage()` → Calls HyperLiquid's `set_leverage(leverage, is_cross=false)`
//! - `set_margin_mode()` → Reads current leverage, then re-applies with new mode
//! - `get_leverage()` → Extracts from position data or returns default 1x
//! - `fetch_positions()` → Parses from `clearinghouseState` endpoint

use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Margin,
    types::{
        FundingRate, FundingRateHistory, Position,
        trading::params::{LeverageParams, MarginMode},
    },
};

use crate::hyperliquid::HyperLiquid;

#[async_trait]
impl Margin for HyperLiquid {
    // ========================================================================
    // Position Management
    // ========================================================================

    /// Fetch positions.
    async fn fetch_positions_for(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        let symbols_option = if symbols.is_empty() {
            None
        } else {
            Some(symbols.iter().map(|s| s.to_string()).collect())
        };

        // Call account module implementation
        HyperLiquid::fetch_positions(self, symbols_option).await
    }

    /// Fetch single position.
    async fn fetch_position(&self, symbol: &str) -> Result<Position> {
        let positions = HyperLiquid::fetch_positions(self, Some(vec![symbol.to_string()])).await?;

        positions.into_iter().next().ok_or_else(|| {
            ccxt_core::Error::invalid_request(format!("No position found for {}", symbol))
        })
    }

    // ========================================================================
    // Leverage Management
    // ========================================================================

    /// Set leverage.
    ///
    /// For HyperLiquid, this sets leverage with isolated margin mode.
    async fn set_leverage_with_params(&self, params: LeverageParams) -> Result<()> {
        // Default to isolated mode for set_leverage
        let is_cross = false;

        // Call trading module implementation
        HyperLiquid::set_leverage(self, &params.symbol, params.leverage, is_cross).await
    }

    /// Get current leverage.
    ///
    /// Retrieves leverage from position data or returns default 1x.
    async fn get_leverage(&self, symbol: &str) -> Result<u32> {
        // Call margin module implementation
        HyperLiquid::get_leverage(self, symbol).await
    }

    // ========================================================================
    // Margin Mode
    // ========================================================================

    /// Set margin mode.
    ///
    /// For HyperLiquid, this reads current leverage and re-applies it with
    /// the new margin mode, since HyperLiquid combines both operations.
    async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()> {
        // Call margin module implementation directly
        // No conversion needed since rest/margin.rs now accepts MarginMode
        HyperLiquid::set_margin_mode(self, symbol, mode).await
    }

    // ========================================================================
    // Funding Rates
    // ========================================================================

    /// Fetch funding rate.
    async fn fetch_funding_rate(&self, symbol: &str) -> Result<FundingRate> {
        // Call the market_data implementation using turbofish syntax
        HyperLiquid::fetch_funding_rate(self, symbol).await
    }

    /// Fetch funding rates.
    async fn fetch_funding_rates(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
        let symbols_option = if symbols.is_empty() {
            None
        } else {
            Some(symbols.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };

        // Call margin module implementation
        let rates_map = HyperLiquid::fetch_funding_rates(self, symbols_option.as_deref()).await?;

        Ok(rates_map.into_values().collect())
    }

    /// Fetch funding rate history.
    ///
    /// Returns empty vector as HyperLiquid does not provide this via REST API.
    async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        // Call margin module implementation
        HyperLiquid::fetch_funding_rate_history(self, symbol, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperliquid::HyperLiquidOptions;
    use ccxt_core::ExchangeConfig;
    use ccxt_core::types::common::default_type::DefaultType;

    #[test]
    fn test_hyperliquid_margin_trait_object_safety() {
        let options = HyperLiquidOptions {
            default_type: DefaultType::Swap,
            ..Default::default()
        };
        let hyperliquid = HyperLiquid::new_with_options(
            ExchangeConfig::default(),
            options,
            None, // No auth for this test
        )
        .unwrap();
        let margin: Box<dyn Margin> = Box::new(hyperliquid);

        assert_eq!(margin.id(), "hyperliquid");

        let caps = margin.capabilities();
        assert!(caps.set_leverage(), "set_leverage should be supported");
        assert!(
            caps.set_margin_mode(),
            "set_margin_mode should be supported"
        );
        assert!(
            caps.fetch_positions(),
            "fetch_positions should be supported"
        );
    }
}

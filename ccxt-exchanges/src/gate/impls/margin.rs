//! Margin Trait implementation for Gate.io
//!
//! This module implements the Margin trait for Gate.io exchange,
//! providing leverage and margin mode management for contract trading.
//!
//! # Implementation Status
//!
//! - `fetch_positions` / `fetch_positions_for` / `fetch_position`: Wired to `fetch_contract_positions`
//! - `set_leverage_with_params` / `get_leverage`: Returns `not_implemented` (API endpoint not yet added)
//! - `set_margin_mode`: Returns `not_implemented` (API endpoint not yet added)
//! - `fetch_funding_rate` / `fetch_funding_rates` / `fetch_funding_rate_history`: Returns `not_implemented`

use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Margin,
    types::{
        FundingRate, FundingRateHistory, Position,
        trading::params::{LeverageParams, MarginMode},
    },
};

use crate::gate::Gate;

#[async_trait]
impl Margin for Gate {
    // ========================================================================
    // Position Management
    // ========================================================================

    async fn fetch_positions_for(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        let positions = Gate::fetch_contract_positions(self).await?;

        if symbols.is_empty() {
            return Ok(positions);
        }

        // Filter positions by requested symbols
        Ok(positions
            .into_iter()
            .filter(|p| symbols.contains(&p.symbol.as_str()))
            .collect())
    }

    async fn fetch_position(&self, symbol: &str) -> Result<Position> {
        let positions = Gate::fetch_contract_positions(self).await?;

        positions
            .into_iter()
            .find(|p| p.symbol == symbol)
            .ok_or_else(|| {
                ccxt_core::Error::not_implemented(format!(
                    "Position not found for symbol: {symbol}"
                ))
            })
    }

    // ========================================================================
    // Leverage Management
    // ========================================================================

    async fn set_leverage_with_params(&self, _params: LeverageParams) -> Result<()> {
        Err(ccxt_core::Error::not_implemented(
            "set_leverage_with_params - Gate contract leverage API not yet implemented",
        ))
    }

    async fn get_leverage(&self, _symbol: &str) -> Result<u32> {
        Err(ccxt_core::Error::not_implemented(
            "get_leverage - Gate contract leverage API not yet implemented",
        ))
    }

    // ========================================================================
    // Margin Mode
    // ========================================================================

    async fn set_margin_mode(&self, _symbol: &str, _mode: MarginMode) -> Result<()> {
        Err(ccxt_core::Error::not_implemented(
            "set_margin_mode - Gate contract margin mode API not yet implemented",
        ))
    }

    // ========================================================================
    // Funding Rates
    // ========================================================================

    async fn fetch_funding_rate(&self, _symbol: &str) -> Result<FundingRate> {
        Err(ccxt_core::Error::not_implemented(
            "fetch_funding_rate - Gate funding rate API not yet implemented",
        ))
    }

    async fn fetch_funding_rates(&self, _symbols: &[&str]) -> Result<Vec<FundingRate>> {
        Err(ccxt_core::Error::not_implemented(
            "fetch_funding_rates - Gate funding rate API not yet implemented",
        ))
    }

    async fn fetch_funding_rate_history(
        &self,
        _symbol: &str,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        Err(ccxt_core::Error::not_implemented(
            "fetch_funding_rate_history - Gate funding rate history API not yet implemented",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::GateBuilder;
    use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};

    #[test]
    fn test_gate_margin_trait_object_safety() {
        let gate = GateBuilder::new()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear)
            .build()
            .unwrap();
        let margin: Box<dyn Margin> = Box::new(gate);

        // Verify trait object can be created
        assert_eq!(margin.id(), "gate");
    }

    #[test]
    fn test_gate_margin_not_implemented_methods() {
        let gate = GateBuilder::new()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear)
            .build()
            .unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();

        // Leverage methods should return not_implemented
        let result = rt.block_on(gate.set_leverage("BTC/USDT:USDT", 10));
        assert!(result.is_err());

        let result = rt.block_on(gate.get_leverage("BTC/USDT:USDT"));
        assert!(result.is_err());

        // Margin mode should return not_implemented
        let result = rt.block_on(gate.set_margin_mode("BTC/USDT:USDT", MarginMode::Isolated));
        assert!(result.is_err());

        // Funding rate methods should return not_implemented
        let result = rt.block_on(gate.fetch_funding_rate("BTC/USDT:USDT"));
        assert!(result.is_err());

        let result = rt.block_on(gate.fetch_funding_rate_history("BTC/USDT:USDT", None, Some(10)));
        assert!(result.is_err());
    }
}

//! Margin trait implementation for OKX.
//!
//! Implements the `Margin` trait from `ccxt-core` for OKX, providing
//! position management, leverage configuration, margin mode, and funding rate operations.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Margin,
    types::{
        FundingRate, FundingRateHistory, Position,
        trading::params::{LeverageParams, MarginMode},
    },
};

use crate::okx::Okx;

// ============================================================================
// Margin Trait Implementation
// ============================================================================

#[async_trait]
impl Margin for Okx {
    async fn fetch_positions_for(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        self.fetch_positions_impl(symbols).await
    }

    async fn fetch_position(&self, symbol: &str) -> Result<Position> {
        self.fetch_position_impl(symbol).await
    }

    async fn set_leverage_with_params(&self, params: LeverageParams) -> Result<()> {
        let margin_mode = params.margin_mode.map(|m| match m {
            MarginMode::Isolated => "isolated",
            MarginMode::Cross => "cross",
        });
        self.set_leverage_impl(&params.symbol, params.leverage, margin_mode)
            .await
    }

    async fn get_leverage(&self, symbol: &str) -> Result<u32> {
        self.get_leverage_impl(symbol).await
    }

    async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()> {
        let mode_str = match mode {
            MarginMode::Isolated => "isolated",
            MarginMode::Cross => "cross",
        };
        self.set_margin_mode_impl(symbol, mode_str).await
    }

    async fn fetch_funding_rate(&self, symbol: &str) -> Result<FundingRate> {
        self.fetch_funding_rate_impl(symbol).await
    }

    async fn fetch_funding_rates(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
        self.fetch_funding_rates_impl(symbols).await
    }

    async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        self.fetch_funding_rate_history_impl(symbol, since, limit)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_okx_margin_trait_object_safety() {
        let okx = Okx::new(ExchangeConfig::default()).unwrap();
        // Verify trait is object-safe by creating a trait object
        let _margin: Box<dyn Margin> = Box::new(okx);
    }
}

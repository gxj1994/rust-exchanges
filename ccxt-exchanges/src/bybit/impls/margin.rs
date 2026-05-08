//! Margin Trait implementation for Bybit.
//!
//! This module implements the Margin trait for Bybit exchange,
//! providing leverage and margin mode management.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Margin,
    types::{
        FundingRate, FundingRateHistory, Position,
        trading::params::{LeverageParams, MarginMode},
    },
};

use crate::bybit::Bybit;

#[async_trait]
impl Margin for Bybit {
    // ========================================================================
    // Position Management
    // ========================================================================
    async fn fetch_positions_for(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        // If single symbol, query directly; otherwise fetch all and filter
        if symbols.len() == 1 {
            self.fetch_positions(Some(symbols[0].to_string()), None)
                .await
        } else if symbols.is_empty() {
            self.fetch_positions(None, None).await
        } else {
            // Multiple symbols: fetch all and filter
            let filter: Vec<String> = symbols.iter().map(|s| s.to_string()).collect();
            self.fetch_positions(None, Some(filter)).await
        }
    }

    async fn fetch_position(&self, symbol: &str) -> Result<Position> {
        // Fetch positions for the specific symbol
        let positions = self.fetch_positions(Some(symbol.to_string()), None).await?;

        // Return the first position or error if not found
        positions.into_iter().next().ok_or_else(|| {
            ccxt_core::Error::invalid_request(format!("No position found for {}", symbol))
        })
    }

    // ========================================================================
    // Leverage Management
    // ========================================================================

    async fn set_leverage_with_params(&self, params: LeverageParams) -> Result<()> {
        // Bybit's set_leverage doesn't support margin mode in the same call
        // Margin mode must be set separately
        self.set_leverage(&params.symbol, params.leverage).await
    }

    async fn get_leverage(&self, symbol: &str) -> Result<u32> {
        // Call the underlying get_leverage method
        self.get_leverage(symbol).await
    }

    // ========================================================================
    // Margin Mode
    // ========================================================================

    async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()> {
        // Call the underlying set_margin_mode method directly
        // No conversion needed since rest/margin.rs now accepts MarginMode
        Bybit::set_margin_mode(self, symbol, mode).await
    }

    // ========================================================================
    // Funding Rates
    // ========================================================================

    async fn fetch_funding_rate(&self, symbol: &str) -> Result<FundingRate> {
        // Call the underlying fetch_funding_rate method
        self.fetch_funding_rate(symbol).await
    }

    async fn fetch_funding_rates(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
        // Convert &[&str] to Option<&[String]> for the underlying method
        let symbols_option = if symbols.is_empty() {
            None
        } else {
            Some(symbols.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };

        // Call the underlying fetch_funding_rates method
        let rates_map = self.fetch_funding_rates(symbols_option.as_deref()).await?;

        // Convert BTreeMap to Vec
        Ok(rates_map.into_values().collect())
    }

    async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        // Call the underlying fetch_funding_rate_history method
        self.fetch_funding_rate_history(symbol, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bybit::BybitOptions;
    use ccxt_core::ExchangeConfig;
    use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};

    #[test]
    fn test_bybit_margin_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(ExchangeConfig::default(), options).unwrap();
        let margin: Box<dyn Margin> = Box::new(bybit);

        // Verify the exchange ID
        assert_eq!(margin.id(), "bybit");

        // Verify capabilities include margin trading features
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

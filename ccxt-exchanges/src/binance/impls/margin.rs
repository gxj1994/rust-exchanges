//! Margin Trait implementation for Binance.
//!
//! This module implements the Margin trait for Binance exchange,
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

use crate::binance::Binance;

/// Helper function to convert Decimal to f64
fn decimal_to_f64(d: Option<rust_decimal::Decimal>) -> Option<f64> {
    d.map(|dec| {
        // Convert Decimal to string then parse as f64
        dec.to_string().parse::<f64>().unwrap_or(0.0)
    })
}

/// Helper function to convert FeeFundingRate to FundingRate
fn convert_funding_rate(fee_rate: ccxt_core::types::FeeFundingRate) -> FundingRate {
    FundingRate {
        info: fee_rate.info,
        symbol: fee_rate.symbol,
        mark_price: decimal_to_f64(fee_rate.mark_price),
        index_price: decimal_to_f64(fee_rate.index_price),
        interest_rate: decimal_to_f64(fee_rate.interest_rate),
        estimated_settle_price: decimal_to_f64(fee_rate.estimated_settle_price),
        funding_rate: decimal_to_f64(fee_rate.funding_rate),
        funding_timestamp: fee_rate.funding_timestamp,
        funding_datetime: fee_rate.funding_datetime,
        previous_funding_rate: decimal_to_f64(fee_rate.previous_funding_rate),
        previous_funding_timestamp: fee_rate.previous_funding_timestamp,
        previous_funding_datetime: fee_rate.previous_funding_datetime,
        timestamp: fee_rate.timestamp,
        datetime: fee_rate.datetime,
    }
}

#[async_trait]
impl Margin for Binance {
    async fn fetch_positions_for(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        // Convert &[&str] to Option<Vec<String>> for the underlying method
        let symbols_option = if symbols.is_empty() {
            None
        } else {
            Some(symbols.iter().map(|s| s.to_string()).collect())
        };

        // Call the underlying fetch_positions method
        self.fetch_positions(symbols_option, None).await
    }

    async fn fetch_position(&self, symbol: &str) -> Result<Position> {
        // Call the underlying fetch_position method
        self.fetch_position(symbol, None).await
    }

    async fn set_leverage_with_params(&self, params: LeverageParams) -> Result<()> {
        // Binance set_leverage doesn't support margin mode parameter
        // Margin mode (cross/isolated) is set separately
        self.set_leverage(&params.symbol, params.leverage as i64, None)
            .await?;
        Ok(())
    }

    async fn get_leverage(&self, symbol: &str) -> Result<u32> {
        // Fetch leverage from the position endpoint
        let leverage_info = self.fetch_leverage(symbol, None).await?;
        // Use long_leverage as default (for one-way mode, both are the same)
        Ok(leverage_info.long_leverage.unwrap_or(1) as u32)
    }

    async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()> {
        // Convert MarginMode to Binance's expected format
        let mode_str = match mode {
            MarginMode::Isolated => "ISOLATED",
            MarginMode::Cross => "CROSSED",
        };

        // Call the underlying set_margin_mode method
        self.set_margin_mode(symbol, mode_str, None).await?;
        Ok(())
    }

    async fn fetch_funding_rate(&self, symbol: &str) -> Result<FundingRate> {
        // Call the underlying fetch_funding_rate method
        let fee_rate = self.fetch_funding_rate(symbol, None).await?;

        // Convert FeeFundingRate (from trading/fee.rs) to FundingRate (from market/funding_rate.rs)
        Ok(convert_funding_rate(fee_rate))
    }

    async fn fetch_funding_rates(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
        // Convert &[&str] to Option<&[String]> for the underlying method
        let symbols_option = if symbols.is_empty() {
            None
        } else {
            Some(symbols.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };

        // Call the underlying fetch_funding_rates method
        let rates_map = self
            .fetch_funding_rates(symbols_option.as_deref(), None)
            .await?;

        // Convert BTreeMap<String, FeeFundingRate> to Vec<FundingRate>
        Ok(rates_map.into_values().map(convert_funding_rate).collect())
    }

    async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        // Call the underlying fetch_funding_rate_history method
        let history = self
            .fetch_funding_rate_history(symbol, since, limit, None)
            .await?;

        // Convert Vec<FeeFundingRateHistory> to Vec<FundingRateHistory>
        Ok(history
            .into_iter()
            .map(|h| FundingRateHistory {
                info: h.info,
                symbol: h.symbol,
                funding_rate: decimal_to_f64(h.funding_rate),
                timestamp: h.timestamp,
                datetime: h.datetime,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binance::BinanceOptions;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_binance_margin_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let options = BinanceOptions::default();
        let binance = Binance::new_with_options(ExchangeConfig::default(), options).unwrap();
        let margin: Box<dyn Margin> = Box::new(binance);

        // Verify the exchange ID
        assert_eq!(margin.id(), "binance");

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

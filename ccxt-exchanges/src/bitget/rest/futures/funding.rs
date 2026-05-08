//! Funding rate operations for Bitget futures/swap.

use super::super::super::{Bitget, core::symbol::BitgetSymbolConverter, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{FundingRate, FundingRateHistory},
};
use std::collections::HashMap;
use tracing::warn;

impl Bitget {
    /// Fetch current funding rate for a symbol.
    ///
    /// Uses Bitget GET `/api/v3/market/current-fund-rate` (public endpoint).
    pub async fn fetch_funding_rate_impl(&self, symbol: &str) -> Result<FundingRate> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), exchange_id);

        let data = self
            .public_request("GET", "/api/v3/market/current-fund-rate", Some(&params))
            .await?;

        let rates_array = data["data"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format("data", "Expected data array"))
        })?;

        if rates_array.is_empty() {
            return Err(Error::from(ParseError::missing_field_owned(format!(
                "No funding rate found for symbol: {}",
                symbol
            ))));
        }

        parser::parse_funding_rate(&rates_array[0], symbol)
    }

    /// Fetch funding rates for multiple symbols.
    pub async fn fetch_funding_rates_impl(&self, symbols: &[&str]) -> Result<Vec<FundingRate>> {
        let mut rates = Vec::new();
        for symbol in symbols {
            match self.fetch_funding_rate_impl(symbol).await {
                Ok(rate) => rates.push(rate),
                Err(e) => {
                    warn!(error = %e, symbol = %symbol, "Failed to fetch Bitget funding rate");
                }
            }
        }
        Ok(rates)
    }

    /// Fetch funding rate history for a symbol.
    ///
    /// Uses Bitget GET `/api/v3/market/history-fund-rate` (public endpoint).
    pub async fn fetch_funding_rate_history_impl(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let category = BitgetSymbolConverter::product_type_from_symbol(symbol);

        let mut params = HashMap::new();
        params.insert("category".to_string(), category.to_string());
        params.insert("symbol".to_string(), exchange_id);

        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }

        // V3 uses cursor for pagination, startTime is not supported
        // Default cursor is 1 (first page)
        params.insert("cursor".to_string(), "1".to_string());

        let data = self
            .public_request("GET", "/api/v3/market/history-fund-rate", Some(&params))
            .await?;

        // V3 response structure: data.resultList[]
        let history_array = data["data"]["resultList"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data.resultList",
                "Expected data.resultList array",
            ))
        })?;

        let mut history = Vec::new();
        for item in history_array {
            match parser::parse_funding_rate_history(item, symbol) {
                Ok(record) => history.push(record),
                Err(e) => {
                    warn!(error = %e, "Failed to parse Bitget funding rate history");
                }
            }
        }

        Ok(history)
    }
}

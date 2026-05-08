//! WebSocket Ticker data parser for HyperLiquid.
//!
//! Parses allMids channel messages.

use ccxt_core::{
    error::{Error, Result},
    types::{Symbol, Ticker, financial::Price},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use crate::hyperliquid::core::symbol::HyperliquidSymbolConverter;

/// Parse ticker from WebSocket allMids message.
///
/// # Arguments
///
/// * `data` - The `data` field from WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
///
/// # Note
///
/// allMids contains mid prices for all coins. This function returns
/// the first coin's ticker for simplicity. For multi-coin parsing,
/// use `parse_all_mids_map`.
pub fn parse_all_mids(data: &Value) -> Result<Ticker> {
    // allMids 包含所有币种的中间价
    // mids 是一个 Map<coin, price>
    let mids = data
        .get("mids")
        .ok_or_else(|| Error::invalid_request("Missing mids in allMids message"))?;

    // 取第一个币种作为示例
    if let Some((coin, price)) = mids.as_object().and_then(|obj| obj.iter().next()) {
        let symbol = Symbol::new_unchecked(
            HyperliquidSymbolConverter::exchange_to_unified_inferred(coin),
        );
        let last = Decimal::from_str(price.as_str().unwrap_or("0"))
            .ok()
            .map(Price::new);

        return Ok(Ticker {
            symbol,
            timestamp: 0,
            datetime: None,
            high: None,
            low: None,
            bid: last,
            ask: last,
            bid_volume: None,
            ask_volume: None,
            vwap: None,
            open: None,
            close: last,
            last,
            previous_close: None,
            change: None,
            percentage: None,
            average: None,
            base_volume: None,
            quote_volume: None,
            funding_rate: None,
            open_interest: None,
            index_price: None,
            mark_price: None,
            info: HashMap::new(),
        });
    }

    Err(Error::invalid_request("Empty mids in allMids message"))
}

/// Parse all tickers from allMids message.
///
/// # Arguments
///
/// * `data` - The `data` field from WebSocket message
///
/// # Returns
///
/// Returns a map of symbol to ticker.
pub fn parse_all_mids_map(data: &Value) -> Result<Vec<Ticker>> {
    let mids = data
        .get("mids")
        .ok_or_else(|| Error::invalid_request("Missing mids in allMids message"))?;

    let tickers = mids
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(coin, price)| {
                    let symbol = Symbol::new_unchecked(
                        HyperliquidSymbolConverter::exchange_to_unified_inferred(coin),
                    );
                    let last = Decimal::from_str(price.as_str().unwrap_or("0"))
                        .ok()
                        .map(Price::new);

                    Some(Ticker {
                        symbol,
                        timestamp: 0,
                        datetime: None,
                        high: None,
                        low: None,
                        bid: last,
                        ask: last,
                        bid_volume: None,
                        ask_volume: None,
                        vwap: None,
                        open: None,
                        close: last,
                        last,
                        previous_close: None,
                        change: None,
                        percentage: None,
                        average: None,
                        base_volume: None,
                        quote_volume: None,
                        funding_rate: None,
                        open_interest: None,
                        index_price: None,
                        mark_price: None,
                        info: HashMap::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(tickers)
}

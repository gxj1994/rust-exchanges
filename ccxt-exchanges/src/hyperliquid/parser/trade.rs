//! Trade data parser for HyperLiquid.

use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{
        OrderSide, Symbol, Trade,
        financial::{Amount, Cost, Price},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;

use super::timestamp_to_datetime;

/// Parse trade data from HyperLiquid fill response.
///
/// # Arguments
///
/// * `data` - HyperLiquid trade data JSON object
/// * `market` - Optional market info for symbol resolution
pub fn parse_trade(data: &Value, market: Option<&ccxt_core::types::Market>) -> Result<Trade> {
    let symbol: Symbol = market.map_or_else(
        || Symbol::new_unchecked(data["coin"].as_str().unwrap_or("")),
        |m| m.symbol.clone(),
    );

    let timestamp = parse_timestamp(data, "time").unwrap_or(0);

    let side = match data["side"].as_str() {
        Some("A" | "sell" | "Sell") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let price = parse_decimal(data, "px").unwrap_or(Decimal::ZERO);
    let amount = parse_decimal(data, "sz").unwrap_or(Decimal::ZERO);
    let cost = price * amount;

    Ok(Trade {
        id: data["tid"]
            .as_str()
            .or(data["hash"].as_str())
            .map(ToString::to_string),
        order: data["oid"].as_str().map(ToString::to_string),
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        symbol,
        trade_type: None,
        side,
        taker_or_maker: None,
        price: Price::new(price),
        amount: Amount::new(amount),
        cost: Some(Cost::new(cost)),
        fee: None,
        info: value_to_hashmap(data),
    })
}

/// Parse a single trade from a userFills REST API response.
///
/// # Arguments
///
/// * `fill_data` - A single fill object from the userFills response
/// * `symbol_filter` - Optional symbol filter (CCXT standard format)
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
pub fn parse_trade_from_fill(fill_data: &Value, symbol_filter: &str) -> Result<Trade> {
    use crate::hyperliquid::core::symbol::HyperliquidSymbolConverter;

    let coin = fill_data
        .get("coin")
        .and_then(|c| c.as_str())
        .unwrap_or_default();

    // Use symbol filter if provided, otherwise construct from coin
    let symbol = if !symbol_filter.is_empty() {
        Symbol::new_unchecked(symbol_filter)
    } else {
        Symbol::new_unchecked(HyperliquidSymbolConverter::exchange_to_unified_inferred(
            coin,
        ))
    };

    let price = parse_decimal(fill_data, "px").unwrap_or(Decimal::ZERO);
    let amount = parse_decimal(fill_data, "sz").unwrap_or(Decimal::ZERO);
    let cost = price * amount;

    let side = match fill_data["side"].as_str() {
        Some("B" | "buy" | "Buy") => OrderSide::Buy,
        _ => OrderSide::Sell,
    };

    let timestamp = parse_timestamp(fill_data, "time").unwrap_or(0);

    Ok(Trade {
        id: fill_data
            .get("oid")
            .and_then(|o| o.as_u64())
            .map(|i| i.to_string()),
        order: fill_data
            .get("oid")
            .and_then(|o| o.as_u64())
            .map(|i| i.to_string()),
        symbol,
        trade_type: None,
        side,
        taker_or_maker: None,
        price: Price::new(price),
        amount: Amount::new(amount),
        cost: Some(Cost::new(cost)),
        fee: None,
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        info: value_to_hashmap(fill_data),
    })
}

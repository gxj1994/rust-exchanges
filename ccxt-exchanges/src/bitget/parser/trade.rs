//! Trade data parser for Bitget.

use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{
        Market, OrderSide, Symbol, Trade,
        financial::{Amount, Cost, Price},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;

use super::timestamp_to_datetime;

/// Parse trade data from Bitget trade response.
///
/// # Arguments
///
/// * `data` - Bitget trade data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
pub fn parse_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .map(|s| Symbol::new_unchecked(s))
            .ok_or_else(|| {
                ccxt_core::Error::from(ccxt_core::ParseError::missing_field("symbol"))
                    .context("Failed to parse trade: missing symbol identifier")
            })?
    };

    let id = data["execId"]
        .as_str()
        .or_else(|| data["tradeId"].as_str())
        .or_else(|| data["id"].as_str())
        .map(ToString::to_string);

    let timestamp = parse_timestamp(data, "createdTime")
        .or_else(|| parse_timestamp(data, "ts"))
        .or_else(|| parse_timestamp(data, "timestamp"))
        .unwrap_or(0);

    // Bitget uses "side" field with "buy" or "sell" values
    let side = match data["side"].as_str() {
        Some("sell" | "Sell" | "SELL") => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to buy if not specified
    };

    let price = parse_decimal(data, "execPrice")
        .or_else(|| parse_decimal(data, "price"))
        .or_else(|| parse_decimal(data, "fillPrice"));
    let amount = parse_decimal(data, "execQty")
        .or_else(|| parse_decimal(data, "size"))
        .or_else(|| parse_decimal(data, "amount"))
        .or_else(|| parse_decimal(data, "fillSize"));

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    Ok(Trade {
        id,
        order: data["orderId"].as_str().map(ToString::to_string),
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        symbol,
        trade_type: None,
        side,
        taker_or_maker: None,
        price: Price::new(price.unwrap_or(Decimal::ZERO)),
        amount: Amount::new(amount.unwrap_or(Decimal::ZERO)),
        cost: cost.map(Cost::new),
        fee: None,
        info: value_to_hashmap(data),
    })
}

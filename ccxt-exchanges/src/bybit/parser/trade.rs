//! Trade parser for Bybit.

use crate::common::parser_helpers::ParseHelper;
use ccxt_core::{
    Result,
    parser_utils::{parse_timestamp, value_to_hashmap},
    types::financial::{Amount, Cost, Price},
    types::{Market, OrderSide, Symbol, Trade},
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse trade data from Bybit trade response.
///
/// # Arguments
///
/// * `data` - Bybit trade data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
pub fn parse_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        // Check both "symbol" (REST API) and "s" (WebSocket) fields
        data["symbol"]
            .as_str()
            .or_else(|| data["s"].as_str())
            .map(|s| Symbol::new_unchecked(s))
            .ok_or_else(|| {
                ccxt_core::Error::from(ccxt_core::ParseError::missing_field("symbol/s"))
                    .context("Failed to parse: missing symbol identifier")
            })?
    };

    let id = data["execId"]
        .as_str()
        .or_else(|| data["id"].as_str())
        .map(ToString::to_string);

    let timestamp = parse_timestamp(data, "time")
        .or_else(|| parse_timestamp(data, "T"))
        .unwrap_or(0);

    // Bybit uses "side" field with "Buy" or "Sell" values
    let side = match data["side"].as_str() {
        Some("Sell" | "sell" | "SELL") => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to buy if not specified
    };

    let price = ParseHelper::decimal_any(data, &["price", "execPrice"]);
    let amount = ParseHelper::decimal_any(data, &["size", "execQty", "qty"]);

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    Ok(Trade {
        id,
        order: data["orderId"].as_str().map(ToString::to_string),
        timestamp,
        datetime: ccxt_core::parser_utils::timestamp_to_datetime(timestamp),
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_trade() {
        let data = json!({
            "execId": "123456",
            "symbol": "BTCUSDT",
            "side": "Buy",
            "price": "50000.00",
            "size": "0.5",
            "time": "1700000000000"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.id, Some("123456".to_string()));
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.price, Price::new(dec!(50000.00)));
        assert_eq!(trade.amount, Amount::new(dec!(0.5)));
    }
}

//! Trade parser for OKX.

use crate::common::parser_helpers::ParseHelper;
use ccxt_core::{
    Result,
    parser_utils::{parse_timestamp, value_to_hashmap},
    types::financial::{Amount, Cost, Price},
    types::{Market, OrderSide, Symbol, Trade},
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse trade data from OKX trade response.
///
/// # Arguments
///
/// * `data` - OKX trade data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
pub fn parse_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["instId"].as_str().map_or_else(
            || Symbol::new_unchecked(""),
            |s| Symbol::new_unchecked(s.replace('-', "/")),
        )
    };

    let id = data["tradeId"].as_str().map(ToString::to_string);

    let timestamp = parse_timestamp(data, "ts").unwrap_or(0);

    // OKX uses "side" field with "buy" or "sell" values
    let side = match data["side"].as_str() {
        Some("sell" | "Sell" | "SELL") => OrderSide::Sell,
        _ => OrderSide::Buy, // Default to buy if not specified
    };

    let price = ParseHelper::decimal_any(data, &["px", "fillPx"]);
    let amount = ParseHelper::decimal_any(data, &["sz", "fillSz"]);

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    Ok(Trade {
        id,
        order: data["ordId"].as_str().map(ToString::to_string),
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
            "tradeId": "123456",
            "instId": "BTC-USDT",
            "side": "buy",
            "px": "50000.00",
            "sz": "0.5",
            "ts": "1700000000000"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.id, Some("123456".to_string()));
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.price, Price::new(dec!(50000.00)));
        assert_eq!(trade.amount, Amount::new(dec!(0.5)));
    }
}

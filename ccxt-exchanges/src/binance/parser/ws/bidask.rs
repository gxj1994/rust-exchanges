//! WebSocket BidAsk data parser for Binance.
//!
//! Parses bookTicker events.

use ccxt_core::{
    error::{Error, ParseError, Result},
    types::BidAsk,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

use super::to_unified_symbol;

/// Parse bids/asks from WebSocket bookTicker message.
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`BidAsk`] structure.
///
/// # Binance bookTicker WebSocket Format
///
/// Both Spot and Swap use the same field names:
/// - `b`: bid price
/// - `B`: bid quantity
/// - `a`: ask price
/// - `A`: ask quantity
///
/// Example:
/// ```json
/// {
///   "e": "bookTicker",
///   "u": 400900217,
///   "E": 1568014460893,
///   "T": 1568014460891,
///   "s": "BNBUSDT",
///   "b": "25.35190000",
///   "B": "31.21000000",
///   "a": "25.36520000",
///   "A": "40.66000000"
/// }
/// ```
pub fn parse_bids_asks(msg: &Value) -> Result<BidAsk> {
    let symbol = to_unified_symbol(msg);

    let timestamp = msg.get("E").and_then(|t| t.as_i64()).unwrap_or(0);

    // Parse bid price - required field
    let bid_price = msg
        .get("b")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("bid_price (b)")))?;

    // Parse bid quantity - required field
    let bid_quantity = msg
        .get("B")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("bid_quantity (B)")))?;

    // Parse ask price - required field
    let ask_price = msg
        .get("a")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("ask_price (a)")))?;

    // Parse ask quantity - required field
    let ask_quantity = msg
        .get("A")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .ok_or_else(|| Error::from(ParseError::missing_field("ask_quantity (A)")))?;

    Ok(BidAsk::new(
        symbol,
        bid_price,
        bid_quantity,
        ask_price,
        ask_quantity,
        timestamp,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_bookticker_spot() {
        // 现货 bookTicker 格式
        let msg = json!({
            "e": "bookTicker",
            "u": 400900217,
            "E": 1568014460893i64,
            "T": 1568014460891i64,
            "s": "BTCUSDT",
            "b": "50000.00",
            "B": "1.5",
            "a": "50010.00",
            "A": "2.0"
        });

        let bidask = parse_bids_asks(&msg).unwrap();
        assert_eq!(bidask.symbol, "BTC/USDT");
        assert_eq!(bidask.bid_price, Decimal::new(50000, 0));
        assert_eq!(bidask.bid_quantity, Decimal::new(15, 1));
        assert_eq!(bidask.ask_price, Decimal::new(50010, 0));
        assert_eq!(bidask.ask_quantity, Decimal::new(2, 0));
        assert_eq!(bidask.timestamp, 1568014460893);
    }

    #[test]
    fn test_parse_bookticker_swap() {
        // 合约 bookTicker 格式（字段相同）
        let msg = json!({
            "e": "bookTicker",
            "u": 500900218,
            "E": 1568014460894i64,
            "T": 1568014460892i64,
            "s": "BTCUSDT",
            "b": "50000.50",
            "B": "10.5",
            "a": "50010.50",
            "A": "20.5"
        });

        let bidask = parse_bids_asks(&msg).unwrap();
        assert_eq!(bidask.symbol, "BTC/USDT");
        assert_eq!(bidask.bid_price, Decimal::new(5000050, 2));
        assert_eq!(bidask.bid_quantity, Decimal::new(105, 1));
        assert_eq!(bidask.ask_price, Decimal::new(5001050, 2));
        assert_eq!(bidask.ask_quantity, Decimal::new(205, 1));
    }

    #[test]
    fn test_parse_bookticker_missing_bid_price() {
        // 缺少 bid price 应该报错
        let msg = json!({
            "e": "bookTicker",
            "u": 400900217,
            "E": 1568014460893i64,
            "s": "BTCUSDT",
            "B": "1.5",
            "a": "50010.00",
            "A": "2.0"
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("bid_price"),
            "Error should mention bid_price field"
        );
    }

    #[test]
    fn test_parse_bookticker_missing_ask_price() {
        // 缺少 ask price 应该报错
        let msg = json!({
            "e": "bookTicker",
            "u": 400900217,
            "E": 1568014460893i64,
            "s": "BTCUSDT",
            "b": "50000.00",
            "B": "1.5",
            "A": "2.0"
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("ask_price"),
            "Error should mention ask_price field"
        );
    }

    #[test]
    fn test_parse_bookticker_zero_values() {
        // 合法的零值（市场深度为空的情况）
        let msg = json!({
            "e": "bookTicker",
            "u": 400900217,
            "E": 1568014460893i64,
            "s": "BTCUSDT",
            "b": "0",
            "B": "0",
            "a": "0",
            "A": "0"
        });

        let bidask = parse_bids_asks(&msg).unwrap();
        assert_eq!(bidask.bid_price, Decimal::ZERO);
        assert_eq!(bidask.bid_quantity, Decimal::ZERO);
        assert_eq!(bidask.ask_price, Decimal::ZERO);
        assert_eq!(bidask.ask_quantity, Decimal::ZERO);
    }

    #[test]
    fn test_parse_bookticker_spread_calculation() {
        // 测试价差计算
        let msg = json!({
            "e": "bookTicker",
            "u": 400900217,
            "E": 1568014460893i64,
            "s": "BTCUSDT",
            "b": "50000.00",
            "B": "1.5",
            "a": "50010.00",
            "A": "2.0"
        });

        let bidask = parse_bids_asks(&msg).unwrap();
        let spread = bidask.ask_price - bidask.bid_price;
        assert_eq!(spread, Decimal::new(10, 0));
    }
}

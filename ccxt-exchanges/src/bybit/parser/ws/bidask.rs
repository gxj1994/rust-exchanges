//! WebSocket BidsAsks (orderbook.1) data parser for Bybit.
//!
//! Bybit 的 orderbook.1 就是最优买卖价（bids_asks）。
//! 根据官方文档：
//! - 1档数据只推送 snapshot 消息
//! - 3秒内无变化会重推（u值相同）
//! - 推送频率：10ms

use ccxt_core::{
    error::{Error, ParseError, Result},
    types::BidAsk,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

use super::{extract_data, to_unified_symbol};

/// Parse bids_asks (orderbook.1) from WebSocket message.
///
/// # Message Format
/// ```json
/// {
///     "topic": "orderbook.1.BTCUSDT",
///     "type": "snapshot",
///     "ts": 1672304484978,
///     "data": {
///         "s": "BTCUSDT",
///         "b": [["16493.50", "0.006"]],
///         "a": [["16611.00", "0.029"]],
///         "u": 18521288,
///         "seq": 7961638724
///     },
///     "cts": 1672304484976
/// }
/// ```
pub fn parse_bids_asks(msg: &Value) -> Result<BidAsk> {
    let data = extract_data(msg)?;

    // Extract symbol from topic: orderbook.1.BTCUSDT -> BTCUSDT
    let symbol_str = msg
        .get("topic")
        .and_then(|t| t.as_str())
        .and_then(|t| {
            let parts: Vec<&str> = t.split('.').collect();
            parts.get(2).copied()
        })
        .ok_or_else(|| Error::from(ParseError::missing_field("topic")))?;

    let symbol = to_unified_symbol(symbol_str, Some(msg));

    // Parse timestamp from outer level (not in data)
    // 注意：文档说现货和合约1档的ts在type之前
    let timestamp = msg.get("ts").and_then(|t| t.as_u64()).unwrap_or(0) as i64;

    // Parse best bid: b[0][0] = price, b[0][1] = amount
    let (bid_price, bid_quantity) = parse_best_level(data, "b")?;

    // Parse best ask: a[0][0] = price, a[0][1] = amount
    let (ask_price, ask_quantity) = parse_best_level(data, "a")?;

    let ba = BidAsk::new(
        symbol,
        bid_price,
        bid_quantity,
        ask_price,
        ask_quantity,
        timestamp,
    );

    Ok(ba)
}

/// Parse the best level (price and quantity) from bids or asks array.
fn parse_best_level(data: &Value, field: &str) -> Result<(Decimal, Decimal)> {
    let missing_array_err = Error::invalid_request(format!("Missing {} array", field));
    let array = data
        .get(field)
        .and_then(|v| v.as_array())
        .ok_or(missing_array_err)?;

    let empty_array_err = Error::invalid_request(format!("Empty {} array", field));
    let first_level = array.first().ok_or(empty_array_err)?;

    let missing_price_err = Error::invalid_request(format!("Missing {}[0][0]", field));
    let price_str = first_level
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or(missing_price_err)?;

    let missing_qty_err = Error::invalid_request(format!("Missing {}[0][1]", field));
    let quantity_str = first_level
        .get(1)
        .and_then(|v| v.as_str())
        .ok_or(missing_qty_err)?;

    let price = Decimal::from_str(price_str)
        .map_err(|e| Error::invalid_request(format!("Invalid price in {}: {}", field, e)))?;

    let quantity = Decimal::from_str(quantity_str)
        .map_err(|e| Error::invalid_request(format!("Invalid quantity in {}: {}", field, e)))?;

    Ok((price, quantity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_orderbook_1_spot() {
        let msg = json!({
            "topic": "orderbook.1.BTCUSDT",
            "type": "snapshot",
            "ts": 1672304484978u64,
            "data": {
                "s": "BTCUSDT",
                "b": [["76863.00", "1.46312000"]],
                "a": [["76863.01", "5.30942000"]],
                "u": 18521288u64,
                "seq": 7961638724u64
            },
            "cts": 1672304484976u64
        });

        let ba = parse_bids_asks(&msg).unwrap();
        assert_eq!(ba.symbol, "BTC/USDT");
        assert_eq!(ba.bid_price, Decimal::from_str("76863.00").unwrap());
        assert_eq!(ba.bid_quantity, Decimal::from_str("1.46312000").unwrap());
        assert_eq!(ba.ask_price, Decimal::from_str("76863.01").unwrap());
        assert_eq!(ba.ask_quantity, Decimal::from_str("5.30942000").unwrap());
        assert_eq!(ba.timestamp, 1672304484978);

        // Verify spread
        let spread = ba.ask_price - ba.bid_price;
        assert_eq!(spread, Decimal::from_str("0.01").unwrap());
    }

    #[test]
    fn test_parse_orderbook_1_swap() {
        let msg = json!({
            "topic": "orderbook.1.BTCUSDT",
            "type": "snapshot",
            "ts": 1700000000000u64,
            "data": {
                "s": "BTCUSDT",
                "b": [["76820.00", "1.565"]],
                "a": [["76820.10", "10.090"]],
                "u": 177400507u64,
                "seq": 66544703342u64
            },
            "cts": 1700000000000u64
        });

        let ba = parse_bids_asks(&msg).unwrap();
        assert_eq!(ba.symbol, "BTC/USDT");
        assert!(ba.bid_price > Decimal::ZERO);
        assert!(ba.ask_price > Decimal::ZERO);
        assert!(
            ba.ask_price > ba.bid_price,
            "Ask should be greater than bid"
        );
    }

    #[test]
    fn test_parse_orderbook_1_missing_topic() {
        let msg = json!({
            "data": {
                "s": "BTCUSDT",
                "b": [["50000.00", "1.0"]],
                "a": [["50001.00", "2.0"]]
            }
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err(), "Should fail when topic is missing");
    }

    #[test]
    fn test_parse_orderbook_1_empty_bids() {
        let msg = json!({
            "topic": "orderbook.1.BTCUSDT",
            "type": "snapshot",
            "ts": 1700000000000u64,
            "data": {
                "s": "BTCUSDT",
                "b": [],
                "a": [["50001.00", "2.0"]]
            }
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err(), "Should fail when bids array is empty");
    }

    #[test]
    fn test_parse_orderbook_1_invalid_price() {
        let msg = json!({
            "topic": "orderbook.1.BTCUSDT",
            "type": "snapshot",
            "ts": 1700000000000u64,
            "data": {
                "s": "BTCUSDT",
                "b": [["invalid_price", "1.0"]],
                "a": [["50001.00", "2.0"]]
            }
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err(), "Should fail when price is invalid");
    }

    #[test]
    fn test_parse_orderbook_1_zero_spread() {
        // 理论上价差不应为0，但要验证解析器能正确处理
        let msg = json!({
            "topic": "orderbook.1.BTCUSDT",
            "type": "snapshot",
            "ts": 1700000000000u64,
            "data": {
                "s": "BTCUSDT",
                "b": [["50000.00", "1.0"]],
                "a": [["50000.00", "2.0"]]
            }
        });

        let ba = parse_bids_asks(&msg).unwrap();
        let spread = ba.ask_price - ba.bid_price;
        assert_eq!(
            spread,
            Decimal::ZERO,
            "Spread should be zero in this edge case"
        );
    }
}

//! WebSocket BidsAsks (book_ticker) data parser for Gate.io.
//!
//! Gate.io 的 spot.book_ticker 和 futures.book_ticker 推送最优买卖价。
//!
//! ## Spot book_ticker Format
//! ```json
//! {
//!     "time": 1606292218,
//!     "channel": "spot.book_ticker",
//!     "event": "update",
//!     "result": {
//!         "t": 1606292218000,
//!         "s": "BTC_USDT",
//!         "b": "50000.0",
//!         "B": "1.5",
//!         "a": "50010.0",
//!         "A": "2.0"
//!     }
//! }
//! ```
//!
//! ## Futures book_ticker Format
//! ```json
//! {
//!     "time": 1606292218,
//!     "channel": "futures.book_ticker",
//!     "event": "update",
//!     "result": {
//!         "t": 1606292218000,
//!         "s": "BTC_USDT",
//!         "b": "50000.0",
//!         "B": "10",
//!         "a": "50010.0",
//!         "A": "20"
//!     }
//! }
//! ```

use ccxt_core::{
    error::{Error, Result},
    types::BidAsk,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;

/// Parse bids_asks (book_ticker) from Gate.io WebSocket message.
///
/// # Arguments
///
/// * `msg` - WebSocket message (the outer envelope)
///
/// # Returns
///
/// Parsed BidAsk structure
pub fn parse_bids_asks(msg: &Value) -> Result<BidAsk> {
    // Extract result from Gate message format
    let result = msg
        .get("result")
        .ok_or_else(|| Error::invalid_request("Missing result in book_ticker message"))?;

    // Parse timestamp (milliseconds)
    let timestamp = result.get("t").and_then(|t| t.as_i64()).unwrap_or(0);

    // Parse symbol
    let gate_symbol = result
        .get("s")
        .and_then(|s| s.as_str())
        .ok_or_else(|| Error::invalid_request("Missing symbol (s) in book_ticker"))?;

    // Convert Gate symbol format to unified: BTC_USDT -> BTC/USDT, BTC_USDT:USDT -> BTC/USDT:USDT
    let symbol = gate_symbol.replace('_', "/");

    // Parse bid price and quantity
    let bid_price_str = result
        .get("b")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::invalid_request("Missing bid price (b) in book_ticker"))?;

    // Bid quantity can be string (spot) or number (futures)
    let bid_quantity = if let Some(s) = result.get("B").and_then(|v| v.as_str()) {
        Decimal::from_str(s)
            .map_err(|e| Error::invalid_request(format!("Invalid bid quantity: {}", e)))?
    } else if let Some(n) = result.get("B").and_then(|v| v.as_i64()) {
        Decimal::from_i64(n)
            .ok_or_else(|| Error::invalid_request("Invalid bid quantity (B) in book_ticker"))?
    } else {
        return Err(Error::invalid_request(
            "Missing bid quantity (B) in book_ticker",
        ));
    };

    // Parse ask price and quantity
    let ask_price_str = result
        .get("a")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::invalid_request("Missing ask price (a) in book_ticker"))?;

    // Ask quantity can be string (spot) or number (futures)
    let ask_quantity = if let Some(s) = result.get("A").and_then(|v| v.as_str()) {
        Decimal::from_str(s)
            .map_err(|e| Error::invalid_request(format!("Invalid ask quantity: {}", e)))?
    } else if let Some(n) = result.get("A").and_then(|v| v.as_i64()) {
        Decimal::from_i64(n)
            .ok_or_else(|| Error::invalid_request("Invalid ask quantity (A) in book_ticker"))?
    } else {
        return Err(Error::invalid_request(
            "Missing ask quantity (A) in book_ticker",
        ));
    };

    // Convert to Decimal
    let bid_price = Decimal::from_str(bid_price_str)
        .map_err(|e| Error::invalid_request(format!("Invalid bid price: {}", e)))?;

    let ask_price = Decimal::from_str(ask_price_str)
        .map_err(|e| Error::invalid_request(format!("Invalid ask price: {}", e)))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_spot_book_ticker() {
        let msg = json!({
            "time": 1606292218,
            "channel": "spot.book_ticker",
            "event": "update",
            "result": {
                "t": 1606292218000i64,
                "s": "BTC_USDT",
                "b": "76863.00",
                "B": "1.46312000",
                "a": "76863.01",
                "A": "5.30942000"
            }
        });

        let ba = parse_bids_asks(&msg).unwrap();
        assert_eq!(ba.symbol, "BTC/USDT");
        assert_eq!(ba.bid_price, Decimal::from_str("76863.00").unwrap());
        assert_eq!(ba.bid_quantity, Decimal::from_str("1.46312000").unwrap());
        assert_eq!(ba.ask_price, Decimal::from_str("76863.01").unwrap());
        assert_eq!(ba.ask_quantity, Decimal::from_str("5.30942000").unwrap());
        assert_eq!(ba.timestamp, 1606292218000);

        // Verify spread
        let spread = ba.ask_price - ba.bid_price;
        assert_eq!(spread, Decimal::from_str("0.01").unwrap());
    }

    #[test]
    fn test_parse_futures_book_ticker() {
        // Futures book_ticker returns B and A as numbers (integers), not strings
        let msg = json!({
            "time": 1606292218,
            "channel": "futures.book_ticker",
            "event": "update",
            "result": {
                "t": 1700000000000i64,
                "s": "BTC_USDT",
                "b": "76820.00",
                "B": 10,
                "a": "76820.10",
                "A": 20
            }
        });

        let ba = parse_bids_asks(&msg).unwrap();
        assert_eq!(ba.symbol, "BTC/USDT");
        assert_eq!(ba.bid_price, Decimal::from_str("76820.00").unwrap());
        assert_eq!(ba.bid_quantity, Decimal::from_i64(10).unwrap());
        assert_eq!(ba.ask_price, Decimal::from_str("76820.10").unwrap());
        assert_eq!(ba.ask_quantity, Decimal::from_i64(20).unwrap());
        assert!(
            ba.ask_price > ba.bid_price,
            "Ask should be greater than bid"
        );
    }

    #[test]
    fn test_parse_missing_result() {
        let msg = json!({
            "time": 1606292218,
            "channel": "spot.book_ticker",
            "event": "update"
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err(), "Should fail when result is missing");
    }

    #[test]
    fn test_parse_missing_bid_price() {
        let msg = json!({
            "time": 1606292218,
            "channel": "spot.book_ticker",
            "event": "update",
            "result": {
                "t": 1700000000000i64,
                "s": "BTC_USDT",
                "B": "1.0",
                "a": "50001.00",
                "A": "2.0"
            }
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err(), "Should fail when bid price is missing");
    }

    #[test]
    fn test_parse_missing_ask_price() {
        let msg = json!({
            "time": 1606292218,
            "channel": "spot.book_ticker",
            "event": "update",
            "result": {
                "t": 1700000000000i64,
                "s": "BTC_USDT",
                "b": "50000.00",
                "B": "1.0",
                "A": "2.0"
            }
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err(), "Should fail when ask price is missing");
    }

    #[test]
    fn test_parse_zero_spread() {
        // Edge case: spread should theoretically not be 0
        let msg = json!({
            "time": 1606292218,
            "channel": "spot.book_ticker",
            "event": "update",
            "result": {
                "t": 1700000000000i64,
                "s": "BTC_USDT",
                "b": "50000.00",
                "B": "1.0",
                "a": "50000.00",
                "A": "2.0"
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

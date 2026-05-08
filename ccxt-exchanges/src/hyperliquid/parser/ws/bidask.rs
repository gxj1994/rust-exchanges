//! WebSocket BidsAsks (bbo) data parser for HyperLiquid.
//!
//! HyperLiquid 的 bbo (Best Bid/Offer) 推送最优买卖价。
//!
//! ## BBO Format
//! ```json
//! {
//!     "channel": "bbo",
//!     "data": {
//!         "coin": "BTC",
//!         "time": 1700000000000,
//!         "bids": [{"px": "50000.0", "sz": "1.5"}],
//!         "asks": [{"px": "50001.0", "sz": "2.0"}]
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

/// Parse bids_asks (bbo) from HyperLiquid WebSocket message.
///
/// # Arguments
///
/// * `msg` - WebSocket message (the outer envelope)
///
/// # Returns
///
/// Parsed BidAsk structure
pub fn parse_bids_asks(msg: &Value) -> Result<BidAsk> {
    // Extract data from HyperLiquid message format
    let data = msg
        .get("data")
        .ok_or_else(|| Error::invalid_request("Missing data in bbo message"))?;

    // Parse timestamp (milliseconds)
    let timestamp = data.get("time").and_then(|t| t.as_i64()).unwrap_or(0);

    // Parse coin symbol
    let coin = data
        .get("coin")
        .and_then(|s| s.as_str())
        .ok_or_else(|| Error::invalid_request("Missing coin in bbo"))?;

    // Convert to unified symbol format: BTC -> BTC/USDC:USDC
    let symbol = format!("{}/USDC:USDC", coin);

    // HyperLiquid bbo format: single "bbo" array containing bid and ask levels
    // First element is typically the best bid, second is the best ask
    let bbo_array = data
        .get("bbo")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::invalid_request("Missing bbo array in message"))?;

    if bbo_array.len() < 2 {
        return Err(Error::invalid_request(
            "bbo array must contain at least 2 elements (bid and ask)",
        ));
    }

    // Extract best bid (first element) and best ask (second element)
    let best_bid = &bbo_array[0];
    let best_ask = &bbo_array[1];

    // Parse bid price and quantity
    let bid_price_str = best_bid
        .get("px")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::invalid_request("Missing bid price (px) in bbo"))?;

    let bid_quantity = parse_decimal_from_value(
        best_bid
            .get("sz")
            .ok_or_else(|| Error::invalid_request("Missing bid quantity (sz) in bbo"))?,
    )
    .ok_or_else(|| Error::invalid_request("Invalid bid quantity in bbo"))?;

    // Parse ask price and quantity
    let ask_price_str = best_ask
        .get("px")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::invalid_request("Missing ask price (px) in bbo"))?;

    let ask_quantity = parse_decimal_from_value(
        best_ask
            .get("sz")
            .ok_or_else(|| Error::invalid_request("Missing ask quantity (sz) in bbo"))?,
    )
    .ok_or_else(|| Error::invalid_request("Invalid ask quantity in bbo"))?;

    // Convert price strings to Decimal
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

/// Helper to parse decimal from a JSON value.
///
/// Supports both number and string representations.
fn parse_decimal_from_value(v: &Value) -> Option<Decimal> {
    if let Some(s) = v.as_str() {
        Decimal::from_str(s).ok()
    } else if let Some(n) = v.as_i64() {
        Decimal::from_i64(n)
    } else if let Some(f) = v.as_f64() {
        Decimal::from_f64(f)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_bbo() {
        let msg = json!({
            "channel": "bbo",
            "data": {
                "coin": "BTC",
                "time": 1700000000000i64,
                "bbo": [
                    {"px": "76863.00", "sz": "1.46312000", "n": 24},
                    {"px": "76863.01", "sz": "5.30942000", "n": 21}
                ]
            }
        });

        let ba = parse_bids_asks(&msg).unwrap();
        assert_eq!(ba.symbol, "BTC/USDC:USDC");
        assert_eq!(ba.bid_price, Decimal::from_str("76863.00").unwrap());
        assert_eq!(ba.bid_quantity, Decimal::from_str("1.46312000").unwrap());
        assert_eq!(ba.ask_price, Decimal::from_str("76863.01").unwrap());
        assert_eq!(ba.ask_quantity, Decimal::from_str("5.30942000").unwrap());
        assert_eq!(ba.timestamp, 1700000000000);

        // Verify spread
        let spread = ba.ask_price - ba.bid_price;
        assert_eq!(spread, Decimal::from_str("0.01").unwrap());
    }

    #[test]
    fn test_parse_bbo_numeric_quantity() {
        // Test with numeric quantity (if HyperLiquid sends numbers instead of strings)
        let msg = json!({
            "channel": "bbo",
            "data": {
                "coin": "ETH",
                "time": 1700000000000i64,
                "bbo": [
                    {"px": "3000.00", "sz": 10, "n": 15},
                    {"px": "3000.10", "sz": 20, "n": 18}
                ]
            }
        });

        let ba = parse_bids_asks(&msg).unwrap();
        assert_eq!(ba.symbol, "ETH/USDC:USDC");
        assert_eq!(ba.bid_quantity, Decimal::from_i64(10).unwrap());
        assert_eq!(ba.ask_quantity, Decimal::from_i64(20).unwrap());
    }

    #[test]
    fn test_parse_missing_data() {
        let msg = json!({
            "channel": "bbo"
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err(), "Should fail when data is missing");
    }

    #[test]
    fn test_parse_missing_bids() {
        let msg = json!({
            "channel": "bbo",
            "data": {
                "coin": "BTC",
                "time": 1700000000000i64,
                "bbo": [
                    {"px": "50001.00", "sz": "2.0", "n": 18}
                ]
            }
        });

        let result = parse_bids_asks(&msg);
        assert!(
            result.is_err(),
            "Should fail when bbo has less than 2 elements"
        );
    }

    #[test]
    fn test_parse_missing_ask_price() {
        let msg = json!({
            "channel": "bbo",
            "data": {
                "coin": "BTC",
                "time": 1700000000000i64,
                "bbo": [
                    {"px": "50000.00", "sz": "1.0", "n": 24},
                    {"sz": "2.0", "n": 18}
                ]
            }
        });

        let result = parse_bids_asks(&msg);
        assert!(result.is_err(), "Should fail when ask price is missing");
    }

    #[test]
    fn test_parse_zero_spread() {
        // Edge case: spread should theoretically not be 0
        let msg = json!({
            "channel": "bbo",
            "data": {
                "coin": "BTC",
                "time": 1700000000000i64,
                "bbo": [
                    {"px": "50000.00", "sz": "1.0", "n": 24},
                    {"px": "50000.00", "sz": "2.0", "n": 18}
                ]
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

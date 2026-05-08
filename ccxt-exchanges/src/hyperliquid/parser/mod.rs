//! HyperLiquid data parser module.
//!
//! Converts HyperLiquid API response data into standardized CCXT format structures.
//!
//! ## Module Structure
//!
//! - `market` - Market data parsing
//! - `ticker` - Ticker data parsing
//! - `orderbook` - OrderBook data parsing
//! - `trade` - Trade data parsing
//! - `order` - Order data parsing
//! - `ohlcv` - OHLCV (candlestick) data parsing
//! - `balance` - Balance data parsing
//! - `ws` - WebSocket-specific data parsing

use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;

// Re-export all parser functions
pub use balance::parse_balance;
pub use market::{parse_market, parse_spot_market};
pub use ohlcv::parse_ohlcv;
pub use order::{parse_order, parse_order_status};
pub use orderbook::parse_orderbook;
pub use ticker::parse_ticker;
pub use trade::{parse_trade, parse_trade_from_fill};

// Re-export WebSocket parser functions
pub use ws::{
    parse_all_mids, parse_all_mids_map, parse_balance as parse_ws_balance,
    parse_bids_asks as parse_ws_bids_asks, parse_ohlcv as parse_ws_ohlcv,
    parse_order as parse_ws_order, parse_order_from_data, parse_order_update,
    parse_orderbook as parse_ws_orderbook, parse_trades, parse_user_fills,
};

// Re-export for backward compatibility
pub use ccxt_core::parser_utils::timestamp_to_datetime;

mod balance;
mod market;
mod ohlcv;
mod order;
mod orderbook;
mod ticker;
mod trade;
pub mod ws;

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to parse decimal from a JSON value directly.
///
/// Supports both number and string representations.
pub(crate) fn parse_decimal_from_value(v: &Value) -> Option<Decimal> {
    if let Some(num) = v.as_f64() {
        Decimal::from_f64(num)
    } else if let Some(s) = v.as_str() {
        Decimal::from_str(s).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_market() {
        let data = json!({
            "name": "BTC",
            "szDecimals": 4
        });

        let market = parse_market(&data, 0).unwrap();
        assert_eq!(
            market.symbol,
            ccxt_core::types::Symbol::new_unchecked("BTC/USDC:USDC")
        );
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDC");
        assert!(market.active);
    }

    #[test]
    fn test_parse_ticker() {
        let ticker = parse_ticker("BTC/USDC:USDC", dec!(50000), None).unwrap();
        assert_eq!(
            ticker.symbol,
            ccxt_core::types::Symbol::new_unchecked("BTC/USDC:USDC")
        );
        assert_eq!(
            ticker.last,
            Some(ccxt_core::types::financial::Price::new(dec!(50000)))
        );
    }

    #[test]
    fn test_parse_orderbook() {
        let data = json!({
            "levels": [
                [{"px": "50000", "sz": "1.5"}, {"px": "49999", "sz": "2.0"}],
                [{"px": "50001", "sz": "1.0"}, {"px": "50002", "sz": "3.0"}]
            ]
        });

        let orderbook = parse_orderbook(&data, "BTC/USDC:USDC".to_string()).unwrap();
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);
        // Bids should be sorted descending
        assert!(orderbook.bids[0].price >= orderbook.bids[1].price);
        // Asks should be sorted ascending
        assert!(orderbook.asks[0].price <= orderbook.asks[1].price);
    }

    #[test]
    fn test_parse_trade() {
        let data = json!({
            "coin": "BTC",
            "side": "B",
            "px": "50000",
            "sz": "0.5",
            "time": 1700000000000i64,
            "tid": "123456"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.side, ccxt_core::types::OrderSide::Buy);
        assert_eq!(
            trade.price,
            ccxt_core::types::financial::Price::new(dec!(50000))
        );
        assert_eq!(
            trade.amount,
            ccxt_core::types::financial::Amount::new(dec!(0.5))
        );
    }

    #[test]
    fn test_parse_order_status() {
        assert_eq!(
            parse_order_status("open"),
            ccxt_core::types::OrderStatus::Open
        );
        assert_eq!(
            parse_order_status("filled"),
            ccxt_core::types::OrderStatus::Closed
        );
        assert_eq!(
            parse_order_status("canceled"),
            ccxt_core::types::OrderStatus::Cancelled
        );
    }

    #[test]
    fn test_parse_balance() {
        let data = json!({
            "marginSummary": {
                "accountValue": "10000",
                "totalMarginUsed": "2000"
            },
            "withdrawable": "8000"
        });

        let balance = parse_balance(&data).unwrap();
        let usdc = balance.get("USDC").unwrap();
        assert_eq!(usdc.total, dec!(10000));
        assert_eq!(usdc.free, dec!(8000));
    }
}

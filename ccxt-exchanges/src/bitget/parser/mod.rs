//! Bitget data parser module.
//!
//! Converts Bitget API response data into standardized CCXT format structures.
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
//! - `position` - Position data parsing
//! - `funding` - Funding rate data parsing
//! - `ws` - WebSocket data parsing

use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;

// WebSocket parser module
pub mod ws;

// Re-export all parser functions
pub use balance::parse_balance;
pub use funding::{parse_funding_rate, parse_funding_rate_history};
pub use market::parse_market;
pub use ohlcv::parse_ohlcv;
pub use order::{parse_order, parse_order_status};
pub use orderbook::parse_orderbook;
pub use position::parse_position;
pub use ticker::parse_ticker;
pub use trade::parse_trade;

// Re-export WebSocket parser functions
pub use ws::{
    parse_balance as parse_balance_ws, parse_ohlcv as parse_ohlcv_ws,
    parse_order as parse_order_ws, parse_order_from_data, parse_orderbook as parse_orderbook_ws,
    parse_ticker as parse_ticker_ws, parse_trades, parse_ws_account_trade,
};

// Re-export for backward compatibility
pub use ccxt_core::parser_utils::{datetime_to_timestamp, timestamp_to_datetime};

mod balance;
mod funding;
mod market;
mod ohlcv;
mod order;
mod orderbook;
mod position;
mod ticker;
mod trade;

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to parse a string field as f64.
pub(crate) fn parse_f64_field(data: &Value, field: &str) -> Option<f64> {
    data[field]
        .as_str()
        .and_then(|s| {
            if s.is_empty() {
                None
            } else {
                s.parse::<f64>().ok()
            }
        })
        .or_else(|| data[field].as_f64())
}

/// Helper to parse decimal from various formats.
#[allow(unused)]
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
    #![allow(clippy::disallowed_methods)]
    use super::*;
    use ccxt_core::types::{
        OrderSide, OrderStatus, OrderType, Symbol,
        financial::{Amount, Price},
    };
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_market() {
        let data = json!({
            "symbol": "BTCUSDT",
            "baseCoin": "BTC",
            "quoteCoin": "USDT",
            "status": "online",
            "category": "SPOT",
            "pricePrecision": "2",
            "quantityPrecision": "4",
            "minTradeNum": "0.0001",
            "makerFeeRate": "0.001",
            "takerFeeRate": "0.001"
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(market.id, "BTCUSDT");
        assert_eq!(market.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
    }

    #[test]
    fn test_parse_ticker() {
        let data = json!({
            "symbol": "BTCUSDT",
            "lastPrice": "50000.00",
            "highPrice24h": "51000.00",
            "lowPrice24h": "49000.00",
            "bid1Price": "49999.00",
            "ask1Price": "50001.00",
            "volume24h": "1000.5",
            "ts": "1700000000000"
        });

        let ticker = parse_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol, Symbol::new_unchecked("BTCUSDT"));
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
        assert_eq!(ticker.high, Some(Price::new(dec!(51000.00))));
        assert_eq!(ticker.low, Some(Price::new(dec!(49000.00))));
        assert_eq!(ticker.timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_orderbook() {
        let data = json!({
            "bids": [
                ["50000.00", "1.5"],
                ["49999.00", "2.0"]
            ],
            "asks": [
                ["50001.00", "1.0"],
                ["50002.00", "3.0"]
            ],
            "ts": "1700000000000"
        });

        let orderbook = parse_orderbook(&data, "BTC/USDT".to_string()).unwrap();
        assert_eq!(orderbook.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);
        assert_eq!(orderbook.bids[0].price, Price::new(dec!(50000.00)));
        assert_eq!(orderbook.asks[0].price, Price::new(dec!(50001.00)));
    }

    #[test]
    fn test_parse_trade() {
        let data = json!({
            "tradeId": "123456",
            "symbol": "BTCUSDT",
            "side": "buy",
            "price": "50000.00",
            "size": "0.5",
            "ts": "1700000000000"
        });

        let trade = parse_trade(&data, None).unwrap();
        assert_eq!(trade.id, Some("123456".to_string()));
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.price, Price::new(dec!(50000.00)));
        assert_eq!(trade.amount, Amount::new(dec!(0.5)));
    }

    #[test]
    fn test_parse_ohlcv() {
        let data = json!([
            "1700000000000",
            "50000.00",
            "51000.00",
            "49000.00",
            "50500.00",
            "1000.5"
        ]);

        let ohlcv = parse_ohlcv(&data).unwrap();
        assert_eq!(ohlcv.timestamp, 1700000000000);
        assert_eq!(ohlcv.open, 50000.00);
        assert_eq!(ohlcv.high, 51000.00);
        assert_eq!(ohlcv.low, 49000.00);
        assert_eq!(ohlcv.close, 50500.00);
        assert_eq!(ohlcv.volume, 1000.5);
    }

    #[test]
    fn test_parse_order_status() {
        assert_eq!(parse_order_status("live"), OrderStatus::Open);
        assert_eq!(parse_order_status("partially_filled"), OrderStatus::Open);
        assert_eq!(parse_order_status("filled"), OrderStatus::Closed);
        assert_eq!(parse_order_status("cancelled"), OrderStatus::Cancelled);
        assert_eq!(parse_order_status("expired"), OrderStatus::Expired);
        assert_eq!(parse_order_status("rejected"), OrderStatus::Rejected);
    }

    #[test]
    fn test_parse_order() {
        let data = json!({
            "orderId": "123456789",
            "symbol": "BTCUSDT",
            "side": "buy",
            "orderType": "limit",
            "price": "50000.00",
            "size": "0.5",
            "status": "live",
            "cTime": "1700000000000"
        });

        let order = parse_order(&data, None).unwrap();
        assert_eq!(order.id, "123456789");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, Some(dec!(50000.00)));
        assert_eq!(order.amount, dec!(0.5));
        assert_eq!(order.status, OrderStatus::Open);
    }

    /// Test V3 API order parsing with new field names
    #[test]
    fn test_parse_order_v3_api() {
        // V3 API filled market order
        let data = json!({
            "orderId": "987654321",
            "symbol": "BTCUSDT",
            "side": "buy",
            "orderType": "market",
            "amount": "100",
            "orderStatus": "filled",
            "avgPrice": "75000.00",
            "cumExecQty": "0.001333",
            "cumExecValue": "99.975",
            "createdTime": "1700000000000",
            "updatedTime": "1700000001000"
        });

        let order = parse_order(&data, None).unwrap();
        assert_eq!(order.id, "987654321");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.amount, dec!(100.0));
        assert_eq!(order.status, OrderStatus::Closed); // filled -> Closed
        assert_eq!(order.average, Some(dec!(75000.00)));
        assert_eq!(order.filled, Some(dec!(0.001333)));
        assert_eq!(order.cost, Some(dec!(99.975)));
        assert_eq!(order.timestamp, Some(1700000000000));
        assert_eq!(order.last_trade_timestamp, Some(1700000001000));
    }

    #[test]
    fn test_parse_balance() {
        let data = json!([
            {
                "coin": "BTC",
                "available": "1.5",
                "frozen": "0.5"
            },
            {
                "coin": "USDT",
                "available": "10000.00",
                "frozen": "0"
            }
        ]);

        let balance = parse_balance(&data).unwrap();
        let btc = balance.get("BTC").unwrap();
        assert_eq!(btc.free, dec!(1.5));
        assert_eq!(btc.used, dec!(0.5));
        assert_eq!(btc.total, dec!(2.0));

        let usdt = balance.get("USDT").unwrap();
        assert_eq!(usdt.free, dec!(10000.00));
        assert_eq!(usdt.total, dec!(10000.00));
    }

    #[test]
    fn test_timestamp_to_datetime() {
        let ts = 1700000000000i64;
        let dt = timestamp_to_datetime(ts).unwrap();
        assert!(dt.contains("2023-11-14"));
    }
}

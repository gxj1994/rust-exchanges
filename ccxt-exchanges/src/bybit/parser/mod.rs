//! Bybit parser module.
//!
//! Provides data parsing functions for Bybit exchange API responses.

mod balance;
mod market;
mod ohlcv;
mod order;
mod orderbook;
mod ticker;
mod trade;

// WebSocket parser module
pub mod ws;

// Re-export all public functions
pub use balance::parse_balance;
pub use market::parse_market;
pub use ohlcv::parse_ohlcv;
pub use order::{parse_order, parse_order_status};
pub use orderbook::{parse_orderbook, parse_orderbook_with_ts};
pub use ticker::{parse_ticker, parse_ticker_with_time, parse_ticker_with_ws_timestamp};
pub use trade::parse_trade;

// Re-export WebSocket parser functions
pub use ws::{
    parse_balance as parse_balance_ws, parse_execution, parse_ohlcv as parse_ohlcv_ws,
    parse_order as parse_order_ws, parse_order_from_data, parse_orderbook as parse_orderbook_ws,
    parse_ticker as parse_ticker_ws, parse_trades,
};

// Re-export for backward compatibility
pub use ccxt_core::parser_utils::{datetime_to_timestamp, timestamp_to_datetime};

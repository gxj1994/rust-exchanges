//! OKX parser module.
//!
//! Provides data parsing functions for OKX exchange API responses.

mod balance;
mod funding_rate;
mod market;
mod ohlcv;
mod order;
mod orderbook;
mod position;
mod ticker;
mod trade;

pub mod ws;

// Re-export all public functions
pub use balance::parse_balance;
pub use funding_rate::{parse_funding_rate, parse_funding_rate_history};
pub use market::parse_market;
pub use ohlcv::parse_ohlcv;
pub use order::{parse_order, parse_order_status};
pub use orderbook::parse_orderbook;
pub use position::parse_position;
pub use ticker::parse_ticker;
pub use trade::parse_trade;

// Re-export for backward compatibility
pub use ccxt_core::parser_utils::{datetime_to_timestamp, timestamp_to_datetime};

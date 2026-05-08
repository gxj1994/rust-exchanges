//! Common shared building blocks for exchange implementations.
//!
//! This module provides reusable helpers to reduce duplication across exchanges:
//! - ExchangeMetadata helper
//! - Capability factories
//! - Environment URL resolver
//! - Parser helpers
//! - WebSocket helpers

pub mod environment;
pub mod metadata;
pub mod parser_helpers;
pub mod ws_helpers;

// Re-export commonly used items
pub use metadata::{
    ExchangeMetadata, futures_only_capabilities, market_data_only_capabilities,
    standard_cex_capabilities, standard_cex_public_ws_capabilities,
    trading_without_edit_capabilities,
};

pub use environment::EnvironmentUrlResolver;
pub use environment::{
    binance_resolver, bitget_resolver, bybit_resolver, hyperliquid_resolver, okx_resolver,
};

// Re-export parser helpers
pub use parser_helpers::{
    ParseHelper, is_valid_amount, is_valid_decimal, is_valid_price, is_valid_timestamp,
    parse_amount_safe, parse_decimal_from_value, parse_ohlcv_array, parse_order_book_level,
    parse_price_safe, parse_timestamp_from_value, parse_timestamp_safe,
};

// Re-export WebSocket helpers
pub use ws_helpers::{
    build_subscription_channel, channel_name_to_type, timeframe_to_bybit_interval,
    timeframe_to_uppercase_interval,
};

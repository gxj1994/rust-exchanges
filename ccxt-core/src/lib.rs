//! CCXT Core Library
//!
//! This is the core library for CCXT Rust implementation, providing fundamental
//! data structures, error types, and traits for cryptocurrency exchange integration.
//!
//! # Features
//!
//! - **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - **Precision**: Uses `rust_decimal::Decimal` for accurate financial calculations
//! - **Async/Await**: Built on tokio for high-performance async operations
//! - **Error Handling**: Comprehensive error types with `thiserror`
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_core::prelude::*;
//!
//! # fn example() -> std::result::Result<(), Box<dyn std::error::Error>> {
//! // Create a market
//! let market = Market::new_spot(
//!     "BTCUSDT".to_string(),
//!     Symbol::new_unchecked("BTC/USDT"),
//!     "BTC".to_string(),
//!     "USDT".to_string(),
//! );
//!
//! // Create an order
//! let order = Order::new(
//!     "12345".to_string(),
//!     Symbol::new_unchecked("BTC/USDT"),
//!     OrderType::Limit,
//!     OrderSide::Buy,
//!     rust_decimal_macros::dec!(0.1),
//!     Some(rust_decimal_macros::dec!(50000.0)),
//!     OrderStatus::Open,
//! );
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
// =============================================================================
// Global Clippy Lint Suppressions
// =============================================================================
// 最小化的全局抑制，仅在真正需要时保留
// 原则：优先修复代码问题，而非抑制警告；必须使用局部抑制替代全局抑制
//
// 保留的抑制（有充分理由）：
// - module_name_repetitions: Rust库常见模式，如OrderType在order模块中
// - similar_names: 交易术语需要相似命名（bid/ask, buy/sell）
// =============================================================================
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]

// Re-exports of external dependencies
pub use rust_decimal;
pub use serde;
pub use serde_json;

// Core modules
pub mod base_exchange;
pub mod capability;
pub mod core;
pub mod error;
pub mod network;
pub mod symbol;
pub mod trading;
/// Exchange trait hierarchy for modular capability composition
pub mod traits;
pub mod types;
pub mod utils;

/// WebSocket unified architecture module (v2)
///
/// This module provides a unified WebSocket framework with:
/// - Trait-based abstraction for subscription building, parsing, and endpoint management
/// - Message broadcasting with reference counting
/// - Multi-URL support for exchanges like Bybit
/// - Automatic reconnection with subscription recovery
pub mod ws;

// Backward compatibility alias for ws_exchange
#[doc(hidden)]
pub mod ws_exchange {
    pub use crate::core::ws_exchange::*;
}

// Backward compatibility aliases
#[doc(hidden)]
pub mod exchange {
    pub use crate::core::exchange::*;
}
#[doc(hidden)]
pub use core as exchange_mod;
#[doc(hidden)]
pub use core::auth;
#[doc(hidden)]
pub use core::config;
#[doc(hidden)]
pub use core::credentials;
#[doc(hidden)]
pub use core::time;
#[doc(hidden)]
pub use network as http_client;
#[doc(hidden)]
pub use network as ws_client;
#[doc(hidden)]
pub use network::circuit_breaker;
#[doc(hidden)]
pub use network::rate_limiter;
#[doc(hidden)]
pub use network::retry_strategy;
#[doc(hidden)]
pub use network::signed_request;
#[doc(hidden)]
pub use trading::precision;
#[doc(hidden)]
pub use utils::logging;
#[doc(hidden)]
pub use utils::parser_utils;

// Note: Test configuration has been moved to ccxt-exchanges/tests/support/
// Use `use ccxt_exchanges::tests::support::TestConfig;` in integration tests

// Re-exports of core types for convenience
pub use base_exchange::{BaseExchange, ExchangeConfig, ExchangeConfigBuilder, MarketCache};
pub use capability::{
    Capabilities, Capability, ExchangeCapabilities, ExchangeCapabilitiesBuilder, TraitCategory,
};
pub use core::auth::{
    DigestFormat, HashAlgorithm, base64_to_base64url, base64url_decode, eddsa_sign, hash,
    hmac_sign, jwt_sign,
};
pub use core::credentials::{SecretBytes, SecretString};
pub use core::exchange::{ArcExchange, BoxedExchange, Exchange, ExchangeExt};
pub use core::time::{
    iso8601, microseconds, milliseconds, parse_date, parse_iso8601, seconds, ymd, ymdhms, yyyymmdd,
};
pub use error::{
    ContextExt, Error, ExchangeErrorDetails, NetworkError, OrderError, ParseError, Result,
};
pub use network::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerEvent, CircuitState,
};
pub use network::rate_limiter::{MultiTierRateLimiter, RateLimiter, RateLimiterConfig};
pub use network::retry_strategy::{RetryConfig, RetryStrategy, RetryStrategyType};
pub use network::ws_client::{
    BackoffConfig, BackoffStrategy, DEFAULT_MAX_SUBSCRIPTIONS, DEFAULT_SHUTDOWN_TIMEOUT,
    SubscriptionInfo, SubscriptionManager, WsClient, WsConfig, WsConnectionState, WsError,
    WsErrorKind, WsEvent, WsMessage, WsStats, WsStatsSnapshot,
};
pub use symbol::{SymbolError, SymbolFormatter, SymbolParser};
pub use trading::precision::{
    CountingMode, PaddingMode, RoundingMode, decimal_to_precision, number_to_string,
    precision_from_string,
};
pub use types::common::symbol::{ContractType, ExpiryDate, ParsedSymbol, SymbolMarketType};
pub use types::{
    Amount, Balance, BalanceEntry, Cost, Currency, CurrencyNetwork, DefaultSubType, DefaultType,
    DefaultTypeError, EndpointType, Fee, Market, MarketLimits, MarketPrecision, MarketType, MinMax,
    Ohlcv, Order, OrderBook, OrderBookEntry, OrderBookSide, OrderSide, OrderStatus, OrderType,
    PrecisionMode, Price, TakerOrMaker, Ticker, TickerParams, TickerParamsBuilder, Timeframe,
    Trade, TradingLimits, resolve_market_type,
};
pub use utils::logging::{LogConfig, LogFormat, LogLevel, init_logging, try_init_logging};
pub use ws_exchange::{FullExchange, MessageStream, WsExchange};
// Re-export CancellationToken for convenient access
pub use tokio_util::sync::CancellationToken;
///
/// Import everything you need with:
/// ```rust
/// use ccxt_core::prelude::*;
/// ```
pub mod prelude {
    pub use crate::base_exchange::{
        BaseExchange, ExchangeConfig, ExchangeConfigBuilder, MarketCache,
    };
    pub use crate::core::auth::{
        DigestFormat, HashAlgorithm, base64_to_base64url, base64url_decode, eddsa_sign, hash,
        hmac_sign, jwt_sign,
    };
    pub use crate::error::{ContextExt, Error, Result};
    pub use crate::network::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
    // Re-export unified Exchange trait and capabilities
    pub use crate::core::exchange::{
        ArcExchange, BoxedExchange, Exchange, ExchangeCapabilities, ExchangeExt,
    };
    // Re-export WebSocket exchange trait
    pub use crate::core::time::{
        iso8601, microseconds, milliseconds, parse_date, parse_iso8601, seconds, ymd, ymdhms,
        yymmdd, yyyymmdd,
    };
    pub use crate::network::http_client::{HttpClient, HttpConfig};
    pub use crate::network::rate_limiter::{MultiTierRateLimiter, RateLimiter, RateLimiterConfig};
    pub use crate::network::retry_strategy::{RetryConfig, RetryStrategy, RetryStrategyType};
    pub use crate::trading::precision::{
        CountingMode, PaddingMode, RoundingMode, decimal_to_precision, number_to_string,
        precision_from_string,
    };
    pub use crate::types::{
        Amount, Balance, BalanceEntry, Currency, DefaultSubType, DefaultType, DefaultTypeError,
        EndpointType, Fee, Market, MarketLimits, MarketPrecision, MarketType, Ohlcv, Order,
        OrderBook, OrderBookEntry, OrderBookSide, OrderSide, OrderStatus, OrderType, PrecisionMode,
        Price, Symbol, TakerOrMaker, Ticker, TickerParams, TickerParamsBuilder, Timeframe,
        Timestamp, Trade, TradingLimits, resolve_market_type,
    };
    pub use crate::utils::logging::{
        LogConfig, LogFormat, LogLevel, init_logging, try_init_logging,
    };
    pub use crate::ws_exchange::{FullExchange, MessageStream, WsExchange};
    // Symbol types for unified symbol format
    pub use crate::network::ws_client::{
        BackoffConfig, BackoffStrategy, DEFAULT_MAX_SUBSCRIPTIONS, DEFAULT_SHUTDOWN_TIMEOUT,
        SubscriptionInfo, SubscriptionManager, WsClient, WsConfig, WsConnectionState, WsError,
        WsErrorKind, WsEvent, WsMessage, WsStats, WsStatsSnapshot,
    };
    pub use crate::symbol::{SymbolError, SymbolFormatter, SymbolParser};
    pub use crate::types::common::symbol::{
        ContractType, ExpiryDate, ParsedSymbol, SymbolMarketType,
    };
    // Re-export CancellationToken for convenient access
    pub use rust_decimal::Decimal;
    pub use serde::{Deserialize, Serialize};
    pub use tokio_util::sync::CancellationToken;
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "ccxt-core");
    }
}

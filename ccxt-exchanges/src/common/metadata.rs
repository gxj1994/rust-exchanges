//! Shared building blocks for exchange implementations.
//!
//! This module provides reusable helpers to reduce duplication across exchanges:
//! - ExchangeMetadata helper
//! - Capability factories
//! - (Future) Environment URL resolver

use ccxt_core::exchange::{Capability, ExchangeCapabilities};

/// Lightweight shared metadata container.
///
/// Purpose:
/// - Remove repeated `id/name/version/certified/rate_limit` implementations
/// - Keep metadata as a single source of truth
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::common::metadata::ExchangeMetadata;
///
/// const METADATA: ExchangeMetadata = ExchangeMetadata {
///     id: "example",
///     name: "Example Exchange",
///     version: "v1",
///     certified: false,
///     default_rate_limit: 100,
///     has_websocket: true,
/// };
/// ```
pub struct ExchangeMetadata {
    /// Exchange identifier (lowercase, URL-safe).
    pub id: &'static str,
    /// Human-readable exchange name.
    pub name: &'static str,
    /// API version string.
    pub version: &'static str,
    /// Whether this exchange is certified.
    pub certified: bool,
    /// Default rate limit in milliseconds.
    pub default_rate_limit: u32,
    /// Whether this exchange supports WebSocket.
    pub has_websocket: bool,
}

impl ExchangeMetadata {
    /// Create a new ExchangeMetadata instance.
    pub const fn new(
        id: &'static str,
        name: &'static str,
        version: &'static str,
        default_rate_limit: u32,
        has_websocket: bool,
    ) -> Self {
        Self {
            id,
            name,
            version,
            certified: false,
            default_rate_limit,
            has_websocket,
        }
    }

    /// Create a new ExchangeMetadata with certified flag.
    pub const fn certified(
        id: &'static str,
        name: &'static str,
        version: &'static str,
        default_rate_limit: u32,
        has_websocket: bool,
    ) -> Self {
        Self {
            id,
            name,
            version,
            certified: true,
            default_rate_limit,
            has_websocket,
        }
    }
}

// ============================================================================
// Capability Factories
// ============================================================================

/// Create capabilities for a standard CEX with full trading support.
///
/// This is the most common capability set for centralized exchanges.
pub fn standard_cex_capabilities() -> ExchangeCapabilities {
    ExchangeCapabilities::builder()
        .market_data()
        .trading()
        .capability(Capability::FetchBalance)
        .capability(Capability::FetchMyTrades)
        .capability(Capability::FetchPositions)
        .capability(Capability::SetLeverage)
        .build()
}

/// Create capabilities for a market-data-only exchange (read-only).
///
/// Use this for exchanges that only support public API access.
pub fn market_data_only_capabilities() -> ExchangeCapabilities {
    ExchangeCapabilities::builder().market_data().build()
}

/// Create capabilities for a CEX without order editing.
///
/// Many exchanges don't support order editing, this is a common pattern.
pub fn trading_without_edit_capabilities() -> ExchangeCapabilities {
    ExchangeCapabilities::builder()
        .market_data()
        .trading()
        .without_capability(Capability::FetchOrders)
        .without_capability(Capability::FetchCanceledOrders)
        .capability(Capability::FetchBalance)
        .capability(Capability::FetchMyTrades)
        .build()
}

/// Create capabilities for a futures-only exchange.
///
/// Use this for exchanges that only support derivatives trading.
pub fn futures_only_capabilities() -> ExchangeCapabilities {
    ExchangeCapabilities::builder()
        .market_data()
        .trading()
        .capability(Capability::FetchBalance)
        .capability(Capability::FetchMyTrades)
        .capability(Capability::FetchPositions)
        .capability(Capability::SetLeverage)
        .capability(Capability::SetMarginMode)
        .capability(Capability::FetchFundingRate)
        .capability(Capability::FetchFundingRates)
        .build()
}

/// Create capabilities with WebSocket support.
///
/// Use this as a base and add specific capabilities as needed.
pub fn standard_cex_public_ws_capabilities() -> ExchangeCapabilities {
    ExchangeCapabilities::builder()
        .market_data()
        .trading()
        .capability(Capability::FetchBalance)
        .capability(Capability::FetchMyTrades)
        .capability(Capability::Websocket)
        .capability(Capability::WatchTicker)
        .capability(Capability::WatchOrderBook)
        .capability(Capability::WatchTrades)
        .capability(Capability::WatchOrders)
        .capability(Capability::WatchMyTrades)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_metadata_creation() {
        let metadata = ExchangeMetadata::new("test", "Test Exchange", "v1", 100, true);

        assert_eq!(metadata.id, "test");
        assert_eq!(metadata.name, "Test Exchange");
        assert_eq!(metadata.version, "v1");
        assert!(!metadata.certified);
        assert_eq!(metadata.default_rate_limit, 100);
        assert!(metadata.has_websocket);
    }

    #[test]
    fn test_exchange_metadata_certified() {
        let metadata = ExchangeMetadata::certified("binance", "Binance", "v3", 50, true);

        assert_eq!(metadata.id, "binance");
        assert!(metadata.certified);
    }

    #[test]
    fn test_standard_cex_capabilities() {
        let caps = standard_cex_capabilities();

        assert!(caps.fetch_markets());
        assert!(caps.fetch_ticker());
        assert!(caps.create_order());
        assert!(caps.cancel_order());
        assert!(caps.fetch_balance());
    }

    #[test]
    fn test_market_data_only_capabilities() {
        let caps = market_data_only_capabilities();

        assert!(caps.fetch_markets());
        assert!(caps.fetch_ticker());
        assert!(!caps.create_order());
        assert!(!caps.fetch_balance());
    }

    #[test]
    fn test_trading_without_edit_capabilities() {
        let caps = trading_without_edit_capabilities();

        assert!(caps.create_order());
        assert!(caps.cancel_order());
    }

    #[test]
    fn test_futures_only_capabilities() {
        let caps = futures_only_capabilities();

        assert!(caps.fetch_positions());
        assert!(caps.set_leverage());
        assert!(caps.fetch_funding_rate());
    }
}

//! Symbol converter trait for exchange-specific symbol conversion
//!
//! This module defines the `SymbolConverter` trait that all exchanges must implement
//! to provide unified symbol conversion functionality.

use crate::symbol::context::SymbolContext;
use crate::symbol::error::SymbolError;
use crate::types::common::symbol::{ParsedSymbol, SymbolMarketType};

/// Trait for converting between unified and exchange-specific symbol formats
///
/// Each exchange must implement this trait to provide bidirectional conversion
/// between CCXT unified format and the exchange's native format.
///
/// # Unified Format
///
/// | Market Type | Format | Example |
/// |-------------|--------|---------|
/// | Spot | `BASE/QUOTE` | `BTC/USDT` |
/// | Linear Swap | `BASE/QUOTE:QUOTE` | `BTC/USDT:USDT` |
/// | Inverse Swap | `BASE/QUOTE:BASE` | `BTC/USD:BTC` |
/// | Futures | `BASE/QUOTE:SETTLE-YYMMDD` | `BTC/USDT:USDT-241231` |
///
/// # Implementation Notes
///
/// Implementations should handle:
/// - Case normalization (all symbols should be uppercase)
/// - Market type detection from exchange ID format
/// - Expiry date extraction for futures
///
/// # Example
///
/// ```rust,ignore
/// use ccxt_core::symbol::{SymbolConverter, SymbolContext, ParsedSymbol};
///
/// // Convert unified -> exchange
/// let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
/// let exchange_id = converter.to_exchange_id(&symbol);
///
/// // Convert exchange -> unified (with context)
/// let ctx = SymbolContext::spot().with_base_quote("BTC", "USDT");
/// let unified = converter.from_exchange_id("BTCUSDT", ctx);
///
/// // Convert exchange -> unified (inferred, for WebSocket)
/// let inferred = converter.exchange_to_unified_inferred("BTCUSDT");
/// ```
pub trait SymbolConverter: Send + Sync {
    /// Convert a unified symbol to exchange-specific ID
    ///
    /// # Arguments
    ///
    /// * `symbol` - The parsed unified symbol
    ///
    /// # Returns
    ///
    /// The exchange-specific symbol ID (e.g., "BTCUSDT" for Binance, "BTC-USDT" for OKX)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
    /// let id = converter.to_exchange_id(&symbol);
    /// assert_eq!(id, "BTCUSDT"); // or "BTC-USDT" for OKX, etc.
    /// ```
    fn to_exchange_id(&self, symbol: &ParsedSymbol) -> String;

    /// Convert an exchange-specific ID to unified symbol
    ///
    /// This method requires context information because some exchanges
    /// (like Binance/Bybit) use IDs without separators (e.g., "BTCUSDT"),
    /// making it impossible to determine base/quote without additional info.
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The exchange-specific symbol ID
    /// * `context` - Context information (market type, settle, base, quote)
    ///
    /// # Returns
    ///
    /// `Ok(ParsedSymbol)` on success, `Err(SymbolError)` on failure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ctx = SymbolContext::spot().with_base_quote("BTC", "USDT");
    /// let symbol = converter.from_exchange_id("BTCUSDT", ctx)?;
    /// ```
    fn from_exchange_id(
        &self,
        exchange_id: &str,
        context: SymbolContext,
    ) -> Result<ParsedSymbol, SymbolError>;

    /// Convert an exchange-specific ID to unified format (inferred)
    ///
    /// This method makes a best-effort conversion without requiring context.
    /// Used primarily for WebSocket message parsing where context is not available.
    ///
    /// **Note**: The result may be ambiguous for some exchanges.
    /// For precise conversion, use `from_exchange_id()` with proper context.
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The exchange-specific symbol ID
    ///
    /// # Returns
    ///
    /// The inferred unified symbol string
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // OKX: "BTC-USDT-SWAP" -> "BTC/USDT:USDT"
    /// let unified = converter.exchange_to_unified_inferred("BTC-USDT-SWAP");
    ///
    /// // Binance: "BTCUSD_PERP" -> "BTC/USD:BTC"
    /// let unified = converter.exchange_to_unified_inferred("BTCUSD_PERP");
    /// ```
    fn exchange_to_unified_inferred(&self, exchange_id: &str) -> String;

    /// Detect market type from exchange-specific ID
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The exchange-specific symbol ID
    ///
    /// # Returns
    ///
    /// `Some(SymbolMarketType)` if detected, `None` if ambiguous or unknown
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // OKX: "BTC-USDT-SWAP" -> Swap
    /// let market_type = converter.detect_market_type("BTC-USDT-SWAP");
    ///
    /// // Binance: "BTCUSDT_241231" -> Futures
    /// let market_type = converter.detect_market_type("BTCUSDT_241231");
    /// ```
    fn detect_market_type(&self, exchange_id: &str) -> Option<SymbolMarketType>;

    /// Check if the exchange ID represents a spot market
    fn is_spot(&self, exchange_id: &str) -> bool {
        self.detect_market_type(exchange_id) == Some(SymbolMarketType::Spot)
    }

    /// Check if the exchange ID represents a swap/perpetual market
    fn is_swap(&self, exchange_id: &str) -> bool {
        self.detect_market_type(exchange_id) == Some(SymbolMarketType::Swap)
    }

    /// Check if the exchange ID represents a futures market
    fn is_futures(&self, exchange_id: &str) -> bool {
        self.detect_market_type(exchange_id) == Some(SymbolMarketType::Futures)
    }

    /// Convert a unified symbol string to exchange ID
    ///
    /// Convenience method that parses the string first.
    ///
    /// # Arguments
    ///
    /// * `unified_symbol` - The unified symbol string (e.g., "BTC/USDT")
    ///
    /// # Returns
    ///
    /// `Ok(String)` with exchange ID, or `Err(SymbolError)` if parsing fails
    fn unified_to_exchange(&self, unified_symbol: &str) -> Result<String, SymbolError> {
        let parsed = crate::symbol::SymbolParser::parse(unified_symbol)?;
        Ok(self.to_exchange_id(&parsed))
    }
}

#[cfg(test)]
mod tests {
    // Tests are implemented by each exchange's converter
}

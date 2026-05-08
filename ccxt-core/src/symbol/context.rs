//! Symbol conversion context
//!
//! This module provides the `SymbolContext` struct for converting exchange-specific
//! IDs back to unified symbols when additional context is required.

use crate::types::common::symbol::{ParsedSymbol, SymbolMarketType};

/// Context for converting exchange-specific IDs to unified symbols
///
/// When converting from exchange-specific formats (e.g., "BTCUSDT") back to
/// unified format (e.g., "BTC/USDT"), some exchanges require additional context
/// because their IDs lack separators or have ambiguous formats.
///
/// # Example
///
/// ```rust
/// use ccxt_core::symbol::SymbolContext;
///
/// // Spot context
/// let spot_ctx = SymbolContext::spot();
///
/// // Swap context with settlement currency
/// let swap_ctx = SymbolContext::swap("USDT");
///
/// // Futures context with base/quote info
/// let futures_ctx = SymbolContext::futures("USDT").with_base_quote("BTC", "USDT");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolContext {
    /// Market type (Spot, Swap, Futures)
    pub market_type: SymbolMarketType,
    /// Settlement currency (required for derivatives)
    pub settle: Option<String>,
    /// Base currency (required for exchanges without separators)
    pub base: Option<String>,
    /// Quote currency (required for exchanges without separators)
    pub quote: Option<String>,
}

impl SymbolContext {
    /// Create a context for spot markets
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::symbol::SymbolContext;
    ///
    /// let ctx = SymbolContext::spot();
    /// assert!(ctx.settle.is_none());
    /// ```
    pub fn spot() -> Self {
        Self {
            market_type: SymbolMarketType::Spot,
            settle: None,
            base: None,
            quote: None,
        }
    }

    /// Create a context for swap/perpetual markets
    ///
    /// # Arguments
    ///
    /// * `settle` - Settlement currency (e.g., "USDT" for linear, "BTC" for inverse)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::symbol::SymbolContext;
    ///
    /// let ctx = SymbolContext::swap("USDT");
    /// assert_eq!(ctx.settle, Some("USDT".to_string()));
    /// ```
    pub fn swap(settle: &str) -> Self {
        Self {
            market_type: SymbolMarketType::Swap,
            settle: Some(settle.to_uppercase()),
            base: None,
            quote: None,
        }
    }

    /// Create a context for futures markets
    ///
    /// # Arguments
    ///
    /// * `settle` - Settlement currency (e.g., "USDT" for linear, "BTC" for inverse)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::symbol::SymbolContext;
    ///
    /// let ctx = SymbolContext::futures("USDT");
    /// assert_eq!(ctx.market_type, ccxt_core::types::common::symbol::SymbolMarketType::Futures);
    /// ```
    pub fn futures(settle: &str) -> Self {
        Self {
            market_type: SymbolMarketType::Futures,
            settle: Some(settle.to_uppercase()),
            base: None,
            quote: None,
        }
    }

    /// Add base and quote currency information
    ///
    /// Required for exchanges like Binance/Bybit where exchange IDs
    /// don't have separators (e.g., "BTCUSDT").
    ///
    /// # Arguments
    ///
    /// * `base` - Base currency (e.g., "BTC")
    /// * `quote` - Quote currency (e.g., "USDT")
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::symbol::SymbolContext;
    ///
    /// let ctx = SymbolContext::swap("USDT").with_base_quote("BTC", "USDT");
    /// assert_eq!(ctx.base, Some("BTC".to_string()));
    /// assert_eq!(ctx.quote, Some("USDT".to_string()));
    /// ```
    pub fn with_base_quote(mut self, base: &str, quote: &str) -> Self {
        self.base = Some(base.to_uppercase());
        self.quote = Some(quote.to_uppercase());
        self
    }

    /// Create context from a ParsedSymbol
    ///
    /// # Arguments
    ///
    /// * `symbol` - The parsed symbol to extract context from
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::symbol::{ParsedSymbol, SymbolContext};
    ///
    /// let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
    /// let ctx = SymbolContext::from_symbol(&symbol);
    /// assert_eq!(ctx.settle, Some("USDT".to_string()));
    /// ```
    pub fn from_symbol(symbol: &ParsedSymbol) -> Self {
        Self {
            market_type: symbol.market_type(),
            settle: symbol.settle.clone(),
            base: Some(symbol.base.clone()),
            quote: Some(symbol.quote.clone()),
        }
    }

    /// Check if base/quote info is available
    pub fn has_base_quote(&self) -> bool {
        self.base.is_some() && self.quote.is_some()
    }

    /// Get base currency or empty string
    pub fn base_or_empty(&self) -> &str {
        self.base.as_deref().unwrap_or("")
    }

    /// Get quote currency or empty string
    pub fn quote_or_empty(&self) -> &str {
        self.quote.as_deref().unwrap_or("")
    }

    /// Get settlement currency or empty string
    pub fn settle_or_empty(&self) -> &str {
        self.settle.as_deref().unwrap_or("")
    }
}

impl Default for SymbolContext {
    fn default() -> Self {
        Self::spot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spot_context() {
        let ctx = SymbolContext::spot();
        assert_eq!(ctx.market_type, SymbolMarketType::Spot);
        assert!(ctx.settle.is_none());
    }

    #[test]
    fn test_swap_context() {
        let ctx = SymbolContext::swap("USDT");
        assert_eq!(ctx.market_type, SymbolMarketType::Swap);
        assert_eq!(ctx.settle, Some("USDT".to_string()));
    }

    #[test]
    fn test_futures_context() {
        let ctx = SymbolContext::futures("BTC");
        assert_eq!(ctx.market_type, SymbolMarketType::Futures);
        assert_eq!(ctx.settle, Some("BTC".to_string()));
    }

    #[test]
    fn test_with_base_quote() {
        let ctx = SymbolContext::swap("USDT").with_base_quote("btc", "usdt");
        assert_eq!(ctx.base, Some("BTC".to_string()));
        assert_eq!(ctx.quote, Some("USDT".to_string()));
    }

    #[test]
    fn test_from_symbol_spot() {
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let ctx = SymbolContext::from_symbol(&symbol);
        assert_eq!(ctx.market_type, SymbolMarketType::Spot);
        assert!(ctx.settle.is_none());
        assert_eq!(ctx.base, Some("BTC".to_string()));
        assert_eq!(ctx.quote, Some("USDT".to_string()));
    }

    #[test]
    fn test_from_symbol_swap() {
        let symbol = ParsedSymbol::linear_swap("ETH".to_string(), "USDT".to_string());
        let ctx = SymbolContext::from_symbol(&symbol);
        assert_eq!(ctx.market_type, SymbolMarketType::Swap);
        assert_eq!(ctx.settle, Some("USDT".to_string()));
    }

    #[test]
    fn test_has_base_quote() {
        let ctx1 = SymbolContext::spot();
        assert!(!ctx1.has_base_quote());

        let ctx2 = SymbolContext::spot().with_base_quote("BTC", "USDT");
        assert!(ctx2.has_base_quote());
    }
}

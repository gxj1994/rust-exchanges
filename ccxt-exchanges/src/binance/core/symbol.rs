//! Binance symbol converter implementation
//!
//! This module provides conversion between unified CCXT symbols and Binance-specific
//! exchange IDs for spot, swap (perpetual), futures, and options markets.
//!
//! # Binance Symbol Formats
//!
//! | Market Type | Unified Format | Binance Format |
//! |-------------|----------------|----------------|
//! | Spot | BTC/USDT | BTCUSDT |
//! | Linear Swap | BTC/USDT:USDT | BTCUSDT |
//! | Inverse Swap | BTC/USD:BTC | BTCUSD_PERP |
//! | Linear Futures | BTC/USDT:USDT-241231 | BTCUSDT_241231 |
//! | Inverse Futures | BTC/USD:BTC-241231 | BTCUSD_241231 |
//! | Options | BTC/USDT:USDT-241231-50000-C | BTCUSDT_241231 (simplified) |
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::types::common::symbol::{ParsedSymbol, ExpiryDate, SymbolMarketType};
//! use ccxt_exchanges::binance::symbol::BinanceSymbolConverter;
//!
//! // Convert spot symbol
//! let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
//! assert_eq!(BinanceSymbolConverter::to_exchange_id(&spot), "BTCUSDT");
//!
//! // Convert linear swap symbol
//! let swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
//! assert_eq!(BinanceSymbolConverter::to_exchange_id(&swap), "BTCUSDT");
//!
//! // Convert inverse swap symbol
//! let inverse = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
//! assert_eq!(BinanceSymbolConverter::to_exchange_id(&inverse), "BTCUSD_PERP");
//!
//! // Convert futures symbol
//! let expiry = ExpiryDate::new(24, 12, 31).unwrap();
//! let futures = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
//! assert_eq!(BinanceSymbolConverter::to_exchange_id(&futures), "BTCUSDT_241231");
//! ```

use ccxt_core::symbol::{SymbolContext, SymbolConverter, SymbolError};
use ccxt_core::types::common::symbol::{ExpiryDate, ParsedSymbol, SymbolMarketType};

/// Binance symbol converter
///
/// Provides methods to convert between unified CCXT symbols and Binance-specific
/// exchange IDs.
pub struct BinanceSymbolConverter;

impl BinanceSymbolConverter {
    /// Convert a unified symbol string to Binance exchange format
    ///
    /// # Arguments
    ///
    /// * `symbol` - The unified symbol string (e.g., "BTC/USDT" or "BTC/USDT:USDT")
    ///
    /// # Returns
    ///
    /// Returns the Binance-specific symbol string (e.g., "BTCUSDT")
    ///
    /// # Examples
    ///
    /// ```ignore
    /// assert_eq!(BinanceSymbolConverter::unified_to_exchange("BTC/USDT"), "BTCUSDT");
    /// assert_eq!(BinanceSymbolConverter::unified_to_exchange("BTC/USDT:USDT"), "BTCUSDT");
    /// ```
    pub fn unified_to_exchange(symbol: &str) -> String {
        // Strip settlement part (everything after ':')
        let base_quote = if let Some(pos) = symbol.find(':') {
            &symbol[..pos]
        } else {
            symbol
        };

        // Remove the '/' separator
        base_quote.replace('/', "")
    }

    /// Convert a unified ParsedSymbol to Binance exchange ID
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed unified symbol
    ///
    /// # Returns
    ///
    /// Returns the Binance-specific exchange ID string
    ///
    /// # Format Mapping
    ///
    /// - Spot: `BASE/QUOTE` → `BASEQUOTE` (e.g., "BTC/USDT" → "BTCUSDT")
    /// - Linear Swap: `BASE/QUOTE:QUOTE` → `BASEQUOTE` (e.g., "BTC/USDT:USDT" → "BTCUSDT")
    /// - Inverse Swap: `BASE/QUOTE:BASE` → `BASEQUOTE_PERP` (e.g., "BTC/USD:BTC" → "BTCUSD_PERP")
    /// - Linear Futures: `BASE/QUOTE:QUOTE-YYMMDD` → `BASEQUOTE_YYMMDD` (e.g., "BTC/USDT:USDT-241231" → "BTCUSDT_241231")
    /// - Inverse Futures: `BASE/QUOTE:BASE-YYMMDD` → `BASEQUOTE_YYMMDD` (e.g., "BTC/USD:BTC-241231" → "BTCUSD_241231")
    /// - Options: Same format as futures (uses expiry date)
    pub fn to_exchange_id(parsed: &ParsedSymbol) -> String {
        let base = &parsed.base;
        let quote = &parsed.quote;

        match parsed.market_type() {
            SymbolMarketType::Spot => {
                // Spot: BTCUSDT
                format!("{}{}", base, quote)
            }
            SymbolMarketType::Swap => {
                // Check if inverse (settle == base)
                if parsed.is_inverse() {
                    // Inverse perpetual: BTCUSD_PERP
                    format!("{}{}_PERP", base, quote)
                } else {
                    // Linear perpetual: BTCUSDT (same as spot format on Binance)
                    format!("{}{}", base, quote)
                }
            }
            SymbolMarketType::Futures => {
                // Futures with expiry date
                if let Some(ref expiry) = parsed.expiry {
                    let date_str =
                        format!("{:02}{:02}{:02}", expiry.year, expiry.month, expiry.day);
                    // Both linear and inverse futures use underscore + date
                    format!("{}{}_{}", base, quote, date_str)
                } else {
                    // Fallback if no expiry (shouldn't happen for valid futures)
                    format!("{}{}", base, quote)
                }
            }
        }
    }

    /// Convert a Binance exchange ID to a unified ParsedSymbol
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The Binance-specific exchange ID
    /// * `market_type` - The market type (spot, swap, futures)
    /// * `settle` - Optional settlement currency (required for derivatives)
    /// * `base` - Base currency (required since Binance IDs don't have separators)
    /// * `quote` - Quote currency (required since Binance IDs don't have separators)
    ///
    /// # Returns
    ///
    /// Returns `Ok(ParsedSymbol)` if conversion succeeds, or `Err` if the format is invalid
    pub fn from_exchange_id(
        exchange_id: &str,
        market_type: SymbolMarketType,
        settle: Option<&str>,
        base: &str,
        quote: &str,
    ) -> Result<ParsedSymbol, String> {
        let base = base.to_uppercase();
        let quote = quote.to_uppercase();

        match market_type {
            SymbolMarketType::Spot => Ok(ParsedSymbol::spot(base, quote)),
            SymbolMarketType::Swap => {
                let settle = settle
                    .map(str::to_uppercase)
                    .ok_or_else(|| "Settlement currency required for swap".to_string())?;
                Ok(ParsedSymbol::swap(base, quote, settle))
            }
            SymbolMarketType::Futures => {
                let settle = settle
                    .map(str::to_uppercase)
                    .ok_or_else(|| "Settlement currency required for futures".to_string())?;

                // Extract expiry date from exchange_id (format: BTCUSDT_241231)
                let expiry = Self::extract_expiry_from_exchange_id(exchange_id)?;
                Ok(ParsedSymbol::futures(base, quote, settle, expiry))
            }
        }
    }

    /// Extract expiry date from Binance exchange ID
    ///
    /// Binance futures format: BTCUSDT_241231
    fn extract_expiry_from_exchange_id(exchange_id: &str) -> Result<ExpiryDate, String> {
        // Find underscore and extract date part
        if let Some(underscore_pos) = exchange_id.rfind('_') {
            let date_part = &exchange_id[underscore_pos + 1..];

            // Check if it's a date (6 digits) and not "PERP"
            if date_part.len() == 6 && date_part.chars().all(|c| c.is_ascii_digit()) {
                let year: u8 = date_part[0..2]
                    .parse()
                    .map_err(|_| format!("Invalid year in expiry: {}", date_part))?;
                let month: u8 = date_part[2..4]
                    .parse()
                    .map_err(|_| format!("Invalid month in expiry: {}", date_part))?;
                let day: u8 = date_part[4..6]
                    .parse()
                    .map_err(|_| format!("Invalid day in expiry: {}", date_part))?;

                return ExpiryDate::new(year, month, day)
                    .map_err(|e| format!("Invalid expiry date: {}", e));
            }
        }

        Err(format!(
            "Could not extract expiry date from exchange ID: {}",
            exchange_id
        ))
    }

    /// Check if a Binance exchange ID represents a perpetual contract
    pub fn is_perpetual(exchange_id: &str) -> bool {
        exchange_id.ends_with("_PERP")
    }

    /// Check if a Binance exchange ID represents a futures contract (with expiry)
    pub fn is_futures(exchange_id: &str) -> bool {
        if let Some(underscore_pos) = exchange_id.rfind('_') {
            let suffix = &exchange_id[underscore_pos + 1..];
            suffix.len() == 6 && suffix.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// Convert a Binance exchange ID to a unified symbol (inferred, for WebSocket scenarios)
    ///
    /// This method attempts to convert an exchange ID back to unified format
    /// without requiring additional context.
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The Binance-specific exchange ID (e.g., "BTCUSDT", "BTCUSDT_241231")
    ///
    /// # Returns
    ///
    /// The inferred unified symbol:
    /// - "BTCUSDT" → "BTC/USDT" (assumes spot or linear swap)
    /// - "BTCUSD_PERP" → "BTC/USD:BTC" (inverse perpetual)
    /// - "BTCUSDT_241231" → "BTC/USDT:USDT-241231" (futures)
    ///
    /// # Note
    ///
    /// This method makes best-effort inference. For precise conversion,
    /// use `from_exchange_id()` with proper context.
    pub fn exchange_to_unified_inferred(exchange_id: &str) -> String {
        // 1. Check for inverse perpetual: BTCUSD_PERP
        if Self::is_perpetual(exchange_id) {
            if let Some(base_quote) = exchange_id.strip_suffix("_PERP") {
                if let Some((base, _quote)) = Self::split_exchange_symbol(base_quote) {
                    // Inverse perpetual: BTCUSD_PERP → BTC/USD:BTC
                    return format!("{}/USD:{}", base, base);
                }
            }
        }

        // 2. Check for futures: BTCUSDT_241231
        if Self::is_futures(exchange_id) {
            if let Some(underscore_pos) = exchange_id.rfind('_') {
                let base_quote = &exchange_id[..underscore_pos];
                let date_part = &exchange_id[underscore_pos + 1..];

                if let Some((base, quote)) = Self::split_exchange_symbol(base_quote) {
                    // Assume linear futures (settle = quote)
                    return format!("{}/{}:{}-{}", base, quote, quote, date_part);
                }
            }
        }

        // 3. Try to split as spot/linear swap
        if let Some((base, quote)) = Self::split_exchange_symbol(exchange_id) {
            // Linear swap or spot: BTCUSDT → BTC/USDT
            return format!("{}/{}", base, quote);
        }

        // 4. Fallback: return as-is
        exchange_id.to_string()
    }

    /// Try to split an exchange symbol into base and quote currencies.
    fn split_exchange_symbol(exchange_symbol: &str) -> Option<(String, String)> {
        let common_quotes = ["USDT", "USDC", "BUSD", "USD", "BTC", "ETH"];

        for quote in &common_quotes {
            if exchange_symbol.ends_with(quote) {
                let base = &exchange_symbol[..exchange_symbol.len() - quote.len()];
                if !base.is_empty() {
                    return Some((base.to_string(), quote.to_string()));
                }
            }
        }
        None
    }
}

/// Implement SymbolConverter trait for Binance
impl SymbolConverter for BinanceSymbolConverter {
    fn to_exchange_id(&self, symbol: &ParsedSymbol) -> String {
        Self::to_exchange_id(symbol)
    }

    fn from_exchange_id(
        &self,
        exchange_id: &str,
        context: SymbolContext,
    ) -> Result<ParsedSymbol, SymbolError> {
        let base = context.base.ok_or_else(|| {
            SymbolError::MissingComponent("base currency required for Binance".to_string())
        })?;
        let quote = context.quote.ok_or_else(|| {
            SymbolError::MissingComponent("quote currency required for Binance".to_string())
        })?;

        match context.market_type {
            SymbolMarketType::Spot => Ok(ParsedSymbol::spot(base, quote)),
            SymbolMarketType::Swap => {
                let settle = context.settle.ok_or_else(|| {
                    SymbolError::MissingComponent(
                        "settlement currency required for swap".to_string(),
                    )
                })?;
                Ok(ParsedSymbol::swap(base, quote, settle))
            }
            SymbolMarketType::Futures => {
                let settle = context.settle.ok_or_else(|| {
                    SymbolError::MissingComponent(
                        "settlement currency required for futures".to_string(),
                    )
                })?;
                let expiry = Self::extract_expiry_from_exchange_id(exchange_id)
                    .map_err(|e| SymbolError::InvalidFormat(e))?;
                Ok(ParsedSymbol::futures(base, quote, settle, expiry))
            }
        }
    }

    fn exchange_to_unified_inferred(&self, exchange_id: &str) -> String {
        Self::exchange_to_unified_inferred(exchange_id)
    }

    fn detect_market_type(&self, exchange_id: &str) -> Option<SymbolMarketType> {
        if Self::is_perpetual(exchange_id) {
            return Some(SymbolMarketType::Swap);
        }
        if Self::is_futures(exchange_id) {
            return Some(SymbolMarketType::Futures);
        }
        // Could be spot or linear swap, default to Spot
        Some(SymbolMarketType::Spot)
    }

    fn is_spot(&self, exchange_id: &str) -> bool {
        !BinanceSymbolConverter::is_perpetual(exchange_id)
            && !BinanceSymbolConverter::is_futures(exchange_id)
    }

    fn is_swap(&self, exchange_id: &str) -> bool {
        BinanceSymbolConverter::is_perpetual(exchange_id)
    }

    fn is_futures(&self, exchange_id: &str) -> bool {
        BinanceSymbolConverter::is_futures(exchange_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // to_exchange_id tests
    // ========================================================================

    #[test]
    fn test_spot_to_exchange_id() {
        let symbol = ParsedSymbol::spot("BTC", "USDT");
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&symbol), "BTCUSDT");
    }

    #[test]
    fn test_spot_to_exchange_id_various_pairs() {
        let eth_usdt = ParsedSymbol::spot("ETH", "USDT");
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&eth_usdt), "ETHUSDT");

        let btc_busd = ParsedSymbol::spot("BTC", "BUSD");
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&btc_busd), "BTCBUSD");

        let sol_btc = ParsedSymbol::spot("SOL", "BTC");
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&sol_btc), "SOLBTC");
    }

    #[test]
    fn test_linear_swap_to_exchange_id() {
        let symbol = ParsedSymbol::linear_swap("BTC", "USDT");
        assert_eq!(BinanceSymbolConverter::to_exchange_id(&symbol), "BTCUSDT");
    }

    #[test]
    fn test_inverse_swap_to_exchange_id() {
        let symbol = ParsedSymbol::inverse_swap("BTC", "USD");
        assert_eq!(
            BinanceSymbolConverter::to_exchange_id(&symbol),
            "BTCUSD_PERP"
        );
    }

    #[test]
    fn test_linear_futures_to_exchange_id() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures("BTC", "USDT", "USDT", expiry);
        assert_eq!(
            BinanceSymbolConverter::to_exchange_id(&symbol),
            "BTCUSDT_241231"
        );
    }

    #[test]
    fn test_inverse_futures_to_exchange_id() {
        let expiry = ExpiryDate::new(25, 3, 15).unwrap();
        let symbol = ParsedSymbol::futures("BTC", "USD", "BTC", expiry);
        assert_eq!(
            BinanceSymbolConverter::to_exchange_id(&symbol),
            "BTCUSD_250315"
        );
    }

    #[test]
    fn test_futures_date_padding() {
        let expiry = ExpiryDate::new(25, 1, 5).unwrap();
        let symbol = ParsedSymbol::futures("ETH", "USDT", "USDT", expiry);
        assert_eq!(
            BinanceSymbolConverter::to_exchange_id(&symbol),
            "ETHUSDT_250105"
        );
    }

    // ========================================================================
    // from_exchange_id tests
    // ========================================================================

    #[test]
    fn test_spot_from_exchange_id() {
        let parsed = BinanceSymbolConverter::from_exchange_id(
            "BTCUSDT",
            SymbolMarketType::Spot,
            None,
            "BTC",
            "USDT",
        )
        .unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert!(parsed.settle.is_none());
        assert!(parsed.is_spot());
    }

    #[test]
    fn test_swap_from_exchange_id() {
        let parsed = BinanceSymbolConverter::from_exchange_id(
            "BTCUSDT",
            SymbolMarketType::Swap,
            Some("USDT"),
            "BTC",
            "USDT",
        )
        .unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert_eq!(parsed.settle, Some("USDT".to_string()));
        assert!(parsed.is_swap());
    }

    #[test]
    fn test_futures_from_exchange_id() {
        let parsed = BinanceSymbolConverter::from_exchange_id(
            "BTCUSDT_241231",
            SymbolMarketType::Futures,
            Some("USDT"),
            "BTC",
            "USDT",
        )
        .unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert_eq!(parsed.settle, Some("USDT".to_string()));
        assert!(parsed.is_futures());
        let expiry = parsed.expiry.unwrap();
        assert_eq!(expiry.year, 24);
        assert_eq!(expiry.month, 12);
        assert_eq!(expiry.day, 31);
    }

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_is_perpetual() {
        assert!(BinanceSymbolConverter::is_perpetual("BTCUSD_PERP"));
        assert!(BinanceSymbolConverter::is_perpetual("ETHUSD_PERP"));
        assert!(!BinanceSymbolConverter::is_perpetual("BTCUSDT"));
        assert!(!BinanceSymbolConverter::is_perpetual("BTCUSDT_241231"));
    }

    #[test]
    fn test_is_futures() {
        assert!(BinanceSymbolConverter::is_futures("BTCUSDT_241231"));
        assert!(BinanceSymbolConverter::is_futures("ETHUSDT_250315"));
        assert!(!BinanceSymbolConverter::is_futures("BTCUSDT"));
        assert!(!BinanceSymbolConverter::is_futures("BTCUSD_PERP"));
    }

    // ========================================================================
    // exchange_to_unified_inferred tests
    // ========================================================================

    #[test]
    fn test_exchange_to_unified_inferred_spot() {
        assert_eq!(
            BinanceSymbolConverter::exchange_to_unified_inferred("BTCUSDT"),
            "BTC/USDT"
        );
        assert_eq!(
            BinanceSymbolConverter::exchange_to_unified_inferred("ETHBTC"),
            "ETH/BTC"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred_inverse_perpetual() {
        assert_eq!(
            BinanceSymbolConverter::exchange_to_unified_inferred("BTCUSD_PERP"),
            "BTC/USD:BTC"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred_futures() {
        assert_eq!(
            BinanceSymbolConverter::exchange_to_unified_inferred("BTCUSDT_241231"),
            "BTC/USDT:USDT-241231"
        );
    }
}

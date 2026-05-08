//! OKX symbol converter implementation
//!
//! This module provides conversion between unified CCXT symbols and OKX-specific
//! exchange IDs for spot, swap (perpetual), and futures markets.
//!
//! # OKX Symbol Formats
//!
//! | Market Type | Unified Format | OKX Format |
//! |-------------|----------------|------------|
//! | Spot | BTC/USDT | BTC-USDT |
//! | Linear Swap | BTC/USDT:USDT | BTC-USDT-SWAP |
//! | Inverse Swap | BTC/USD:BTC | BTC-USD-SWAP |
//! | Linear Futures | BTC/USDT:USDT-241231 | BTC-USDT-241231 |
//! | Inverse Futures | BTC/USD:BTC-241231 | BTC-USD-241231 |
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::types::common::symbol::{ParsedSymbol, ExpiryDate, SymbolMarketType};
//! use ccxt_exchanges::okx::core::symbol::OkxSymbolConverter;
//!
//! // Convert spot symbol
//! let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
//! assert_eq!(OkxSymbolConverter::to_exchange_id(&spot), "BTC-USDT");
//!
//! // Convert linear swap symbol
//! let swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
//! assert_eq!(OkxSymbolConverter::to_exchange_id(&swap), "BTC-USDT-SWAP");
//!
//! // Convert futures symbol
//! let expiry = ExpiryDate::new(24, 12, 31).unwrap();
//! let futures = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
//! assert_eq!(OkxSymbolConverter::to_exchange_id(&futures), "BTC-USDT-241231");
//! ```

use ccxt_core::symbol::{SymbolContext, SymbolConverter, SymbolError};
use ccxt_core::types::common::symbol::{ExpiryDate, ParsedSymbol, SymbolMarketType};

/// OKX symbol converter
///
/// Provides methods to convert between unified CCXT symbols and OKX-specific
/// exchange IDs.
pub struct OkxSymbolConverter;

impl OkxSymbolConverter {
    /// Convert a unified ParsedSymbol to OKX exchange ID
    ///
    /// # Arguments
    ///
    /// * `parsed` - The parsed unified symbol
    ///
    /// # Returns
    ///
    /// Returns the OKX-specific exchange ID string
    ///
    /// # Format Mapping
    ///
    /// - Spot: `BASE/QUOTE` → `BASE-QUOTE` (e.g., "BTC/USDT" → "BTC-USDT")
    /// - Linear Swap: `BASE/QUOTE:QUOTE` → `BASE-QUOTE-SWAP` (e.g., "BTC/USDT:USDT" → "BTC-USDT-SWAP")
    /// - Inverse Swap: `BASE/QUOTE:BASE` → `BASE-QUOTE-SWAP` (e.g., "BTC/USD:BTC" → "BTC-USD-SWAP")
    /// - Linear Futures: `BASE/QUOTE:QUOTE-YYMMDD` → `BASE-QUOTE-YYMMDD` (e.g., "BTC/USDT:USDT-241231" → "BTC-USDT-241231")
    /// - Inverse Futures: `BASE/QUOTE:BASE-YYMMDD` → `BASE-QUOTE-YYMMDD` (e.g., "BTC/USD:BTC-241231" → "BTC-USD-241231")
    pub fn to_exchange_id(parsed: &ParsedSymbol) -> String {
        let base = &parsed.base;
        let quote = &parsed.quote;

        match parsed.market_type() {
            SymbolMarketType::Spot => {
                // Spot: BTC-USDT
                format!("{}-{}", base, quote)
            }
            SymbolMarketType::Swap => {
                // Perpetual swap: BTC-USDT-SWAP (both linear and inverse use same format)
                format!("{}-{}-SWAP", base, quote)
            }
            SymbolMarketType::Futures => {
                // Futures with expiry date: BTC-USDT-241231
                if let Some(ref expiry) = parsed.expiry {
                    let date_str =
                        format!("{:02}{:02}{:02}", expiry.year, expiry.month, expiry.day);
                    format!("{}-{}-{}", base, quote, date_str)
                } else {
                    // Fallback if no expiry (shouldn't happen for valid futures)
                    format!("{}-{}", base, quote)
                }
            }
        }
    }

    /// Convert an OKX exchange ID to a unified ParsedSymbol
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The OKX-specific exchange ID
    /// * `market_type` - The market type (spot, swap, futures)
    /// * `settle` - Optional settlement currency (required for derivatives)
    ///
    /// # Returns
    ///
    /// Returns `Ok(ParsedSymbol)` if conversion succeeds, or `Err` if the format is invalid
    pub fn from_exchange_id(
        exchange_id: &str,
        market_type: SymbolMarketType,
        settle: Option<&str>,
    ) -> Result<ParsedSymbol, String> {
        let parts: Vec<&str> = exchange_id.split('-').collect();

        if parts.len() < 2 {
            return Err(format!("Invalid OKX exchange ID format: {}", exchange_id));
        }

        let base = parts[0].to_uppercase();
        let quote = parts[1].to_uppercase();

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

                // Extract expiry date from exchange_id (format: BTC-USDT-241231)
                let expiry = Self::extract_expiry_from_exchange_id(exchange_id)?;
                Ok(ParsedSymbol::futures(base, quote, settle, expiry))
            }
        }
    }

    /// Extract expiry date from OKX exchange ID
    ///
    /// OKX futures format: BTC-USDT-241231
    fn extract_expiry_from_exchange_id(exchange_id: &str) -> Result<ExpiryDate, String> {
        let parts: Vec<&str> = exchange_id.split('-').collect();

        // Futures should have 3 parts: BASE-QUOTE-YYMMDD
        if parts.len() >= 3 {
            let date_part = parts[2];

            // Check if it's a date (6 digits) and not "SWAP"
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

    /// Check if an OKX exchange ID represents a perpetual swap
    pub fn is_swap(exchange_id: &str) -> bool {
        exchange_id.ends_with("-SWAP")
    }

    /// Check if an OKX exchange ID represents a futures contract (with expiry)
    pub fn is_futures(exchange_id: &str) -> bool {
        let parts: Vec<&str> = exchange_id.split('-').collect();
        if parts.len() >= 3 {
            let suffix = parts[2];
            suffix.len() == 6 && suffix.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    }

    /// Check if an OKX exchange ID represents a spot market
    pub fn is_spot(exchange_id: &str) -> bool {
        let parts: Vec<&str> = exchange_id.split('-').collect();
        parts.len() == 2
    }

    /// Convert an OKX exchange ID to a unified symbol (inferred, for WebSocket scenarios)
    ///
    /// This method attempts to convert an exchange ID back to unified format
    /// without requiring additional context.
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The OKX-specific exchange ID (e.g., "BTC-USDT", "BTC-USDT-SWAP")
    ///
    /// # Returns
    ///
    /// The inferred unified symbol:
    /// - "BTC-USDT" → "BTC/USDT" (spot)
    /// - "BTC-USDT-SWAP" → "BTC/USDT:USDT" (assumes linear swap)
    /// - "BTC-USD-SWAP" → "BTC/USD:BTC" (assumes inverse swap)
    /// - "BTC-USDT-241231" → "BTC/USDT:USDT-241231" (futures)
    ///
    /// # Note
    ///
    /// This method makes best-effort inference. For precise conversion,
    /// use `from_exchange_id()` with proper context.
    pub fn exchange_to_unified_inferred(exchange_id: &str) -> String {
        let parts: Vec<&str> = exchange_id.split('-').collect();

        match parts.len() {
            2 => {
                // Spot: BTC-USDT → BTC/USDT
                format!("{}/{}", parts[0], parts[1])
            }
            3 => {
                if parts[2] == "SWAP" {
                    // Swap: need to determine linear vs inverse
                    // If quote is USD (not USDT/USDC), assume inverse
                    if parts[1] == "USD" {
                        // Inverse swap: BTC-USD-SWAP → BTC/USD:BTC
                        format!("{}/USD:{}", parts[0], parts[0])
                    } else {
                        // Linear swap: BTC-USDT-SWAP → BTC/USDT:USDT
                        format!("{}/{}:{}", parts[0], parts[1], parts[1])
                    }
                } else {
                    // Futures: BTC-USDT-241231 → BTC/USDT:USDT-241231
                    // Assume linear futures (settle = quote)
                    format!("{}/{}:{}-{}", parts[0], parts[1], parts[1], parts[2])
                }
            }
            _ => {
                // Fallback: return as-is
                exchange_id.to_string()
            }
        }
    }
}

/// Implement SymbolConverter trait for OKX
impl SymbolConverter for OkxSymbolConverter {
    fn to_exchange_id(&self, symbol: &ParsedSymbol) -> String {
        Self::to_exchange_id(symbol)
    }

    fn from_exchange_id(
        &self,
        exchange_id: &str,
        context: SymbolContext,
    ) -> Result<ParsedSymbol, SymbolError> {
        Self::from_exchange_id(exchange_id, context.market_type, context.settle.as_deref())
            .map_err(|e| SymbolError::InvalidFormat(e))
    }

    fn exchange_to_unified_inferred(&self, exchange_id: &str) -> String {
        Self::exchange_to_unified_inferred(exchange_id)
    }

    fn detect_market_type(&self, exchange_id: &str) -> Option<SymbolMarketType> {
        if OkxSymbolConverter::is_swap(exchange_id) {
            return Some(SymbolMarketType::Swap);
        }
        if OkxSymbolConverter::is_futures(exchange_id) {
            return Some(SymbolMarketType::Futures);
        }
        if OkxSymbolConverter::is_spot(exchange_id) {
            return Some(SymbolMarketType::Spot);
        }
        None
    }

    fn is_spot(&self, exchange_id: &str) -> bool {
        OkxSymbolConverter::is_spot(exchange_id)
    }

    fn is_swap(&self, exchange_id: &str) -> bool {
        OkxSymbolConverter::is_swap(exchange_id)
    }

    fn is_futures(&self, exchange_id: &str) -> bool {
        OkxSymbolConverter::is_futures(exchange_id)
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
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        assert_eq!(OkxSymbolConverter::to_exchange_id(&symbol), "BTC-USDT");
    }

    #[test]
    fn test_spot_to_exchange_id_various_pairs() {
        let eth_usdt = ParsedSymbol::spot("ETH".to_string(), "USDT".to_string());
        assert_eq!(OkxSymbolConverter::to_exchange_id(&eth_usdt), "ETH-USDT");

        let btc_usdc = ParsedSymbol::spot("BTC".to_string(), "USDC".to_string());
        assert_eq!(OkxSymbolConverter::to_exchange_id(&btc_usdc), "BTC-USDC");

        let sol_btc = ParsedSymbol::spot("SOL".to_string(), "BTC".to_string());
        assert_eq!(OkxSymbolConverter::to_exchange_id(&sol_btc), "SOL-BTC");
    }

    #[test]
    fn test_linear_swap_to_exchange_id() {
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        assert_eq!(OkxSymbolConverter::to_exchange_id(&symbol), "BTC-USDT-SWAP");
    }

    #[test]
    fn test_inverse_swap_to_exchange_id() {
        let symbol = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        assert_eq!(OkxSymbolConverter::to_exchange_id(&symbol), "BTC-USD-SWAP");
    }

    #[test]
    fn test_linear_futures_to_exchange_id() {
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        assert_eq!(
            OkxSymbolConverter::to_exchange_id(&symbol),
            "BTC-USDT-241231"
        );
    }

    #[test]
    fn test_inverse_futures_to_exchange_id() {
        let expiry = ExpiryDate::new(25, 3, 15).unwrap();
        let symbol = ParsedSymbol::futures(
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            expiry,
        );
        assert_eq!(
            OkxSymbolConverter::to_exchange_id(&symbol),
            "BTC-USD-250315"
        );
    }

    #[test]
    fn test_futures_date_padding() {
        let expiry = ExpiryDate::new(25, 1, 5).unwrap();
        let symbol = ParsedSymbol::futures(
            "ETH".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        assert_eq!(
            OkxSymbolConverter::to_exchange_id(&symbol),
            "ETH-USDT-250105"
        );
    }

    // ========================================================================
    // from_exchange_id tests
    // ========================================================================

    #[test]
    fn test_spot_from_exchange_id() {
        let parsed =
            OkxSymbolConverter::from_exchange_id("BTC-USDT", SymbolMarketType::Spot, None).unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert!(parsed.settle.is_none());
        assert!(parsed.is_spot());
    }

    #[test]
    fn test_swap_from_exchange_id() {
        let parsed = OkxSymbolConverter::from_exchange_id(
            "BTC-USDT-SWAP",
            SymbolMarketType::Swap,
            Some("USDT"),
        )
        .unwrap();
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
        assert_eq!(parsed.settle, Some("USDT".to_string()));
        assert!(parsed.is_swap());
    }

    #[test]
    fn test_futures_from_exchange_id() {
        let parsed = OkxSymbolConverter::from_exchange_id(
            "BTC-USDT-241231",
            SymbolMarketType::Futures,
            Some("USDT"),
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
    fn test_is_swap() {
        assert!(OkxSymbolConverter::is_swap("BTC-USDT-SWAP"));
        assert!(OkxSymbolConverter::is_swap("ETH-USD-SWAP"));
        assert!(!OkxSymbolConverter::is_swap("BTC-USDT"));
        assert!(!OkxSymbolConverter::is_swap("BTC-USDT-241231"));
    }

    #[test]
    fn test_is_futures() {
        assert!(OkxSymbolConverter::is_futures("BTC-USDT-241231"));
        assert!(OkxSymbolConverter::is_futures("ETH-USDT-250315"));
        assert!(!OkxSymbolConverter::is_futures("BTC-USDT"));
        assert!(!OkxSymbolConverter::is_futures("BTC-USDT-SWAP"));
    }

    #[test]
    fn test_is_spot() {
        assert!(OkxSymbolConverter::is_spot("BTC-USDT"));
        assert!(OkxSymbolConverter::is_spot("ETH-BTC"));
        assert!(!OkxSymbolConverter::is_spot("BTC-USDT-SWAP"));
        assert!(!OkxSymbolConverter::is_spot("BTC-USDT-241231"));
    }

    // ========================================================================
    // exchange_to_unified_inferred tests
    // ========================================================================

    #[test]
    fn test_exchange_to_unified_inferred_spot() {
        assert_eq!(
            OkxSymbolConverter::exchange_to_unified_inferred("BTC-USDT"),
            "BTC/USDT"
        );
        assert_eq!(
            OkxSymbolConverter::exchange_to_unified_inferred("ETH-BTC"),
            "ETH/BTC"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred_linear_swap() {
        assert_eq!(
            OkxSymbolConverter::exchange_to_unified_inferred("BTC-USDT-SWAP"),
            "BTC/USDT:USDT"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred_inverse_swap() {
        assert_eq!(
            OkxSymbolConverter::exchange_to_unified_inferred("BTC-USD-SWAP"),
            "BTC/USD:BTC"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred_futures() {
        assert_eq!(
            OkxSymbolConverter::exchange_to_unified_inferred("BTC-USDT-241231"),
            "BTC/USDT:USDT-241231"
        );
    }
}

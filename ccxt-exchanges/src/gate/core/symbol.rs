//! Gate.io symbol converter implementation
//!
//! This module provides conversion between unified CCXT symbols and Gate.io-specific
//! exchange IDs for spot, swap (perpetual), and futures markets.
//!
//! # Gate.io Symbol Formats
//!
//! | Market Type | Unified Format | Gate Format | Example |
//! |-------------|----------------|-------------|---------|
//! | Spot | BASE/QUOTE | BASE_QUOTE | BTC/USDT → BTC_USDT |
//! | Linear Swap | BASE/QUOTE:QUOTE | BASE_QUOTE | BTC/USDT:USDT → BTC_USDT |
//! | USDC Swap | BASE/QUOTE:USDC | BASE_USDC | BTC/USDT:USDC → BTC_USDC |
//! | Inverse Swap | BASE/QUOTE:BASE | BASE_USD | BTC/USD:BTC → BTC_USD |
//! | Futures | BASE/QUOTE:SETTLE-YYMMDD | BASE_QUOTE_YYYYMMDD | BTC/USDT:USDT-240628 → BTC_USDT_20240628 |
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::types::common::symbol::{ParsedSymbol, ExpiryDate, SymbolMarketType};
//! use ccxt_exchanges::gate::core::symbol::GateSymbolConverter;
//!
//! // Convert spot symbol
//! let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
//! assert_eq!(GateSymbolConverter.to_exchange_id(&spot), "BTC_USDT");
//!
//! // Convert linear swap symbol
//! let swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
//! assert_eq!(GateSymbolConverter.to_exchange_id(&swap), "BTC_USDT");
//!
//! // Convert futures symbol
//! let expiry = ExpiryDate::new(24, 6, 28).unwrap();
//! let futures = ParsedSymbol::futures("BTC".to_string(), "USDT".to_string(), "USDT".to_string(), expiry);
//! assert_eq!(GateSymbolConverter.to_exchange_id(&futures), "BTC_USDT_20240628");
//! ```

use ccxt_core::symbol::{SymbolContext, SymbolConverter, SymbolError};
use ccxt_core::types::common::symbol::{ExpiryDate, ParsedSymbol, SymbolMarketType};

/// Gate.io symbol converter
///
/// Provides methods to convert between unified CCXT symbols and Gate.io-specific
/// exchange IDs.
#[derive(Debug, Clone)]
pub struct GateSymbolConverter;

impl SymbolConverter for GateSymbolConverter {
    /// Convert a unified ParsedSymbol to Gate.io exchange ID
    ///
    /// # Format Mapping
    ///
    /// - Spot: `BASE/QUOTE` → `BASE_QUOTE` (e.g., "BTC/USDT" → "BTC_USDT")
    /// - Linear Swap: `BASE/QUOTE:QUOTE` → `BASE_QUOTE` (e.g., "BTC/USDT:USDT" → "BTC_USDT")
    /// - USDC Swap: `BASE/QUOTE:USDC` → `BASE_USDC` (e.g., "BTC/USDT:USDC" → "BTC_USDC")
    /// - Inverse Swap: `BASE/QUOTE:BASE` → `BASE_USD` (e.g., "BTC/USD:BTC" → "BTC_USD")
    /// - Futures: `BASE/QUOTE:SETTLE-YYMMDD` → `BASE_QUOTE_YYYYMMDD` (e.g., "BTC/USDT:USDT-240628" → "BTC_USDT_20240628")
    fn to_exchange_id(&self, symbol: &ParsedSymbol) -> String {
        let base = &symbol.base;
        let quote = &symbol.quote;

        match symbol.market_type() {
            SymbolMarketType::Spot => {
                // Spot: BTC_USDT
                format!("{}_{}", base, quote)
            }
            SymbolMarketType::Swap => {
                // Check if this is an inverse swap (settle == base)
                if let Some(ref settle) = symbol.settle {
                    if settle == &symbol.base {
                        // Inverse: BTC_USD (always use USD for inverse contracts)
                        format!("{}_USD", base)
                    } else {
                        // Linear/USDC: BTC_USDT or BTC_USDC
                        format!("{}_{}", base, settle)
                    }
                } else {
                    // Default to quote currency
                    format!("{}_{}", base, quote)
                }
            }
            SymbolMarketType::Futures => {
                // Futures with expiry date: BTC_USDT_20240628
                if let Some(ref expiry) = symbol.expiry {
                    // Convert 2-digit year to 4-digit year (24 → 2024)
                    let full_year = expiry.year as u16 + 2000;
                    let date_str = format!("{:04}{:02}{:02}", full_year, expiry.month, expiry.day);

                    if let Some(ref settle) = symbol.settle {
                        if settle == "USD" {
                            format!("{}_USD_{}", base, date_str)
                        } else {
                            format!("{}_{}_{}", base, settle, date_str)
                        }
                    } else {
                        format!("{}_{}_{}", base, quote, date_str)
                    }
                } else {
                    // Fallback if no expiry
                    format!("{}_{}", base, quote)
                }
            }
        }
    }

    /// Convert a Gate.io exchange ID to a unified ParsedSymbol
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The Gate.io-specific exchange ID
    /// * `context` - Context information (market type, settle, base, quote)
    ///
    /// # Returns
    ///
    /// Returns `Ok(ParsedSymbol)` if conversion succeeds, or `Err` if the format is invalid
    fn from_exchange_id(
        &self,
        exchange_id: &str,
        context: SymbolContext,
    ) -> Result<ParsedSymbol, SymbolError> {
        let parts: Vec<&str> = exchange_id.split('_').collect();

        if parts.len() < 2 {
            return Err(SymbolError::InvalidFormat(format!(
                "Invalid Gate symbol: {}",
                exchange_id
            )));
        }

        let base = parts[0].to_uppercase();
        let quote = parts[1].to_uppercase();

        match context.market_type {
            SymbolMarketType::Spot => {
                // BTC_USDT → BTC/USDT
                Ok(ParsedSymbol::spot(base, quote))
            }
            SymbolMarketType::Swap => {
                // BTC_USDT + context.settle=USDT → BTC/USDT:USDT
                let settle = context.settle.ok_or_else(|| {
                    SymbolError::MissingComponent("settle currency required for swap".to_string())
                })?;
                Ok(ParsedSymbol::swap(base, quote, settle))
            }
            SymbolMarketType::Futures => {
                // BTC_USDT_20240628 + context.settle=USDT → BTC/USDT:USDT-240628
                let settle = context.settle.ok_or_else(|| {
                    SymbolError::MissingComponent(
                        "settle currency required for futures".to_string(),
                    )
                })?;

                // Extract date part (3rd part)
                if parts.len() >= 3 {
                    let date_str = parts[2];
                    let expiry = Self::parse_expiry_4digit(date_str)?;
                    Ok(ParsedSymbol::futures(base, quote, settle, expiry))
                } else {
                    Err(SymbolError::MissingDateSuffix)
                }
            }
        }
    }

    /// Convert a Gate.io exchange ID to unified format (inferred)
    ///
    /// This method makes a best-effort conversion without requiring context.
    /// Used primarily for WebSocket message parsing where context is not available.
    ///
    /// # Inference Rules
    ///
    /// - BTC_USDT → BTC/USDT (default to spot)
    /// - BTC_USDT_20240628 → BTC/USDT:USDT-240628 (has date → futures)
    /// - BTC_USD → BTC/USD:BTC (quote=USD → inverse)
    fn exchange_to_unified_inferred(&self, exchange_id: &str) -> String {
        let parts: Vec<&str> = exchange_id.split('_').collect();

        if parts.len() < 2 {
            return exchange_id.to_string();
        }

        let base = parts[0];
        let quote = parts[1];

        // Check if it's a futures contract (3 parts: BASE_QUOTE_YYYYMMDD)
        if parts.len() == 3 && parts[2].len() == 8 {
            // BTC_USDT_20240628 → BTC/USDT:USDT-240628
            let date_str = parts[2];
            // Convert 4-digit year to 2-digit year (2024 → 24)
            let year: u8 = date_str[2..4].parse().unwrap_or(0);
            let month: u8 = date_str[4..6].parse().unwrap_or(0);
            let day: u8 = date_str[6..8].parse().unwrap_or(0);

            format!(
                "{}/{}:{}-{:02}{:02}{:02}",
                base, quote, quote, year, month, day
            )
        } else if quote == "USD" {
            // BTC_USD → BTC/USD:BTC (inverse perpetual)
            format!("{}/{}:{}", base, quote, base)
        } else {
            // BTC_USDT → BTC/USDT (default to spot)
            format!("{}/{}", base, quote)
        }
    }

    /// Detect market type from exchange-specific ID
    ///
    /// # Detection Rules
    ///
    /// - 3 parts and 3rd part is 8 digits → Futures
    /// - 2 parts and quote=USD → Swap (inverse)
    /// - 2 parts → Spot (default, ambiguous with linear swap)
    fn detect_market_type(&self, exchange_id: &str) -> Option<SymbolMarketType> {
        let parts: Vec<&str> = exchange_id.split('_').collect();

        if parts.len() == 3 && parts[2].len() == 8 && parts[2].chars().all(|c| c.is_ascii_digit()) {
            // BTC_USDT_20240628 → Futures
            Some(SymbolMarketType::Futures)
        } else if parts.len() == 2 && parts[1] == "USD" {
            // BTC_USD → Swap (inverse)
            Some(SymbolMarketType::Swap)
        } else if parts.len() == 2 {
            // BTC_USDT → Spot (default, ambiguous)
            Some(SymbolMarketType::Spot)
        } else {
            None
        }
    }
}

impl GateSymbolConverter {
    /// Parse a 4-digit year date string (Gate.io specific format)
    ///
    /// # Example
    ///
    /// - "20240628" → ExpiryDate { year: 24, month: 6, day: 28 }
    fn parse_expiry_4digit(date_str: &str) -> Result<ExpiryDate, SymbolError> {
        if date_str.len() != 8 {
            return Err(SymbolError::InvalidFormat(format!(
                "Gate expiry date must be 8 digits (YYYYMMDD), got: {}",
                date_str
            )));
        }

        let year: u8 = date_str[2..4].parse().map_err(|_| {
            SymbolError::InvalidFormat(format!("Invalid year in expiry: {}", date_str))
        })?;

        let month: u8 = date_str[4..6].parse().map_err(|_| {
            SymbolError::InvalidFormat(format!("Invalid month in expiry: {}", date_str))
        })?;

        let day: u8 = date_str[6..8].parse().map_err(|_| {
            SymbolError::InvalidFormat(format!("Invalid day in expiry: {}", date_str))
        })?;

        ExpiryDate::new(year, month, day)
            .map_err(|_| SymbolError::InvalidFormat(format!("Invalid date: {}", date_str)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spot_to_exchange_id() {
        let symbol = ParsedSymbol::spot("BTC", "USDT");
        assert_eq!(GateSymbolConverter.to_exchange_id(&symbol), "BTC_USDT");
    }

    #[test]
    fn test_linear_swap_to_exchange_id() {
        let symbol = ParsedSymbol::linear_swap("BTC", "USDT");
        assert_eq!(GateSymbolConverter.to_exchange_id(&symbol), "BTC_USDT");
    }

    #[test]
    fn test_usdc_swap_to_exchange_id() {
        let symbol = ParsedSymbol::swap("BTC", "USDT", "USDC");
        assert_eq!(GateSymbolConverter.to_exchange_id(&symbol), "BTC_USDC");
    }

    #[test]
    fn test_inverse_swap_to_exchange_id() {
        let symbol = ParsedSymbol::inverse_swap("BTC", "USD");
        assert_eq!(GateSymbolConverter.to_exchange_id(&symbol), "BTC_USD");
    }

    #[test]
    fn test_futures_to_exchange_id() {
        let expiry = ExpiryDate::new(24, 6, 28).unwrap();
        let symbol = ParsedSymbol::futures("BTC", "USDT", "USDT", expiry);
        assert_eq!(
            GateSymbolConverter.to_exchange_id(&symbol),
            "BTC_USDT_20240628"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred() {
        // Spot
        assert_eq!(
            GateSymbolConverter.exchange_to_unified_inferred("BTC_USDT"),
            "BTC/USDT"
        );

        // Inverse perpetual
        assert_eq!(
            GateSymbolConverter.exchange_to_unified_inferred("BTC_USD"),
            "BTC/USD:BTC"
        );

        // Futures
        assert_eq!(
            GateSymbolConverter.exchange_to_unified_inferred("BTC_USDT_20240628"),
            "BTC/USDT:USDT-240628"
        );
    }

    #[test]
    fn test_detect_market_type() {
        assert_eq!(
            GateSymbolConverter.detect_market_type("BTC_USDT"),
            Some(SymbolMarketType::Spot)
        );
        assert_eq!(
            GateSymbolConverter.detect_market_type("BTC_USD"),
            Some(SymbolMarketType::Swap)
        );
        assert_eq!(
            GateSymbolConverter.detect_market_type("BTC_USDT_20240628"),
            Some(SymbolMarketType::Futures)
        );
    }
}

//! Bitget symbol converter implementation.
//!
//! This module provides conversion between unified CCXT symbols and Bitget-specific
//! exchange IDs for spot, swap (perpetual), and futures markets.
//!
//! # Bitget Symbol Formats
//!
//! | Market Type | Unified Format | Bitget Format | Product Type |
//! |-------------|----------------|---------------|--------------|
//! | Spot | BTC/USDT | BTCUSDT | spot |
//! | Linear Swap | BTC/USDT:USDT | BTCUSDT | USDT-FUTURES |
//! | Inverse Swap | BTC/USD:BTC | BTCUSD | COIN-FUTURES |
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::bitget::core::symbol::BitgetSymbolConverter;
//!
//! assert_eq!(BitgetSymbolConverter::unified_to_exchange("BTC/USDT"), "BTCUSDT");
//! assert_eq!(BitgetSymbolConverter::unified_to_exchange("BTC/USDT:USDT"), "BTCUSDT");
//! assert_eq!(BitgetSymbolConverter::product_type_from_symbol("BTC/USDT:USDT"), "USDT-FUTURES");
//! assert_eq!(BitgetSymbolConverter::product_type_from_symbol("BTC/USDT"), "spot");
//! ```

use ccxt_core::symbol::{SymbolContext, SymbolConverter, SymbolError};
use ccxt_core::types::common::symbol::{ParsedSymbol, SymbolMarketType};

/// Bitget symbol converter.
///
/// Provides methods to convert between unified CCXT symbols and Bitget-specific
/// exchange IDs, and to determine the appropriate product type.
pub struct BitgetSymbolConverter;

impl BitgetSymbolConverter {
    /// Convert a unified CCXT symbol to Bitget exchange ID.
    ///
    /// Strips the settlement currency suffix and removes separators:
    /// - "BTC/USDT" → "BTCUSDT"
    /// - "BTC/USDT:USDT" → "BTCUSDT"
    /// - "BTC/USD:BTC" → "BTCUSD"
    /// - "BTC/USDT:USDT-241231" → "BTCUSDT"
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

    /// Determine the Bitget product type from a unified symbol.
    ///
    /// Returns:
    /// - "USDT-FUTURES" for linear perpetual/futures (e.g., "BTC/USDT:USDT")
    /// - "COIN-FUTURES" for inverse perpetual/futures (e.g., "BTC/USD:BTC")
    /// - "spot" for spot markets (e.g., "BTC/USDT")
    pub fn product_type_from_symbol(symbol: &str) -> &'static str {
        if let Some(pos) = symbol.find(':') {
            let settle_part = &symbol[pos + 1..];
            // Strip expiry date if present
            let settle = if let Some(dash_pos) = settle_part.find('-') {
                &settle_part[..dash_pos]
            } else {
                settle_part
            };

            // Extract quote currency
            let base_quote = &symbol[..pos];
            let quote = if let Some(slash_pos) = base_quote.find('/') {
                &base_quote[slash_pos + 1..]
            } else {
                ""
            };

            // If settle == quote, it's linear (USDT-FUTURES)
            // If settle != quote (settle == base), it's inverse (COIN-FUTURES)
            if settle == quote {
                "USDT-FUTURES"
            } else {
                "COIN-FUTURES"
            }
        } else {
            "spot"
        }
    }

    /// Check if a symbol is a contract (has settlement currency).
    pub fn is_contract(symbol: &str) -> bool {
        symbol.contains(':')
    }

    /// Check if a symbol is a spot market.
    pub fn is_spot(symbol: &str) -> bool {
        !symbol.contains(':')
    }

    /// Get WebSocket instType from a unified symbol.
    ///
    /// This returns the instType value used in Bitget V3 UTA WebSocket API:
    /// - "spot" for spot markets (e.g., "BTC/USDT")
    /// - "usdt-futures" for linear perpetual/futures (e.g., "BTC/USDT:USDT")
    /// - "coin-futures" for inverse perpetual/futures (e.g., "BTC/USD:BTC")
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol (can be either unified format or exchange format)
    ///
    /// # Example
    ///
    /// ```
    /// use ccxt_exchanges::bitget::core::symbol::BitgetSymbolConverter;
    ///
    /// assert_eq!(BitgetSymbolConverter::ws_inst_type("BTC/USDT"), "spot");
    /// assert_eq!(BitgetSymbolConverter::ws_inst_type("BTC/USDT:USDT"), "usdt-futures");
    /// assert_eq!(BitgetSymbolConverter::ws_inst_type("BTC/USD:BTC"), "coin-futures");
    /// // Also works with exchange format (no colon = spot)
    /// assert_eq!(BitgetSymbolConverter::ws_inst_type("BTCUSDT"), "spot");
    /// ```
    pub fn ws_inst_type(symbol: &str) -> &'static str {
        // If symbol contains '/', it's unified format
        if symbol.contains('/') {
            let product_type = Self::product_type_from_symbol(symbol);
            match product_type {
                "spot" => "spot",                 // V3 UTA: lowercase
                "USDT-FUTURES" => "usdt-futures", // V3 UTA: lowercase
                "COIN-FUTURES" => "coin-futures", // V3 UTA: lowercase
                other => other,
            }
        } else {
            // Exchange format (no '/'), assume spot unless we have market context
            // The caller should provide unified symbol for proper detection
            "spot" // V3 UTA: lowercase
        }
    }

    /// Convert a Bitget exchange ID back to a rough unified symbol hint.
    ///
    /// This is a best-effort conversion:
    /// - "BTCUSDT" with product_type "USDT-FUTURES" → "BTC/USDT:USDT"
    /// - "BTCUSD" with product_type "COIN-FUTURES" → "BTC/USD:BTC"
    /// - "BTCUSDT" with product_type "spot" → "BTC/USDT"
    pub fn exchange_to_unified_hint(exchange_id: &str, product_type: &str) -> String {
        // Try to split the exchange ID into base and quote
        // Common quote currencies in order of length (longest first to avoid partial matches)
        let quote_currencies = ["USDT", "USDC", "USD", "BTC", "ETH"];

        for quote in &quote_currencies {
            if exchange_id.ends_with(quote) && exchange_id.len() > quote.len() {
                let base = &exchange_id[..exchange_id.len() - quote.len()];
                return match product_type {
                    "USDT-FUTURES" | "usdt-futures" => {
                        format!("{}/{}:{}", base, quote, quote)
                    }
                    "COIN-FUTURES" | "coin-futures" => {
                        format!("{}/{}:{}", base, quote, base)
                    }
                    _ => {
                        format!("{}/{}", base, quote)
                    }
                };
            }
        }

        // Fallback: return as-is
        exchange_id.to_string()
    }

    /// Convert a Bitget exchange ID to a unified symbol (inferred, for WebSocket scenarios)
    ///
    /// This method attempts to convert an exchange ID back to unified format
    /// without requiring additional context.
    ///
    /// # Arguments
    ///
    /// * `exchange_id` - The Bitget-specific exchange ID (e.g., "BTCUSDT", "BTCUSD")
    ///
    /// # Returns
    ///
    /// The inferred unified symbol:
    /// - "BTCUSDT" → "BTC/USDT" (assumes spot or linear swap)
    /// - "BTCUSD" → "BTC/USD:BTC" (assumes inverse swap)
    ///
    /// # Note
    ///
    /// This method makes best-effort inference. For precise conversion,
    /// use `exchange_to_unified_hint()` with proper product_type.
    pub fn exchange_to_unified_inferred(exchange_id: &str) -> String {
        // Try to split the exchange ID into base and quote
        let quote_currencies = ["USDT", "USDC", "USD", "BTC", "ETH"];

        for quote in &quote_currencies {
            if exchange_id.ends_with(quote) && exchange_id.len() > quote.len() {
                let base = &exchange_id[..exchange_id.len() - quote.len()];
                // For USD (not USDT/USDC), assume inverse (coin-margined)
                if *quote == "USD" && !exchange_id.contains("USDT") && !exchange_id.contains("USDC")
                {
                    return format!("{}/USD:{}", base, base);
                }
                // For USDT/USDC/BTC/ETH, assume linear or spot
                return format!("{}/{}", base, quote);
            }
        }

        // Fallback: return as-is
        exchange_id.to_string()
    }
}

/// Implement SymbolConverter trait for Bitget
impl SymbolConverter for BitgetSymbolConverter {
    fn to_exchange_id(&self, symbol: &ParsedSymbol) -> String {
        // Bitget uses same format for all: BASEQUOTE (no separators)
        format!("{}{}", symbol.base, symbol.quote)
    }

    fn from_exchange_id(
        &self,
        _exchange_id: &str,
        context: SymbolContext,
    ) -> Result<ParsedSymbol, SymbolError> {
        let base = context.base.ok_or_else(|| {
            SymbolError::MissingComponent("base currency required for Bitget".to_string())
        })?;
        let quote = context.quote.ok_or_else(|| {
            SymbolError::MissingComponent("quote currency required for Bitget".to_string())
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
                let _settle = context.settle.ok_or_else(|| {
                    SymbolError::MissingComponent(
                        "settlement currency required for futures".to_string(),
                    )
                })?;
                // Bitget futures expiry would need to be extracted from context or exchange_id
                // For now, return error as Bitget futures format is more complex
                Err(SymbolError::InvalidFormat(
                    "Bitget futures conversion not fully implemented".to_string(),
                ))
            }
        }
    }

    fn exchange_to_unified_inferred(&self, exchange_id: &str) -> String {
        Self::exchange_to_unified_inferred(exchange_id)
    }

    fn detect_market_type(&self, _exchange_id: &str) -> Option<SymbolMarketType> {
        // Bitget uses same format for spot and swap, cannot detect from ID alone
        // Need productType context
        None
    }

    fn is_spot(&self, symbol: &str) -> bool {
        Self::is_spot(symbol)
    }

    fn is_swap(&self, symbol: &str) -> bool {
        Self::is_contract(symbol)
    }

    fn is_futures(&self, _exchange_id: &str) -> bool {
        // Bitget futures format is similar to swap
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // unified_to_exchange tests
    // ========================================================================

    #[test]
    fn test_spot_to_exchange() {
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("BTC/USDT"),
            "BTCUSDT"
        );
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("ETH/USDT"),
            "ETHUSDT"
        );
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("SOL/BTC"),
            "SOLBTC"
        );
    }

    #[test]
    fn test_linear_swap_to_exchange() {
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("BTC/USDT:USDT"),
            "BTCUSDT"
        );
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("ETH/USDT:USDT"),
            "ETHUSDT"
        );
    }

    #[test]
    fn test_inverse_swap_to_exchange() {
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("BTC/USD:BTC"),
            "BTCUSD"
        );
    }

    #[test]
    fn test_futures_with_expiry_to_exchange() {
        assert_eq!(
            BitgetSymbolConverter::unified_to_exchange("BTC/USDT:USDT-241231"),
            "BTCUSDT"
        );
    }

    // ========================================================================
    // product_type_from_symbol tests
    // ========================================================================

    #[test]
    fn test_product_type_spot() {
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("BTC/USDT"),
            "spot"
        );
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("ETH/BTC"),
            "spot"
        );
    }

    #[test]
    fn test_product_type_linear() {
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("BTC/USDT:USDT"),
            "USDT-FUTURES"
        );
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("ETH/USDT:USDT"),
            "USDT-FUTURES"
        );
    }

    #[test]
    fn test_product_type_inverse() {
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("BTC/USD:BTC"),
            "COIN-FUTURES"
        );
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("ETH/USD:ETH"),
            "COIN-FUTURES"
        );
    }

    #[test]
    fn test_product_type_futures_with_expiry() {
        assert_eq!(
            BitgetSymbolConverter::product_type_from_symbol("BTC/USDT:USDT-241231"),
            "USDT-FUTURES"
        );
    }

    // ========================================================================
    // Helper function tests
    // ========================================================================

    #[test]
    fn test_is_contract() {
        assert!(BitgetSymbolConverter::is_contract("BTC/USDT:USDT"));
        assert!(BitgetSymbolConverter::is_contract("BTC/USD:BTC"));
        assert!(!BitgetSymbolConverter::is_contract("BTC/USDT"));
    }

    #[test]
    fn test_is_spot() {
        assert!(BitgetSymbolConverter::is_spot("BTC/USDT"));
        assert!(!BitgetSymbolConverter::is_spot("BTC/USDT:USDT"));
    }

    // ========================================================================
    // exchange_to_unified_hint tests
    // ========================================================================

    #[test]
    fn test_exchange_to_unified_spot() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_hint("BTCUSDT", "spot"),
            "BTC/USDT"
        );
    }

    #[test]
    fn test_exchange_to_unified_linear() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_hint("BTCUSDT", "USDT-FUTURES"),
            "BTC/USDT:USDT"
        );
    }

    #[test]
    fn test_exchange_to_unified_inverse() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_hint("BTCUSD", "COIN-FUTURES"),
            "BTC/USD:BTC"
        );
    }

    // ========================================================================
    // ws_inst_type tests
    // ========================================================================

    #[test]
    fn test_ws_inst_type_spot() {
        assert_eq!(BitgetSymbolConverter::ws_inst_type("BTC/USDT"), "spot");
        assert_eq!(BitgetSymbolConverter::ws_inst_type("ETH/BTC"), "spot");
    }

    #[test]
    fn test_ws_inst_type_linear_futures() {
        assert_eq!(
            BitgetSymbolConverter::ws_inst_type("BTC/USDT:USDT"),
            "usdt-futures"
        );
        assert_eq!(
            BitgetSymbolConverter::ws_inst_type("ETH/USDT:USDT"),
            "usdt-futures"
        );
    }

    #[test]
    fn test_ws_inst_type_coin_futures() {
        assert_eq!(
            BitgetSymbolConverter::ws_inst_type("BTC/USD:BTC"),
            "coin-futures"
        );
        assert_eq!(
            BitgetSymbolConverter::ws_inst_type("ETH/USD:ETH"),
            "coin-futures"
        );
    }

    #[test]
    fn test_ws_inst_type_exchange_format() {
        // Exchange format (no "/") defaults to spot
        assert_eq!(BitgetSymbolConverter::ws_inst_type("BTCUSDT"), "spot");
    }

    // ========================================================================
    // exchange_to_unified_inferred tests
    // ========================================================================

    #[test]
    fn test_exchange_to_unified_inferred_spot() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_inferred("BTCUSDT"),
            "BTC/USDT"
        );
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_inferred("ETHBTC"),
            "ETH/BTC"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred_inverse() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_inferred("BTCUSD"),
            "BTC/USD:BTC"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred_linear() {
        assert_eq!(
            BitgetSymbolConverter::exchange_to_unified_inferred("BTCUSDC"),
            "BTC/USDC"
        );
    }
}

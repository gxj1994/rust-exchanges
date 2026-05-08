//! Hyperliquid Symbol 转换器
//!
//! Hyperliquid 使用 coin 名称作为交易标识（如 "BTC"、"ETH"）。
//! Oracle 价格以 USDT 计价，但保证金和结算使用 USDC。
//! 为简化处理，永续合约统一使用 USDC 作为 quote/settle 货币。
//!
//! # 市场类型
//!
//! - **永续合约（Perpetuals）**: coin = "BTC", 统一格式 = "BTC/USDC:USDC"
//! - **现货（Spot）**: coin = "PURR/USDC" 或 "@{index}", 统一格式 = "PURR/USDC"

use ccxt_core::symbol::{SymbolContext, SymbolConverter, SymbolError};
use ccxt_core::types::common::symbol::{ParsedSymbol, SymbolMarketType};

/// Hyperliquid Symbol 转换器
pub struct HyperliquidSymbolConverter;

impl HyperliquidSymbolConverter {
    /// 判断 coin 是否为现货格式
    ///
    /// # 现货格式
    ///
    /// - `PURR/USDC` 格式（包含斜杠）
    /// - `@{index}` 格式（以 @ 开头）
    pub fn is_spot_coin(coin: &str) -> bool {
        coin.contains('/') || coin.starts_with('@')
    }

    /// 将 Hyperliquid coin 名称转换为统一符号格式
    ///
    /// # Arguments
    ///
    /// * `coin` - Hyperliquid coin 名称（如 "BTC", "PURR/USDC", "@107"）
    ///
    /// # Returns
    ///
    /// 统一符号格式
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // 永续合约
    /// assert_eq!(coin_to_unified("BTC"), "BTC/USDC:USDC");
    ///
    /// // 现货
    /// assert_eq!(coin_to_unified("PURR/USDC"), "PURR/USDC");
    /// ```
    pub fn coin_to_unified(coin: &str) -> String {
        if Self::is_spot_coin(coin) {
            // 现货：PURR/USDC 格式已经是统一格式
            coin.to_string()
        } else {
            // 永续合约：使用 USDC 作为 quote 和 settle
            format!("{}/USDC:USDC", coin)
        }
    }

    /// 将统一符号格式转换为 Hyperliquid coin 名称
    ///
    /// # Arguments
    ///
    /// * `unified_symbol` - 统一符号格式（如 "BTC/USDC:USDC"）
    ///
    /// # Returns
    ///
    /// Hyperliquid coin 名称（如 "BTC"）
    pub fn unified_to_coin(unified_symbol: &str) -> String {
        // 提取 base currency (BTC/USDC:USDC -> BTC)
        unified_symbol
            .split('/')
            .next()
            .unwrap_or(unified_symbol)
            .to_string()
    }

    /// Convert Hyperliquid exchange format to unified symbol (inferred)
    ///
    /// This is an alias for `coin_to_unified` for API consistency.
    pub fn exchange_to_unified_inferred(coin: &str) -> String {
        Self::coin_to_unified(coin)
    }
}

/// Implement SymbolConverter trait for Hyperliquid
impl SymbolConverter for HyperliquidSymbolConverter {
    fn to_exchange_id(&self, symbol: &ParsedSymbol) -> String {
        // Hyperliquid uses coin name (base currency)
        symbol.base.clone()
    }

    fn from_exchange_id(
        &self,
        exchange_id: &str,
        context: SymbolContext,
    ) -> Result<ParsedSymbol, SymbolError> {
        // Hyperliquid only supports USDC-margined perpetuals
        match context.market_type {
            SymbolMarketType::Spot => {
                // Hyperliquid 现货格式: "PURR/USDC" 或 "@{index}"
                if exchange_id.contains('/') {
                    // "PURR/USDC" 格式 - 解析 base/quote
                    let parts: Vec<&str> = exchange_id.split('/').collect();
                    if parts.len() == 2 {
                        Ok(ParsedSymbol::spot(
                            parts[0].to_string(),
                            parts[1].to_string(),
                        ))
                    } else {
                        Err(SymbolError::InvalidFormat(format!(
                            "Invalid spot symbol format: {}",
                            exchange_id
                        )))
                    }
                } else {
                    // "@{index}" 格式 - 需要 spotMeta 解析，这里返回占位符
                    Ok(ParsedSymbol::spot(
                        exchange_id.to_string(),
                        "USDC".to_string(),
                    ))
                }
            }
            SymbolMarketType::Swap => {
                // Create linear swap with USDC as quote/settle
                Ok(ParsedSymbol::linear_swap(
                    exchange_id.to_string(),
                    "USDC".to_string(),
                ))
            }
            SymbolMarketType::Futures => {
                // Hyperliquid doesn't have traditional futures
                Err(SymbolError::InvalidFormat(
                    "Hyperliquid does not support futures markets".to_string(),
                ))
            }
        }
    }

    fn exchange_to_unified_inferred(&self, exchange_id: &str) -> String {
        Self::coin_to_unified(exchange_id)
    }

    fn detect_market_type(&self, exchange_id: &str) -> Option<SymbolMarketType> {
        if Self::is_spot_coin(exchange_id) {
            Some(SymbolMarketType::Spot)
        } else {
            Some(SymbolMarketType::Swap)
        }
    }

    fn is_spot(&self, exchange_id: &str) -> bool {
        Self::is_spot_coin(exchange_id)
    }

    fn is_swap(&self, exchange_id: &str) -> bool {
        !Self::is_spot_coin(exchange_id)
    }

    fn is_futures(&self, _exchange_id: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coin_to_unified() {
        // 永续合约
        assert_eq!(
            HyperliquidSymbolConverter::coin_to_unified("BTC"),
            "BTC/USDC:USDC"
        );
        assert_eq!(
            HyperliquidSymbolConverter::coin_to_unified("ETH"),
            "ETH/USDC:USDC"
        );

        // 现货
        assert_eq!(
            HyperliquidSymbolConverter::coin_to_unified("PURR/USDC"),
            "PURR/USDC"
        );
        assert_eq!(HyperliquidSymbolConverter::coin_to_unified("@107"), "@107");
    }

    #[test]
    fn test_is_spot_coin() {
        // 现货格式
        assert!(HyperliquidSymbolConverter::is_spot_coin("PURR/USDC"));
        assert!(HyperliquidSymbolConverter::is_spot_coin("@107"));
        assert!(HyperliquidSymbolConverter::is_spot_coin("HYPE/USDC"));

        // 永续合约格式
        assert!(!HyperliquidSymbolConverter::is_spot_coin("BTC"));
        assert!(!HyperliquidSymbolConverter::is_spot_coin("ETH"));
    }

    #[test]
    fn test_unified_to_coin() {
        assert_eq!(
            HyperliquidSymbolConverter::unified_to_coin("BTC/USDC:USDC"),
            "BTC"
        );
        assert_eq!(
            HyperliquidSymbolConverter::unified_to_coin("ETH/USDC:USDC"),
            "ETH"
        );
    }

    #[test]
    fn test_exchange_to_unified_inferred() {
        assert_eq!(
            HyperliquidSymbolConverter::exchange_to_unified_inferred("BTC"),
            "BTC/USDC:USDC"
        );
    }
}

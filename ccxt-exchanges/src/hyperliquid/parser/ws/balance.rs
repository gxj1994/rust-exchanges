//! WebSocket Balance data parser for HyperLiquid.
//!
//! Parses userEvents channel balance messages.

use ccxt_core::{
    error::Result,
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

/// Parse balance from WebSocket userEvents message.
///
/// # Arguments
///
/// * `data` - The `data` field from WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // Hyperliquid 余额格式: {total, available, ...}
    if let Some(total) = data
        .get("total")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
    {
        let free = data
            .get("available")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_default();

        let used = total - free;

        balances.insert("USDC".to_string(), BalanceEntry { free, used, total });
    }

    Ok(Balance {
        balances,
        info: HashMap::new(),
    })
}

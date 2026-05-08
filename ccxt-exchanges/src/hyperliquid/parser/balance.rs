//! Balance data parser for HyperLiquid.

use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, value_to_hashmap},
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

/// Parse balance data from HyperLiquid user state response.
///
/// # Arguments
///
/// * `data` - HyperLiquid user state JSON object
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // Parse margin summary
    if let Some(margin) = data.get("marginSummary") {
        let account_value = parse_decimal(margin, "accountValue").unwrap_or(Decimal::ZERO);
        let total_margin_used = parse_decimal(margin, "totalMarginUsed").unwrap_or(Decimal::ZERO);
        let available = account_value - total_margin_used;

        balances.insert(
            "USDC".to_string(),
            BalanceEntry {
                free: available,
                used: total_margin_used,
                total: account_value,
            },
        );
    }

    // Also check withdrawable
    if let Some(withdrawable) = data.get("withdrawable") {
        if let Some(w) = withdrawable
            .as_str()
            .and_then(|s| Decimal::from_str(s).ok())
        {
            if let Some(entry) = balances.get_mut("USDC") {
                entry.free = w;
            }
        }
    }

    Ok(Balance {
        balances,
        info: value_to_hashmap(data),
    })
}

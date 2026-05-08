//! Balance data parser for Bitget.

use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, value_to_hashmap},
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;

/// Parse balance data from Bitget V3 account info.
///
/// # Arguments
///
/// * `data` - Bitget V3 account data JSON object (contains `assets` array)
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure with all non-zero balances.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // V3 API: data.assets[] array
    if let Some(assets_array) = data["assets"].as_array() {
        for asset in assets_array {
            parse_balance_entry(asset, &mut balances);
        }
    } else if let Some(balances_array) = data.as_array() {
        // Fallback: handle array of balances (V2 format)
        for balance in balances_array {
            parse_balance_entry(balance, &mut balances);
        }
    } else {
        // Fallback: handle single balance object
        parse_balance_entry(data, &mut balances);
    }

    Ok(Balance {
        balances,
        info: value_to_hashmap(data),
    })
}

/// Parse a single balance entry from Bitget V3 response.
fn parse_balance_entry(data: &Value, balances: &mut HashMap<String, BalanceEntry>) {
    let currency = data["coin"]
        .as_str()
        .or_else(|| data["coinName"].as_str())
        .or_else(|| data["asset"].as_str())
        .map(ToString::to_string);

    if let Some(currency) = currency {
        // V3 API: available, locked, balance/equity
        let available = parse_decimal(data, "available")
            .or_else(|| parse_decimal(data, "free"))
            .unwrap_or(Decimal::ZERO);

        let frozen = parse_decimal(data, "locked")
            .or_else(|| parse_decimal(data, "frozen"))
            .or_else(|| parse_decimal(data, "lock"))
            .unwrap_or(Decimal::ZERO);

        // Use balance or equity as total, fallback to available + frozen
        let total = parse_decimal(data, "balance")
            .or_else(|| parse_decimal(data, "equity"))
            .unwrap_or(available + frozen);

        // Only include non-zero balances
        if total > Decimal::ZERO {
            balances.insert(
                currency,
                BalanceEntry {
                    free: available,
                    used: frozen,
                    total,
                },
            );
        }
    }
}

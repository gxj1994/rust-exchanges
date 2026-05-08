//! WebSocket Balance data parser for Bitget.

use ccxt_core::{
    error::Result,
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use super::extract_first_data;

/// Parse balance from WebSocket account message.
pub fn parse_balance(msg: &Value) -> Result<Balance> {
    let data = extract_first_data(msg)?;

    let mut balances = HashMap::new();

    if let Some(details) = data.get("details").and_then(|d| d.as_array()) {
        for detail in details {
            let currency = detail
                .get("coin")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string();

            if currency.is_empty() {
                continue;
            }

            let free = detail
                .get("available")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default();

            let used = detail
                .get("frozen")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default();

            let total = free + used;

            balances.insert(currency, BalanceEntry { free, used, total });
        }
    }

    Ok(Balance {
        balances,
        info: HashMap::new(),
    })
}

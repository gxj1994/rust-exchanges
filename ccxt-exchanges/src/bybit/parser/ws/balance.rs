//! WebSocket Balance data parser for Bybit.

use ccxt_core::{
    error::Result,
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use super::extract_data;

/// Parse balance from WebSocket wallet message.
pub fn parse_balance(msg: &Value) -> Result<Balance> {
    let data = extract_data(msg)?;

    let mut balances = HashMap::new();

    if let Some(coins) = data.get("coin").and_then(|c| c.as_array()) {
        for coin in coins {
            let currency = coin
                .get("coin")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string();

            if currency.is_empty() {
                continue;
            }

            let free = coin
                .get("walletBalance")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default();

            let used = coin
                .get("locked")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default();

            let total = free;

            balances.insert(currency, BalanceEntry { free, used, total });
        }
    }

    Ok(Balance {
        balances,
        info: HashMap::new(),
    })
}

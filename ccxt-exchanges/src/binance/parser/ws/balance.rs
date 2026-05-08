//! WebSocket Balance data parser for Binance.
//!
//! Parses outboundAccountPosition and ACCOUNT_UPDATE events.

use ccxt_core::{
    error::Result,
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

/// Parse balance from WebSocket outboundAccountPosition message (spot).
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure.
pub fn parse_balance(msg: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // outboundAccountPosition format
    if let Some(balances_arr) = msg.get("B").and_then(|b| b.as_array()) {
        for balance in balances_arr {
            let asset = balance
                .get("a")
                .and_then(|a| a.as_str())
                .unwrap_or_default();
            let free = balance
                .get("f")
                .and_then(|f| f.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default();
            let locked = balance
                .get("l")
                .and_then(|l| l.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .unwrap_or_default();

            if free > Decimal::ZERO || locked > Decimal::ZERO {
                balances.insert(
                    asset.to_string(),
                    BalanceEntry {
                        free,
                        used: locked,
                        total: free + locked,
                    },
                );
            }
        }
    }

    Ok(Balance {
        balances,
        info: HashMap::new(),
    })
}

/// Parse balance from WebSocket ACCOUNT_UPDATE message (futures).
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure.
pub fn parse_balance_futures(msg: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    if let Some(account_data) = msg.get("a") {
        if let Some(balances_arr) = account_data.get("B").and_then(|b| b.as_array()) {
            for balance in balances_arr {
                let asset = balance
                    .get("a")
                    .and_then(|a| a.as_str())
                    .unwrap_or_default();
                let wallet_balance = balance
                    .get("wb")
                    .and_then(|wb| wb.as_str())
                    .and_then(|s| Decimal::from_str(s).ok())
                    .unwrap_or_default();

                if wallet_balance > Decimal::ZERO {
                    balances.insert(
                        asset.to_string(),
                        BalanceEntry {
                            free: wallet_balance,
                            used: Decimal::ZERO, // Futures needs calculation from other fields
                            total: wallet_balance,
                        },
                    );
                }
            }
        }
    }

    Ok(Balance {
        balances,
        info: HashMap::new(),
    })
}

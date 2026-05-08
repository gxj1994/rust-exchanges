//! Balance parser for OKX.

use crate::common::parser_helpers::ParseHelper;
use ccxt_core::{
    Result,
    parser_utils::value_to_hashmap,
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;

/// Parse balance data from OKX account info.
///
/// # Arguments
///
/// * `data` - OKX account data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure with all non-zero balances.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // OKX returns balance in details array
    if let Some(details) = data["details"].as_array() {
        for detail in details {
            parse_balance_entry(detail, &mut balances);
        }
    } else if let Some(balances_array) = data.as_array() {
        // Handle array of balance objects
        for balance in balances_array {
            if let Some(details) = balance["details"].as_array() {
                for detail in details {
                    parse_balance_entry(detail, &mut balances);
                }
            } else {
                parse_balance_entry(balance, &mut balances);
            }
        }
    } else {
        // Handle single balance object
        parse_balance_entry(data, &mut balances);
    }

    Ok(Balance {
        balances,
        info: value_to_hashmap(data),
    })
}

/// Parse a single balance entry from OKX response.
fn parse_balance_entry(data: &Value, balances: &mut HashMap<String, BalanceEntry>) {
    let currency = data["ccy"]
        .as_str()
        .or_else(|| data["currency"].as_str())
        .map(ToString::to_string);

    if let Some(currency) = currency {
        // OKX uses different field names depending on account type
        let available = ParseHelper::decimal_any(data, &["availBal", "availEq", "cashBal"])
            .unwrap_or(Decimal::ZERO);

        let frozen =
            ParseHelper::decimal_any(data, &["frozenBal", "ordFrozen"]).unwrap_or(Decimal::ZERO);

        let total = ParseHelper::decimal_any(data, &["eq", "bal"]).unwrap_or(available + frozen);

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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_balance() {
        let data = json!({
            "details": [
                {
                    "ccy": "BTC",
                    "availBal": "1.5",
                    "frozenBal": "0.5",
                    "eq": "2.0"
                },
                {
                    "ccy": "USDT",
                    "availBal": "10000.00",
                    "frozenBal": "0",
                    "eq": "10000.00"
                }
            ]
        });

        let balance = parse_balance(&data).unwrap();
        let btc = balance.get("BTC").unwrap();
        assert_eq!(btc.free, dec!(1.5));
        assert_eq!(btc.used, dec!(0.5));
        assert_eq!(btc.total, dec!(2.0));

        let usdt = balance.get("USDT").unwrap();
        assert_eq!(usdt.free, dec!(10000.00));
        assert_eq!(usdt.total, dec!(10000.00));
    }
}

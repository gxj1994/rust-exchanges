//! Balance parser for Bybit.

use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, value_to_hashmap},
    types::{Balance, BalanceEntry},
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;

/// Parse balance data from Bybit account info.
///
/// # Arguments
///
/// * `data` - Bybit account data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Balance`] structure with all non-zero balances.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // Bybit returns balance in coin array
    if let Some(coins) = data["coin"].as_array() {
        for coin in coins {
            parse_balance_entry(coin, &mut balances);
        }
    } else if let Some(list) = data["list"].as_array() {
        // Handle list format from wallet balance endpoint
        for item in list {
            if let Some(coins) = item["coin"].as_array() {
                for coin in coins {
                    parse_balance_entry(coin, &mut balances);
                }
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

/// Parse a single balance entry from Bybit response.
fn parse_balance_entry(data: &Value, balances: &mut HashMap<String, BalanceEntry>) {
    let currency = data["coin"]
        .as_str()
        .or_else(|| data["currency"].as_str())
        .map(ToString::to_string);

    if let Some(currency) = currency {
        // Bybit uses different field names depending on account type
        let available = parse_decimal(data, "availableToWithdraw")
            .or_else(|| parse_decimal(data, "free"))
            .or_else(|| parse_decimal(data, "walletBalance"))
            .unwrap_or(Decimal::ZERO);

        let frozen = parse_decimal(data, "locked")
            .or_else(|| parse_decimal(data, "frozen"))
            .unwrap_or(Decimal::ZERO);

        let total = parse_decimal(data, "walletBalance")
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_balance() {
        let data = json!({
            "coin": [
                {
                    "coin": "BTC",
                    "walletBalance": "2.0",
                    "availableToWithdraw": "1.5",
                    "locked": "0.5"
                },
                {
                    "coin": "USDT",
                    "walletBalance": "10000.00",
                    "availableToWithdraw": "10000.00",
                    "locked": "0"
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

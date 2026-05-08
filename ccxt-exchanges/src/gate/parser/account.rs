//! Gate.io account/balance parser.
//!
//! Parses balance data from Gate.io spot and contract API responses.
//!
//! # API Response Format
//!
//! Gate.io spot balance response format:
//! ```json
//! [
//!   {
//!     "currency": "BTC",
//!     "available": "0.123456",
//!     "locked": "0.01"
//!   },
//!   {
//!     "currency": "USDT",
//!     "available": "1000.50",
//!     "locked": "100.00"
//!   }
//! ]
//! ```
//!
//! Gate.io futures balance response format:
//! ```json
//! {
//!   "total": 10000.50,
//!   "available": 9500.25,
//!   "unrealised_pnl": 500.25,
//!   "crossed_pos": 5000.00,
//!   "currency": "USDT"
//! }
//! ```

use super::parse_decimal;
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::Balance,
    types::account::BalanceEntry,
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;

/// Convert JSON Value to HashMap<String, Value> for the `info` field.
fn value_to_info(value: &Value) -> HashMap<String, Value> {
    let mut map = HashMap::new();

    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            map.insert(key.clone(), val.clone());
        }
    }

    map
}

/// Parse balance from Gate.io spot API response.
///
/// # Arguments
///
/// * `data` - Gate.io balance response JSON (array of currency balances)
///
/// # Returns
///
/// Returns a CCXT Balance structure.
pub fn parse_balance(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();

    // Gate.io returns an array of currency balances
    let balances_array = data.as_array().ok_or_else(|| {
        Error::from(ParseError::invalid_format(
            "data",
            "Expected array of balances",
        ))
    })?;

    for balance_item in balances_array {
        let currency = balance_item["currency"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("currency")))?
            .to_string();

        let available = parse_decimal(balance_item, "available").unwrap_or(Decimal::ZERO);

        let locked = parse_decimal(balance_item, "locked").unwrap_or(Decimal::ZERO);

        let total = available + locked;

        // Only include currencies with non-zero balance
        if total > Decimal::ZERO {
            balances.insert(
                currency,
                BalanceEntry {
                    free: available,
                    used: locked,
                    total,
                },
            );
        }
    }

    Ok(Balance {
        balances,
        info: value_to_info(data),
    })
}

/// Parse balance from Gate.io futures/swap API response.
///
/// # Arguments
///
/// * `data` - Gate.io futures balance response JSON (single currency)
/// * `settle_currency` - The settle currency (e.g., "USDT", "BTC")
///
/// # Returns
///
/// Returns a CCXT Balance structure.
pub fn parse_futures_balance(data: &Value, settle_currency: &str) -> Result<Balance> {
    let mut balances = HashMap::new();

    let available = parse_decimal(data, "available")
        .or_else(|| parse_decimal(data, "total"))
        .unwrap_or(Decimal::ZERO);

    // let unrealised_pnl = parse_decimal(data, "unrealised_pnl").unwrap_or(Decimal::ZERO);

    let crossed_pos = parse_decimal(data, "crossed_pos").unwrap_or(Decimal::ZERO);

    // In futures, total = available + used (positions)
    let total = available + crossed_pos.abs();
    let used = crossed_pos.abs();

    if total > Decimal::ZERO {
        balances.insert(
            settle_currency.to_string(),
            BalanceEntry {
                free: available,
                used,
                total,
            },
        );
    }

    // Include unrealized PnL in info
    let info = value_to_info(data);

    Ok(Balance { balances, info })
}

/// Parse multiple futures balances (for multi-currency accounts).
///
/// # Arguments
///
/// * `data` - Array of futures balance objects
///
/// # Returns
///
/// Returns a CCXT Balance structure.
#[allow(unused)]
pub fn parse_futures_balances(data: &Value) -> Result<Balance> {
    let mut balances = HashMap::new();
    let mut info_map = HashMap::new();

    let balances_array = data.as_array().ok_or_else(|| {
        Error::from(ParseError::invalid_format(
            "data",
            "Expected array of balances",
        ))
    })?;

    for balance_item in balances_array {
        let currency = balance_item["currency"]
            .as_str()
            .unwrap_or("UNKNOWN")
            .to_string();

        let available = parse_decimal(balance_item, "available").unwrap_or(Decimal::ZERO);

        let crossed_pos = parse_decimal(balance_item, "crossed_pos").unwrap_or(Decimal::ZERO);

        let total = available + crossed_pos.abs();
        let used = crossed_pos.abs();

        if total > Decimal::ZERO {
            balances.insert(
                currency.clone(),
                BalanceEntry {
                    free: available,
                    used,
                    total,
                },
            );
        }

        info_map.insert(currency, balance_item.clone());
    }

    Ok(Balance {
        balances,
        info: info_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_spot_balance_basic() {
        let data = json!([
            {
                "currency": "BTC",
                "available": "0.123456",
                "locked": "0.01"
            },
            {
                "currency": "USDT",
                "available": "1000.50",
                "locked": "100.00"
            }
        ]);

        let balance = parse_balance(&data).unwrap();

        assert_eq!(balance.balances.len(), 2);

        // Check BTC balance
        let btc = balance.balances.get("BTC").unwrap();
        assert_eq!(btc.free, Decimal::new(123456, 6));
        assert_eq!(btc.used, Decimal::new(1, 2));
        assert_eq!(btc.total, Decimal::new(133456, 6));

        // Check USDT balance
        let usdt = balance.balances.get("USDT").unwrap();
        assert_eq!(usdt.free, Decimal::new(100050, 2));
        assert_eq!(usdt.used, Decimal::new(10000, 2));
        assert_eq!(usdt.total, Decimal::new(110050, 2));
    }

    #[test]
    fn test_parse_spot_balance_empty() {
        let data = json!([]);

        let balance = parse_balance(&data).unwrap();

        assert_eq!(balance.balances.len(), 0);
    }

    #[test]
    fn test_parse_spot_balance_zero_amounts() {
        // Should filter out zero balances
        let data = json!([
            {
                "currency": "BTC",
                "available": "0",
                "locked": "0"
            },
            {
                "currency": "ETH",
                "available": "1.5",
                "locked": "0"
            }
        ]);

        let balance = parse_balance(&data).unwrap();

        // BTC should be filtered out (total = 0)
        assert_eq!(balance.balances.len(), 1);
        assert!(balance.balances.contains_key("ETH"));
        assert!(!balance.balances.contains_key("BTC"));
    }

    #[test]
    fn test_parse_futures_balance_usdt() {
        let data = json!({
            "total": 10000.50,
            "available": 9500.25,
            "unrealised_pnl": 500.25,
            "crossed_pos": 5000.00,
            "currency": "USDT"
        });

        let balance = parse_futures_balance(&data, "USDT").unwrap();

        assert_eq!(balance.balances.len(), 1);

        let usdt = balance.balances.get("USDT").unwrap();
        assert_eq!(usdt.free, Decimal::new(950025, 2));
        assert_eq!(usdt.used, Decimal::new(500000, 2));
        assert!(usdt.total >= Decimal::new(1000000, 2));
    }

    #[test]
    fn test_parse_futures_balances_multi_currency() {
        let data = json!([
            {
                "currency": "USDT",
                "available": "5000.00",
                "crossed_pos": "2000.00"
            },
            {
                "currency": "BTC",
                "available": "0.5",
                "crossed_pos": "0.1"
            }
        ]);

        let balance = parse_futures_balances(&data).unwrap();

        assert_eq!(balance.balances.len(), 2);

        let usdt = balance.balances.get("USDT").unwrap();
        assert_eq!(usdt.free, Decimal::new(500000, 2));
        assert_eq!(usdt.used, Decimal::new(200000, 2));

        let btc = balance.balances.get("BTC").unwrap();
        assert_eq!(btc.free, Decimal::new(5, 1));
        assert_eq!(btc.used, Decimal::new(1, 1));
    }

    #[test]
    fn test_parse_spot_balance_missing_fields() {
        // Test with missing optional fields
        let data = json!([
            {
                "currency": "BTC",
                "available": "1.0"
                // "locked" field is missing
            }
        ]);

        let balance = parse_balance(&data).unwrap();

        assert_eq!(balance.balances.len(), 1);

        let btc = balance.balances.get("BTC").unwrap();
        assert_eq!(btc.free, Decimal::new(1, 0));
        assert_eq!(btc.used, Decimal::ZERO); // Should default to 0
        assert_eq!(btc.total, Decimal::new(1, 0));
    }
}

// ============================================================================
// Contract Position Parsing
// ============================================================================

/// Parse a contract position from Gate.io futures API.
///
/// # Gate.io Position Response
///
/// ```json
/// {
///     "user": 123456,
///     "time": 1610611200,
///     "contract": "BTC_USDT",
///     "size": 100,
///     "leverage": "10",
///     "mode": "single",
///     "position_type": "long",
///     "entry_price": "42000.5",
///     "maintenance_margin": "420.05",
///     "unrealised_pnl": "100.5",
///     "realised_pnl": "50.25",
///     "value": "4200050",
///     "margin": "42000.5",
///     "liq_price": "38000",
///     "mark_price": "42100",
///     "initial_margin": "42000.5",
///     "leverage_avg": "10",
///     "cross_leverage_limit": "0"
/// }
/// ```
pub fn parse_contract_position(data: &Value, settle: &str) -> Result<ccxt_core::types::Position> {
    use super::parse_f64;
    use ccxt_core::types::Position;

    let contract = data["contract"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("contract")))?
        .to_string();

    // Try to parse symbol from contract name
    let symbol_str = if contract.contains('_') {
        let parts: Vec<&str> = contract.splitn(2, '_').collect();
        if parts.len() == 2 {
            format!("{}/{}:{}", parts[0], parts[1], settle.to_uppercase())
        } else {
            contract.clone()
        }
    } else {
        contract.clone()
    };

    let size = parse_f64(data, "size");
    let entry_price = parse_f64(data, "entry_price");
    let mark_price = parse_f64(data, "mark_price");
    let leverage = parse_f64(data, "leverage");
    let unrealised_pnl = parse_f64(data, "unrealised_pnl");
    let realised_pnl = parse_f64(data, "realised_pnl");
    let liquidation_price = parse_f64(data, "liq_price");
    let margin = parse_f64(data, "margin");
    let maintenance_margin = parse_f64(data, "maintenance_margin");
    let notional = parse_f64(data, "value");

    let position_type = data["position_type"].as_str().unwrap_or("");

    let side = if position_type == "long" {
        Some("long".to_string())
    } else if position_type == "short" {
        Some("short".to_string())
    } else {
        None
    };

    let contracts = size;
    let contract_size = Some(1.0); // Default, can be overridden by market data

    Ok(Position {
        info: data.clone(),
        id: None,
        symbol: symbol_str,
        side,
        position_side: None,
        dual_side_position: None,
        contracts,
        contract_size,
        entry_price,
        mark_price,
        liquidation_price,
        collateral: margin,
        initial_margin: margin,
        initial_margin_percentage: None,
        notional,
        leverage,
        margin_ratio: None,
        maintenance_margin,
        maintenance_margin_percentage: None,
        unrealized_pnl: unrealised_pnl,
        realized_pnl: realised_pnl,
        percentage: None,
        datetime: None,
        hedged: None,
        margin_mode: None,
        timestamp: None,
    })
}

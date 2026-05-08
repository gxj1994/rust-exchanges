//! Gate.io order parser.
//!
//! Parses order data from Gate.io spot and contract API responses.
//!
//! # API Response Format
//!
//! Gate.io order response format:
//! ```json
//! {
//!   "id": "1234567890",
//!   "text": "user-custom-id",
//!   "create_time": "1609917600",
//!   "update_time": "1609917601",
//!   "create_time_ms": 1609917600000,
//!   "update_time_ms": 1609917601000,
//!   "status": "closed",
//!   "currency_pair": "BTC_USDT",
//!   "type": "limit",
//!   "account": "spot",
//!   "side": "buy",
//!   "amount": "0.01",
//!   "price": "40000",
//!   "time_in_force": "gtc",
//!   "filled_amount": "0.01",
//!   "fill_price": "40000",
//!   "fee": "0.00004",
//!   "fee_currency": "USDT",
//!   "rebated_fee": "0",
//!   "rebated_fee_currency": "USDT"
//! }
//! ```

use super::parse_decimal;
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{Market, Order, OrderSide, OrderStatus, OrderType, Symbol, TimeInForce},
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

/// Parse order from Gate.io spot API response.
///
/// # Arguments
///
/// * `data` - Gate.io order response JSON
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT Order structure.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    // Parse symbol
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        let currency_pair = data["currency_pair"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("currency_pair")))?;

        // Convert BTC_USDT -> BTC/USDT
        let symbol_str = currency_pair.replace('_', "/");
        Symbol::new_unchecked(&symbol_str)
    };

    // Parse order ID
    let id = data["id"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| data["id"].as_u64().map(|v| v.to_string()))
        .ok_or_else(|| Error::from(ParseError::missing_field("id")))?;

    // Parse timestamps
    let timestamp = data["create_time_ms"]
        .as_i64()
        .or_else(|| data["create_time"].as_i64().map(|t| t * 1000));

    let last_trade_timestamp = data["update_time_ms"]
        .as_i64()
        .or_else(|| data["update_time"].as_i64().map(|t| t * 1000));

    // Parse order status
    let status = if let Some(status_str) = data["status"].as_str() {
        match status_str {
            "closed" => OrderStatus::Closed,
            "cancelled" => OrderStatus::Cancelled,
            "open" => OrderStatus::Open,
            "pending" => OrderStatus::Pending,
            "cancelled_over_limit" => OrderStatus::Cancelled,
            _ => OrderStatus::Open,
        }
    } else {
        OrderStatus::Open
    };

    // Parse order side
    let side = if let Some(side_str) = data["side"].as_str() {
        match side_str {
            "buy" => OrderSide::Buy,
            "sell" => OrderSide::Sell,
            _ => {
                return Err(Error::from(ParseError::invalid_format(
                    "side",
                    format!("Invalid side value: {}", side_str),
                )));
            }
        }
    } else {
        return Err(Error::from(ParseError::missing_field("side")));
    };

    // Parse order type
    let order_type = if let Some(type_str) = data["type"].as_str() {
        match type_str {
            "limit" => OrderType::Limit,
            "market" => OrderType::Market,
            _ => OrderType::Limit,
        }
    } else {
        OrderType::Limit
    };

    // Parse time in force
    let time_in_force = if let Some(tif_str) = data["time_in_force"].as_str() {
        match tif_str {
            "gtc" => Some(TimeInForce::GTC),
            "ioc" => Some(TimeInForce::IOC),
            "poc" => Some(TimeInForce::PO),
            "fok" => Some(TimeInForce::FOK),
            _ => Some(TimeInForce::GTC),
        }
    } else {
        Some(TimeInForce::GTC)
    };

    // Parse price, amount, and filled
    let price = parse_decimal(data, "price");
    let amount = parse_decimal(data, "amount").or_else(|| parse_decimal(data, "iceberg"));

    let filled = parse_decimal(data, "filled_amount").or_else(|| {
        parse_decimal(data, "fill_price").map(|_| {
            // If only fill_price exists, try to calculate filled amount
            Decimal::ZERO
        })
    });

    // Calculate remaining
    let remaining = match (&amount, &filled) {
        (Some(a), Some(f)) => Some(*a - *f),
        _ => None,
    };

    // Calculate average price
    let cost = parse_decimal(data, "fill_price");
    let average = match (&cost, &filled) {
        (Some(c), Some(f)) if !f.is_zero() => Some(*c / *f),
        _ => price,
    };

    // Parse fee
    let fee_currency = data["fee_currency"].as_str().map(ToString::to_string);
    let fee_amount = parse_decimal(data, "fee");

    let fee = match (&fee_currency, &fee_amount) {
        (Some(currency), Some(amount)) => Some(ccxt_core::types::Fee {
            currency: currency.clone(),
            cost: *amount,
            rate: None,
        }),
        _ => None,
    };

    // Format datetime
    let datetime = timestamp.map(|t| {
        chrono::DateTime::from_timestamp(t / 1000, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    });

    Ok(Order {
        id,
        client_order_id: data["text"].as_str().map(ToString::to_string),
        timestamp,
        datetime,
        last_trade_timestamp,
        status,
        symbol,
        order_type,
        time_in_force: time_in_force.map(|t| t.to_string()),
        side,
        price,
        average,
        amount: amount.unwrap_or(Decimal::ZERO),
        filled,
        remaining,
        cost,
        trades: None,
        fee: fee.clone(),
        fees: fee.map(|f| vec![f]),
        post_only: None,
        reduce_only: data["reduce_only"].as_bool(),
        trigger_price: parse_decimal(data, "trigger_price"),
        stop_price: parse_decimal(data, "stop_price"),
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        info: value_to_info(data),
    })
}

/// Parse multiple orders from Gate.io API response.
///
/// # Arguments
///
/// * `data` - Array of order objects
/// * `market` - Optional market information
///
/// # Returns
///
/// Returns a vector of parsed orders.
pub fn parse_orders(data: &Value, market: Option<&Market>) -> Result<Vec<Order>> {
    let orders_array = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

    orders_array
        .iter()
        .map(|order_data| parse_order(order_data, market))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_order_basic() {
        let data = json!({
            "id": "1234567890",
            "text": "user-order-001",
            "create_time_ms": 1609917600000i64,
            "update_time_ms": 1609917601000i64,
            "status": "closed",
            "currency_pair": "BTC_USDT",
            "type": "limit",
            "side": "buy",
            "amount": "0.01",
            "price": "40000",
            "filled_amount": "0.01",
            "fill_price": "400",
            "fee": "0.00004",
            "fee_currency": "USDT",
            "time_in_force": "gtc"
        });

        let order = parse_order(&data, None).unwrap();

        assert_eq!(order.id, "1234567890");
        assert_eq!(order.client_order_id, Some("user-order-001".to_string()));
        assert_eq!(order.status, OrderStatus::Closed);
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert!(order.price.is_some());
        assert_eq!(
            order.amount,
            "0.01".parse::<rust_decimal::Decimal>().unwrap()
        );
    }

    #[test]
    fn test_parse_order_market() {
        let data = json!({
            "id": "9876543210",
            "create_time_ms": 1609917600000i64,
            "status": "open",
            "currency_pair": "ETH_USDT",
            "type": "market",
            "side": "sell",
            "amount": "1.5",
            "filled_amount": "0",
            "time_in_force": "gtc"
        });

        let order = parse_order(&data, None).unwrap();

        assert_eq!(order.id, "9876543210");
        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.side, OrderSide::Sell);
        assert_eq!(order.order_type, OrderType::Market);
    }

    #[test]
    fn test_parse_order_cancelled() {
        let data = json!({
            "id": "5555555555",
            "create_time_ms": 1609917600000i64,
            "status": "cancelled",
            "currency_pair": "BTC_USDT",
            "type": "limit",
            "side": "buy",
            "amount": "0.1",
            "price": "30000",
            "filled_amount": "0",
            "time_in_force": "ioc"
        });

        let order = parse_order(&data, None).unwrap();

        assert_eq!(order.status, OrderStatus::Cancelled);
        assert_eq!(order.time_in_force, Some("IOC".to_string()));
    }
}

// ============================================================================
// Contract (Swap/Futures) Order Parsing
// ============================================================================

/// Parse a single contract order from Gate.io futures API response.
///
/// # Contract Order Format
///
/// Contract orders use different fields than spot orders:
/// - `contract` instead of `currency_pair`
/// - `size` (positive=negative=sell) instead of `side`
/// - No `type` field (always limit/market based on context)
///
/// # Example Response
///
/// ```json
/// {
///   "contract": "BTC_USDT",
///   "size": 1,
///   "price": "46551.5",
///   "status": "open",
///   "id": 82472171516617186,
///   "create_time": 1777084711.377,
///   "update_time": 1777084711.377,
///   "tif": "gtc",
///   "fill_price": "0",
///   "left": 1
/// }
/// ```
pub fn parse_contract_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    // Parse symbol from market or contract field
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        let contract = data["contract"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("contract")))?;

        // Convert BTC_USDT -> BTC/USDT:USDT (basic format, settle should come from market)
        let symbol_str = contract.replace('_', "/");
        Symbol::new_unchecked(&symbol_str)
    };

    // Parse order ID
    let id = data["id"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| data["id"].as_i64().map(|v| v.to_string()))
        .ok_or_else(|| Error::from(ParseError::missing_field("id")))?;

    // Parse timestamps (contract uses float seconds)
    let timestamp = data["create_time"]
        .as_f64()
        .map(|t| (t * 1000.0) as i64)
        .or_else(|| data["create_time_ms"].as_i64());

    // Parse status
    // Gate contract orders have both `status` and `finish_as` fields
    // - status: "open", "finished", etc.
    // - finish_as: "cancelled", "filled", etc. (only when status="finished")
    let status = if let Some(finish_as) = data["finish_as"].as_str() {
        // If finish_as exists, it provides more specific info
        match finish_as {
            "cancelled" => OrderStatus::Cancelled,
            "canceled" => OrderStatus::Cancelled,
            "filled" => OrderStatus::Closed,
            _ => OrderStatus::Closed,
        }
    } else if let Some(status_str) = data["status"].as_str() {
        match status_str {
            "closed" => OrderStatus::Closed,
            "cancelled" => OrderStatus::Cancelled,
            "canceled" => OrderStatus::Cancelled,
            "open" => OrderStatus::Open,
            "pending" => OrderStatus::Pending,
            "finished" => OrderStatus::Closed, // Default for finished without finish_as
            _ => OrderStatus::Open,
        }
    } else {
        OrderStatus::Open
    };

    // Parse side from size (positive=buy, negative=sell)
    let size = data["size"]
        .as_i64()
        .or_else(|| data["size"].as_f64().map(|s| s as i64))
        .ok_or_else(|| Error::from(ParseError::missing_field("size")))?;

    let side = if size > 0 {
        OrderSide::Buy
    } else {
        OrderSide::Sell
    };

    // Use absolute value for amount
    let amount = Decimal::from(size.unsigned_abs());

    // Parse price
    let price = parse_decimal(data, "price");

    // Parse filled amount from left
    let left = data["left"]
        .as_i64()
        .or_else(|| data["left"].as_f64().map(|l| l as i64))
        .unwrap_or(0);

    let filled = if left >= 0 {
        amount - Decimal::from(left as u64)
    } else {
        amount
    };

    // Calculate remaining
    let remaining = if filled < amount {
        Some(amount - filled)
    } else {
        Some(Decimal::ZERO)
    };

    // Parse time in force
    let time_in_force = if let Some(tif_str) = data["tif"].as_str() {
        match tif_str {
            "gtc" => Some(TimeInForce::GTC),
            "ioc" => Some(TimeInForce::IOC),
            "poc" => Some(TimeInForce::PO),
            "fok" => Some(TimeInForce::FOK),
            _ => Some(TimeInForce::GTC),
        }
    } else {
        Some(TimeInForce::GTC)
    };

    // Parse fee
    let fee_amount = parse_decimal(data, "fee");
    let fee = fee_amount.map(|amount| ccxt_core::types::Fee {
        currency: "USDT".to_string(), // Contract fees are typically in settle currency
        cost: amount,
        rate: None,
    });

    // Calculate cost
    let cost = if filled > Decimal::ZERO {
        if let Some(p) = price {
            Some(filled * p)
        } else {
            None
        }
    } else {
        Some(Decimal::ZERO)
    };

    // Parse client order ID
    let client_order_id = data["text"].as_str().map(|s| {
        if s.starts_with("t-") {
            s[2..].to_string()
        } else {
            s.to_string()
        }
    });

    let datetime = timestamp.map(|t| {
        chrono::DateTime::from_timestamp(t / 1000, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    });

    // Determine order type from price and tif
    // Market orders: price="0" and tif="ioc"
    let order_type = if price == Some(Decimal::ZERO) && time_in_force == Some(TimeInForce::IOC) {
        OrderType::Market
    } else {
        OrderType::Limit
    };

    let info = value_to_info(data);

    Ok(Order {
        id,
        client_order_id,
        timestamp,
        datetime,
        last_trade_timestamp: None,
        symbol,
        status,
        side,
        order_type,
        time_in_force: time_in_force.map(|t| t.to_string()),
        price,
        average: None,
        amount,
        filled: Some(filled),
        remaining,
        cost,
        trades: None,
        fee: fee.clone(),
        fees: fee.map(|f| vec![f]),
        post_only: None,
        reduce_only: data["is_reduce_only"].as_bool(),
        trigger_price: None,
        stop_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        info,
    })
}

/// Parse multiple contract orders from Gate.io futures API response.
pub fn parse_contract_orders(data: &Value, market: Option<&Market>) -> Result<Vec<Order>> {
    let orders_array = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

    orders_array
        .iter()
        .map(|order_data| parse_contract_order(order_data, market))
        .collect()
}

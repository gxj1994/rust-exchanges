//! WebSocket Order data parser for Binance.
//!
//! Parses executionReport and ORDER_TRADE_UPDATE events.

use ccxt_core::{
    error::{Error, Result},
    types::{Order, OrderSide, OrderStatus, OrderType, Symbol},
};
use serde_json::Value;
use std::collections::HashMap;

use super::{parse_decimal, to_unified_symbol, to_unified_symbol_with_mt};

/// Parse order from WebSocket executionReport message (spot).
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
pub fn parse_order(msg: &Value) -> Result<Order> {
    let symbol = Symbol::new_unchecked(to_unified_symbol(msg));

    let id = msg
        .get("i")
        .and_then(|i| i.as_u64())
        .map(|i| i.to_string())
        .unwrap_or_default();

    let client_order_id = msg.get("c").and_then(|c| c.as_str()).map(|s| s.to_string());

    let side = match msg.get("S").and_then(|s| s.as_str()) {
        Some("BUY") => OrderSide::Buy,
        Some("SELL") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let order_type = match msg.get("o").and_then(|o| o.as_str()) {
        Some("LIMIT") => OrderType::Limit,
        Some("MARKET") => OrderType::Market,
        _ => OrderType::Limit,
    };

    let price = parse_decimal(msg, "p");
    let amount = parse_decimal(msg, "q").unwrap_or_default();
    let filled = parse_decimal(msg, "z");

    let status = match msg.get("X").and_then(|x| x.as_str()) {
        Some("NEW") => OrderStatus::Open,
        Some("PARTIALLY_FILLED") => OrderStatus::Partial,
        Some("FILLED") => OrderStatus::Closed,
        Some("CANCELED") => OrderStatus::Cancelled,
        Some("EXPIRED") => OrderStatus::Expired,
        _ => OrderStatus::Open,
    };

    let timestamp = msg.get("E").and_then(|t| t.as_i64());

    Ok(Order {
        id,
        client_order_id,
        symbol,
        order_type,
        side,
        price,
        amount,
        filled,
        remaining: None,
        cost: None,
        average: None,
        status,
        fee: None,
        fees: None,
        timestamp,
        datetime: None,
        last_trade_timestamp: None,
        time_in_force: None,
        post_only: None,
        reduce_only: None,
        stop_price: None,
        trigger_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        trades: None,
        info: HashMap::new(),
    })
}

/// Parse order from WebSocket ORDER_TRADE_UPDATE message (futures).
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
pub fn parse_order_futures(msg: &Value) -> Result<Order> {
    let order_data = msg
        .get("o")
        .ok_or_else(|| Error::invalid_request("Missing order data 'o' in ORDER_TRADE_UPDATE"))?;

    let symbol_str = order_data
        .get("s")
        .and_then(|s| s.as_str())
        .unwrap_or_default();
    let mt = msg.get("_ccxt_mt").and_then(|v| v.as_str());
    let symbol = Symbol::new_unchecked(to_unified_symbol_with_mt(symbol_str, mt));

    let id = order_data
        .get("i")
        .and_then(|i| i.as_u64())
        .map(|i| i.to_string())
        .unwrap_or_default();

    let client_order_id = order_data
        .get("c")
        .and_then(|c| c.as_str())
        .map(|s| s.to_string());

    let side = match order_data.get("S").and_then(|s| s.as_str()) {
        Some("BUY") => OrderSide::Buy,
        Some("SELL") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let order_type = match order_data.get("o").and_then(|o| o.as_str()) {
        Some("LIMIT") => OrderType::Limit,
        Some("MARKET") => OrderType::Market,
        _ => OrderType::Limit,
    };

    let price = parse_decimal(order_data, "p");
    let amount = parse_decimal(order_data, "q").unwrap_or_default();
    let filled = parse_decimal(order_data, "z");

    let status = match order_data.get("X").and_then(|x| x.as_str()) {
        Some("NEW") => OrderStatus::Open,
        Some("PARTIALLY_FILLED") => OrderStatus::Partial,
        Some("FILLED") => OrderStatus::Closed,
        Some("CANCELED") => OrderStatus::Cancelled,
        Some("EXPIRED") => OrderStatus::Expired,
        _ => OrderStatus::Open,
    };

    let timestamp = order_data.get("T").and_then(|t| t.as_i64());

    let working_type = order_data
        .get("wt")
        .and_then(|wt| wt.as_str())
        .map(|s| s.to_string());

    Ok(Order {
        id,
        client_order_id,
        symbol,
        order_type,
        side,
        price,
        amount,
        filled,
        remaining: None,
        cost: None,
        average: None,
        status,
        fee: None,
        fees: None,
        timestamp,
        datetime: None,
        last_trade_timestamp: None,
        time_in_force: None,
        post_only: None,
        reduce_only: None,
        stop_price: None,
        trigger_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type,
        trades: None,
        info: HashMap::new(),
    })
}

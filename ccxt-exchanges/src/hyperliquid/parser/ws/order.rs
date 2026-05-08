//! WebSocket Order data parser for HyperLiquid.
//!
//! Parses orderUpdates and userEvents channel messages.

use ccxt_core::{
    error::Result,
    types::{Order, OrderSide, OrderStatus, OrderType, Symbol},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use crate::hyperliquid::core::symbol::HyperliquidSymbolConverter;

/// Parse order from WebSocket message data.
///
/// # Arguments
///
/// * `data` - The order data from WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
pub fn parse_order(data: &Value) -> Result<Order> {
    parse_order_from_data(data)
}

/// Parse order from raw data.
///
/// Used by both orderUpdates and userEvents channels.
pub fn parse_order_from_data(data: &Value) -> Result<Order> {
    let coin = data
        .get("coin")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(HyperliquidSymbolConverter::exchange_to_unified_inferred(
        coin,
    ));

    let id = data
        .get("oid")
        .and_then(|o| o.as_u64())
        .map(|o| o.to_string())
        .unwrap_or_default();

    let side = match data.get("side").and_then(|s| s.as_str()).unwrap_or("B") {
        "B" | "buy" => OrderSide::Buy,
        "A" | "sell" => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let order_type = match data.get("orderType").and_then(|o| o.as_str()) {
        Some("market") => OrderType::Market,
        Some("limit") => OrderType::Limit,
        _ => OrderType::Limit,
    };

    let price = parse_decimal(data, "limitPx");
    let amount = parse_decimal(data, "sz").unwrap_or_default();
    let filled = parse_decimal(data, "filled");

    let status = match data.get("status").and_then(|s| s.as_str()) {
        Some("filled") => OrderStatus::Closed,
        Some("canceled") => OrderStatus::Cancelled,
        Some("open") => OrderStatus::Open,
        _ => OrderStatus::Open,
    };

    let timestamp = data.get("timestamp").and_then(|t| t.as_i64());

    Ok(Order {
        id,
        client_order_id: None,
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

/// Parse order update from orderUpdates channel.
///
/// # Arguments
///
/// * `data` - The `data` field from WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
pub fn parse_order_update(data: &Value) -> Result<Order> {
    let coin = data
        .get("coin")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(HyperliquidSymbolConverter::exchange_to_unified_inferred(
        coin,
    ));

    let id = data
        .get("oid")
        .and_then(|o| o.as_u64())
        .map(|o| o.to_string())
        .unwrap_or_default();

    let side = match data.get("side").and_then(|s| s.as_str()).unwrap_or("B") {
        "B" | "buy" => OrderSide::Buy,
        "A" | "sell" => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let order_type = match data.get("orderType").and_then(|o| o.as_str()) {
        Some("market") => OrderType::Market,
        Some("limit") => OrderType::Limit,
        _ => OrderType::Limit,
    };

    let price = parse_decimal(data, "limitPx");
    let amount = parse_decimal(data, "sz").unwrap_or_default();
    let filled = parse_decimal(data, "filled");

    let status = match data.get("status").and_then(|s| s.as_str()) {
        Some("filled") => OrderStatus::Closed,
        Some("canceled") => OrderStatus::Cancelled,
        Some("open") => OrderStatus::Open,
        _ => OrderStatus::Open,
    };

    let timestamp = data.get("timestamp").and_then(|t| t.as_i64());

    Ok(Order {
        id,
        client_order_id: None,
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

/// Parse decimal value from a field.
fn parse_decimal(data: &Value, field: &str) -> Option<Decimal> {
    data.get(field)
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
}

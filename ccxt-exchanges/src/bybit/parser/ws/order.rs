//! WebSocket Order data parser for Bybit.

use ccxt_core::{
    error::{Error, Result},
    types::{Order, OrderSide, OrderStatus, OrderType, Symbol},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use super::{extract_data, to_unified_symbol};

/// Parse order from WebSocket message.
pub fn parse_order(msg: &Value) -> Result<Order> {
    let data = extract_data(msg)?;
    if let Some(first) = data.as_array().and_then(|arr| arr.first()) {
        parse_order_from_data(first)
    } else {
        Err(Error::invalid_request("No order data found"))
    }
}

/// Parse order from data object.
pub fn parse_order_from_data(data: &Value) -> Result<Order> {
    let symbol_str = data
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(symbol_str, None));

    let id = data
        .get("orderId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let client_order_id = data
        .get("orderLinkId")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let side = match data.get("side").and_then(|v| v.as_str()) {
        Some("Sell") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let order_type = match data.get("orderType").and_then(|v| v.as_str()) {
        Some("Market") => OrderType::Market,
        _ => OrderType::Limit,
    };

    let price = data
        .get("price")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok());

    let amount = data
        .get("qty")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let filled = data
        .get("cumExecQty")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok());

    let average = data
        .get("avgPrice")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .filter(|d| !d.is_zero());

    let cost = match (filled, average) {
        (Some(f), Some(a)) => Some(f * a),
        _ => None,
    };

    let status = match data.get("orderStatus").and_then(|v| v.as_str()) {
        Some("Filled") => OrderStatus::Closed,
        Some("Cancelled") => OrderStatus::Cancelled,
        _ => OrderStatus::Open,
    };

    let timestamp = data
        .get("createdTime")
        .and_then(|t| t.as_str())
        .and_then(|s| s.parse::<i64>().ok());

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
        cost,
        average,
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

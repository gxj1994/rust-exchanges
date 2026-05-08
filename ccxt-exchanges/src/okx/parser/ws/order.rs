//! WebSocket Order data parser for OKX.

use ccxt_core::{
    error::Result,
    types::{
        Fee, Order, OrderSide, OrderStatus, OrderType, Symbol, Trade,
        financial::{Amount, Cost, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use super::{extract_first_data, to_unified_symbol};

/// Parse order from WebSocket message.
pub fn parse_order(msg: &Value) -> Result<Order> {
    let data = extract_first_data(msg)?;
    parse_order_from_data(data)
}

/// Parse order from data object.
pub fn parse_order_from_data(data: &Value) -> Result<Order> {
    let inst_id = data
        .get("instId")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(inst_id));

    let id = data
        .get("ordId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let client_order_id = data
        .get("clOrdId")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let side = match data.get("side").and_then(|v| v.as_str()) {
        Some("sell") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let order_type = match data.get("ordType").and_then(|v| v.as_str()) {
        Some("market") => OrderType::Market,
        _ => OrderType::Limit,
    };

    let price = data
        .get("px")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok());

    let amount = data
        .get("sz")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let filled = data
        .get("accFillSz")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok());

    let average = data
        .get("avgPx")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .filter(|d| !d.is_zero());

    let cost = match (filled, average) {
        (Some(f), Some(a)) => Some(f * a),
        _ => None,
    };

    let status = match data.get("state").and_then(|v| v.as_str()) {
        Some("filled") => OrderStatus::Closed,
        Some("canceled" | "cancelled") => OrderStatus::Cancelled,
        _ => OrderStatus::Open,
    };

    let fee = {
        let fee_amount = data
            .get("fee")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());
        let fee_currency = data
            .get("feeCcy")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        fee_amount.map(|f| Fee::new(fee_currency, f.abs()))
    };

    let timestamp = data
        .get("cTime")
        .and_then(|v| v.as_str())
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
        fee,
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

/// Parse a WebSocket order fill into a Trade (my trade).
///
/// Extracts the latest fill information from an order update.
pub fn parse_ws_account_trade(data: &Value) -> Result<Trade> {
    let inst_id = data
        .get("instId")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(inst_id.replace('-', "/"));

    let trade_id = data
        .get("tradeId")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let order_id = data
        .get("ordId")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let side = match data.get("side").and_then(|v| v.as_str()) {
        Some("buy") => OrderSide::Buy,
        _ => OrderSide::Sell,
    };

    let fill_px = data
        .get("fillPx")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let fill_sz = data
        .get("fillSz")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or_default();

    let timestamp = data
        .get("fillTime")
        .or_else(|| data.get("uTime"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let fee = {
        let fee_amount = data
            .get("fillFee")
            .or_else(|| data.get("fee"))
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok());
        let fee_currency = data
            .get("fillFeeCcy")
            .or_else(|| data.get("feeCcy"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        fee_amount.map(|f| Fee::new(fee_currency, f.abs()))
    };

    Ok(Trade {
        id: trade_id,
        order: order_id,
        symbol,
        trade_type: None,
        side,
        taker_or_maker: None,
        price: Price(fill_px),
        amount: Amount(fill_sz),
        cost: Some(Cost(fill_px * fill_sz)),
        fee,
        timestamp,
        datetime: None,
        info: HashMap::new(),
    })
}

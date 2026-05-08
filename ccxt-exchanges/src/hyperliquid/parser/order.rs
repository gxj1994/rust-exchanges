//! Order data parser for HyperLiquid.

use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{Order, OrderSide, OrderStatus, OrderType, Symbol},
};
use rust_decimal::Decimal;
use serde_json::Value;

use super::timestamp_to_datetime;
use crate::common::parser_helpers::ParseHelper;

/// Parse order status from HyperLiquid status string.
///
/// # Arguments
///
/// * `status` - HyperLiquid order status string
///
/// # Returns
///
/// Returns the corresponding CCXT [`OrderStatus`].
pub fn parse_order_status(status: &str) -> OrderStatus {
    match status.to_lowercase().as_str() {
        "filled" => OrderStatus::Closed,
        "canceled" | "cancelled" => OrderStatus::Cancelled,
        "rejected" => OrderStatus::Rejected,
        "pending" => OrderStatus::Pending,
        _ => OrderStatus::Open,
    }
}

/// Parse order data from HyperLiquid order response.
///
/// # Arguments
///
/// * `data` - HyperLiquid order data JSON object
/// * `market` - Optional market info for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
pub fn parse_order(data: &Value, market: Option<&ccxt_core::types::Market>) -> Result<Order> {
    let symbol: Symbol = market.map_or_else(
        || {
            data["coin"].as_str().map_or_else(
                || Symbol::default(),
                |c| Symbol::new_unchecked(format!("{}/USDC:USDC", c)),
            )
        },
        |m| m.symbol.clone(),
    );

    let id = data["oid"]
        .as_u64()
        .map(|n| n.to_string())
        .or_else(|| data["oid"].as_str().map(ToString::to_string))
        .unwrap_or_default();

    let timestamp = parse_timestamp(data, "timestamp");

    let status_str = data["status"].as_str().unwrap_or("open");
    let status = parse_order_status(status_str);

    let side = match data["side"].as_str() {
        Some("B" | "buy") => OrderSide::Buy,
        Some("A" | "sell") => OrderSide::Sell,
        _ => {
            // Check isBuy field
            if data["isBuy"].as_bool().unwrap_or(true) {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            }
        }
    };

    let order_type = match data["orderType"].as_str() {
        Some("Market" | "market") => OrderType::Market,
        _ => OrderType::Limit,
    };

    let price = ParseHelper::decimal_any(data, &["limitPx", "px"]);
    let amount = ParseHelper::decimal_any(data, &["sz", "origSz"]).unwrap_or(Decimal::ZERO);
    let filled = parse_decimal(data, "filledSz");
    let remaining = filled.map(|f| amount - f);

    Ok(Order {
        id,
        client_order_id: data["cloid"].as_str().map(ToString::to_string),
        timestamp,
        datetime: timestamp.and_then(timestamp_to_datetime),
        last_trade_timestamp: None,
        status,
        symbol,
        order_type,
        time_in_force: data["tif"].as_str().map(str::to_uppercase),
        side,
        price,
        average: parse_decimal(data, "avgPx"),
        amount,
        filled,
        remaining,
        cost: None,
        trades: None,
        fee: None,
        post_only: None,
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPx"),
        stop_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

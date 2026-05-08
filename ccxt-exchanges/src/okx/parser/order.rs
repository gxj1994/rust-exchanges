//! Order parser for OKX.

use crate::common::parser_helpers::ParseHelper;
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{Market, Order, OrderSide, OrderStatus, OrderType, Symbol},
};
use serde_json::Value;

/// Map OKX order status to CCXT OrderStatus.
///
/// OKX order states:
/// - live: Order is active
/// - partially_filled: Order is partially filled
/// - filled: Order is completely filled
/// - canceled: Order is canceled
/// - mmp_canceled: Order is canceled by MMP
///
/// # Arguments
///
/// * `status` - OKX order status string
///
/// # Returns
///
/// Returns the corresponding CCXT [`OrderStatus`].
pub fn parse_order_status(status: &str) -> OrderStatus {
    match status.to_lowercase().as_str() {
        "filled" => OrderStatus::Closed,
        "canceled" | "cancelled" | "mmp_canceled" => OrderStatus::Cancelled,
        "expired" => OrderStatus::Expired,
        "rejected" => OrderStatus::Rejected,
        "pending" => OrderStatus::Pending,
        _ => OrderStatus::Open, // Default to Open for unknown statuses
    }
}

/// Parse order data from OKX order response.
///
/// # Arguments
///
/// * `data` - OKX order data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
///
/// # Note
///
/// OKX's place-order API may return minimal response with only ordId and clOrdId.
/// When this happens, we return an Order with `Pending` status.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    // Check if this is a minimal response (only ordId, no sz field)
    let has_size = data["sz"].is_string();

    if !has_size {
        // Minimal response: create a Pending order
        let order_id = data["ordId"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("ordId")))?
            .to_string();

        let client_order_id = data["clOrdId"].as_str().map(ToString::to_string);

        let symbol = if let Some(m) = market {
            m.symbol.clone()
        } else {
            data["instId"].as_str().map_or_else(
                || Symbol::new_unchecked(""),
                |s| Symbol::new_unchecked(s.replace('-', "/")),
            )
        };

        // Return Pending order - caller should fetch full details
        return Ok(Order {
            id: order_id,
            client_order_id,
            symbol,
            status: OrderStatus::Pending,
            // All other fields are unknown at this point
            timestamp: None,
            datetime: None,
            last_trade_timestamp: None,
            order_type: OrderType::Market,
            side: OrderSide::Buy,
            time_in_force: None,
            post_only: None,
            reduce_only: None,
            price: None,
            stop_price: None,
            trigger_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            amount: rust_decimal::Decimal::ZERO,
            filled: None,
            remaining: None,
            cost: None,
            average: None,
            fee: None,
            fees: None,
            trades: None,
            info: std::collections::HashMap::new(),
        });
    }

    // Full response: parse normally
    parse_order_full(data, market)
}

/// Parse complete order data from OKX order response.
fn parse_order_full(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["instId"].as_str().map_or_else(
            || Symbol::new_unchecked(""),
            |s| Symbol::new_unchecked(s.replace('-', "/")),
        )
    };

    let id = data["ordId"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("ordId")))?
        .to_string();

    let timestamp = parse_timestamp(data, "cTime").or_else(|| parse_timestamp(data, "ts"));

    let status_str = data["state"].as_str().unwrap_or("live");
    let status = parse_order_status(status_str);

    // Parse order side
    let side = match data["side"].as_str() {
        Some("buy" | "Buy" | "BUY") => OrderSide::Buy,
        Some("sell" | "Sell" | "SELL") => OrderSide::Sell,
        _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
    };

    // Parse order type
    let order_type = match data["ordType"].as_str() {
        Some("market" | "Market" | "MARKET") => OrderType::Market,
        Some("post_only") => OrderType::LimitMaker,
        _ => OrderType::Limit, // Default to limit (covers limit, fok, ioc)
    };

    let price = parse_decimal(data, "px");
    let amount =
        parse_decimal(data, "sz").ok_or_else(|| Error::from(ParseError::missing_field("sz")))?;
    let filled = ParseHelper::decimal_any(data, &["accFillSz", "fillSz"]);
    let remaining = match filled {
        Some(f) => Some(amount - f),
        None => Some(amount),
    };

    let average = ParseHelper::decimal_any(data, &["avgPx", "fillPx"]);

    // Calculate cost from filled amount and average price
    let cost = match (filled, average) {
        (Some(f), Some(avg)) => Some(f * avg),
        _ => None,
    };

    Ok(Order {
        id,
        client_order_id: data["clOrdId"].as_str().map(ToString::to_string),
        timestamp,
        datetime: timestamp.and_then(ccxt_core::parser_utils::timestamp_to_datetime),
        last_trade_timestamp: parse_timestamp(data, "uTime"),
        status,
        symbol,
        order_type,
        time_in_force: data["ordType"].as_str().map(|s| match s {
            "fok" => "FOK".to_string(),
            "ioc" => "IOC".to_string(),
            "post_only" => "PO".to_string(),
            _ => "GTC".to_string(),
        }),
        side,
        price,
        average,
        amount,
        filled,
        remaining,
        cost,
        trades: None,
        fee: None,
        post_only: Some(data["ordType"].as_str() == Some("post_only")),
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPx"),
        stop_price: parse_decimal(data, "slTriggerPx"),
        take_profit_price: parse_decimal(data, "tpTriggerPx"),
        stop_loss_price: parse_decimal(data, "slTriggerPx"),
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_order_status() {
        assert_eq!(parse_order_status("live"), OrderStatus::Open);
        assert_eq!(parse_order_status("partially_filled"), OrderStatus::Open);
        assert_eq!(parse_order_status("filled"), OrderStatus::Closed);
        assert_eq!(parse_order_status("canceled"), OrderStatus::Cancelled);
        assert_eq!(parse_order_status("mmp_canceled"), OrderStatus::Cancelled);
        assert_eq!(parse_order_status("expired"), OrderStatus::Expired);
        assert_eq!(parse_order_status("rejected"), OrderStatus::Rejected);
    }

    #[test]
    fn test_parse_order() {
        let data = json!({
            "ordId": "123456789",
            "instId": "BTC-USDT",
            "side": "buy",
            "ordType": "limit",
            "px": "50000.00",
            "sz": "0.5",
            "state": "live",
            "cTime": "1700000000000"
        });

        let order = parse_order(&data, None).unwrap();
        assert_eq!(order.id, "123456789");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, Some(dec!(50000.00)));
        assert_eq!(order.amount, dec!(0.5));
        assert_eq!(order.status, OrderStatus::Open);
    }
}

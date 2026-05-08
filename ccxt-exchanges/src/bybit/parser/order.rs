//! Order parser for Bybit.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{Market, Order, OrderSide, OrderStatus, OrderType, Symbol},
};
use serde_json::Value;

/// Map Bybit order status to CCXT OrderStatus.
///
/// Bybit order states:
/// - New: Order is active
/// - PartiallyFilled: Order is partially filled
/// - Filled: Order is completely filled
/// - Cancelled: Order is canceled
/// - Rejected: Order is rejected
/// - PartiallyFilledCanceled: Partially filled then canceled
///
/// # Arguments
///
/// * `status` - Bybit order status string
///
/// # Returns
///
/// Returns the corresponding CCXT [`OrderStatus`].
pub fn parse_order_status(status: &str) -> OrderStatus {
    match status {
        "Filled" => OrderStatus::Closed,
        "Cancelled" | "Canceled" | "PartiallyFilledCanceled" | "Deactivated" => {
            OrderStatus::Cancelled
        }
        "Rejected" => OrderStatus::Rejected,
        "Expired" => OrderStatus::Expired,
        "Pending" => OrderStatus::Pending,
        _ => OrderStatus::Open, // Default to Open for unknown statuses
    }
}

/// Parse order data from Bybit order response.
///
/// # Arguments
///
/// * `data` - Bybit order data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        // Check both "symbol" (REST API) and "s" (WebSocket) fields
        data["symbol"]
            .as_str()
            .or_else(|| data["s"].as_str())
            .map(|s| Symbol::new_unchecked(s))
            .ok_or_else(|| {
                ccxt_core::Error::from(ccxt_core::ParseError::missing_field("symbol/s"))
                    .context("Failed to parse: missing symbol identifier")
            })?
    };

    let id = data["orderId"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?
        .to_string();

    let timestamp =
        parse_timestamp(data, "createdTime").or_else(|| parse_timestamp(data, "createTime"));

    let status_str = data["orderStatus"].as_str().unwrap_or("New");
    let status = parse_order_status(status_str);

    // Parse order side
    let side = match data["side"].as_str() {
        Some("Buy" | "buy" | "BUY") => OrderSide::Buy,
        Some("Sell" | "sell" | "SELL") => OrderSide::Sell,
        _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
    };

    // Parse order type
    // Bybit returns "Limit" for LimitMaker orders, but with timeInForce="PostOnly"
    // We need to check both fields to correctly identify LimitMaker orders
    let order_type_str = data["orderType"].as_str().unwrap_or("");
    let time_in_force_str = data["timeInForce"].as_str().unwrap_or("");

    let order_type = if order_type_str == "Market" {
        OrderType::Market
    } else if order_type_str == "Limit" && time_in_force_str == "PostOnly" {
        // Limit + PostOnly = LimitMaker
        OrderType::LimitMaker
    } else {
        OrderType::Limit // Default to limit
    };

    let price = parse_decimal(data, "price");
    let amount =
        parse_decimal(data, "qty").ok_or_else(|| Error::from(ParseError::missing_field("qty")))?;
    let filled = parse_decimal(data, "cumExecQty");
    let remaining = match filled {
        Some(f) => Some(amount - f),
        None => Some(amount),
    };

    let average = parse_decimal(data, "avgPrice");

    // Calculate cost from filled amount and average price
    let cost = parse_decimal(data, "cumExecValue").or_else(|| match (filled, average) {
        (Some(f), Some(avg)) => Some(f * avg),
        _ => None,
    });

    Ok(Order {
        id,
        client_order_id: data["orderLinkId"].as_str().map(ToString::to_string),
        timestamp,
        datetime: timestamp.and_then(ccxt_core::parser_utils::timestamp_to_datetime),
        last_trade_timestamp: parse_timestamp(data, "updatedTime"),
        status,
        symbol,
        order_type,
        time_in_force: data["timeInForce"].as_str().map(ToString::to_string),
        side,
        price,
        average,
        amount,
        filled,
        remaining,
        cost,
        trades: None,
        fee: None,
        post_only: data["timeInForce"].as_str().map(|s| s == "PostOnly"),
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPrice"),
        stop_price: parse_decimal(data, "stopLoss"),
        take_profit_price: parse_decimal(data, "takeProfit"),
        stop_loss_price: parse_decimal(data, "stopLoss"),
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
        assert_eq!(parse_order_status("New"), OrderStatus::Open);
        assert_eq!(parse_order_status("PartiallyFilled"), OrderStatus::Open);
        assert_eq!(parse_order_status("Filled"), OrderStatus::Closed);
        assert_eq!(parse_order_status("Cancelled"), OrderStatus::Cancelled);
        assert_eq!(parse_order_status("Rejected"), OrderStatus::Rejected);
        assert_eq!(parse_order_status("Expired"), OrderStatus::Expired);
    }

    #[test]
    fn test_parse_order() {
        let data = json!({
            "orderId": "123456789",
            "symbol": "BTCUSDT",
            "side": "Buy",
            "orderType": "Limit",
            "price": "50000.00",
            "qty": "0.5",
            "orderStatus": "New",
            "createdTime": "1700000000000"
        });

        let order = parse_order(&data, None).unwrap();
        assert_eq!(order.id, "123456789");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, Some(dec!(50000.00)));
        assert_eq!(order.amount, dec!(0.5));
        assert_eq!(order.status, OrderStatus::Open);
    }

    #[test]
    fn test_parse_order_limit_maker() {
        // Bybit returns orderType="Limit" with timeInForce="PostOnly" for LimitMaker orders
        let data = json!({
            "orderId": "987654321",
            "symbol": "ETHUSDT",
            "side": "Sell",
            "orderType": "Limit",
            "timeInForce": "PostOnly",
            "price": "3000.00",
            "qty": "1.0",
            "orderStatus": "New",
            "createdTime": "1700000000000"
        });

        let order = parse_order(&data, None).unwrap();
        assert_eq!(order.id, "987654321");
        assert_eq!(order.side, OrderSide::Sell);
        // Should be parsed as LimitMaker because timeInForce is PostOnly
        assert_eq!(order.order_type, OrderType::LimitMaker);
        assert_eq!(order.price, Some(dec!(3000.00)));
        assert_eq!(order.amount, dec!(1.0));
        assert_eq!(order.post_only, Some(true));
    }

    #[test]
    fn test_parse_order_market() {
        let data = json!({
            "orderId": "555555555",
            "symbol": "BTCUSDT",
            "side": "Buy",
            "orderType": "Market",
            "qty": "0.01",
            "orderStatus": "Filled",
            "createdTime": "1700000000000",
            "avgPrice": "50000.00"
        });

        let order = parse_order(&data, None).unwrap();
        assert_eq!(order.id, "555555555");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.status, OrderStatus::Closed);
        assert_eq!(order.average, Some(dec!(50000.00)));
    }
}

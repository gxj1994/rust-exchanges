//! Order data parser for Bitget.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{Market, Order, OrderSide, OrderStatus, OrderType, Symbol},
};
use serde_json::Value;

use super::timestamp_to_datetime;

/// Map Bitget order status to CCXT OrderStatus.
///
/// # Arguments
///
/// * `status` - Bitget order status string
///
/// # Returns
///
/// Returns the corresponding CCXT [`OrderStatus`].
pub fn parse_order_status(status: &str) -> OrderStatus {
    match status.to_lowercase().as_str() {
        "new" | "live" | "init" | "partially_filled" => OrderStatus::Open,
        "filled" | "full_fill" | "full-fill" => OrderStatus::Closed,
        "cancelled" | "canceled" | "cancel" => OrderStatus::Cancelled,
        "expired" | "expire" => OrderStatus::Expired,
        "rejected" | "reject" => OrderStatus::Rejected,
        _ => OrderStatus::Open, // Default to Open for unknown statuses
    }
}

/// Parse order data from Bitget order response.
///
/// # Arguments
///
/// * `data` - Bitget order data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Order`] structure.
///
/// # Note
///
/// Bitget's place-order API only returns `{orderId, clientOid}` (minimal response).
/// When this happens, we return an Order with `Pending` status. The caller should
/// then call `fetch_order` to get complete order details.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    // Check if this is a minimal response (only orderId and clientOid)
    // Bitget's place-order API returns only these two fields
    // V3 API may also return minimal response or full response
    let has_size = data["size"].is_string() 
        || data["baseVolume"].is_string() 
        || data["amount"].is_string()  // V3 API uses 'amount'
        || data["qty"].is_string()  // V3 API may use 'qty'
        || data["orderStatus"].is_string(); // V3 API includes orderStatus in full response

    if !has_size {
        // Minimal response: create a Pending order
        let order_id = data["orderId"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?
            .to_string();

        let client_order_id = data["clientOid"]
            .as_str()
            .or_else(|| data["clientOrderId"].as_str())
            .map(ToString::to_string);

        let symbol = if let Some(m) = market {
            m.symbol.clone()
        } else {
            return Err(Error::from(ParseError::missing_field("symbol"))
                .context("Cannot determine symbol from minimal order response"));
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
            order_type: OrderType::Market, // Will be updated after fetch
            side: OrderSide::Buy,          // Will be updated after fetch
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

/// Parse complete order data from Bitget order response.
fn parse_order_full(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        data["symbol"]
            .as_str()
            .or_else(|| data["instId"].as_str())
            .map(|s| Symbol::new_unchecked(s))
            .ok_or_else(|| {
                ccxt_core::Error::from(ccxt_core::ParseError::missing_field("symbol/instId"))
                    .context("Failed to parse order: missing symbol identifier")
            })?
    };

    let id = data["orderId"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?
        .to_string();

    // Parse timestamps - support both V2 and V3 API field names
    let timestamp = parse_timestamp(data, "cTime")
        .or_else(|| parse_timestamp(data, "createTime"))
        .or_else(|| parse_timestamp(data, "createdTime")) // V3 API
        .or_else(|| parse_timestamp(data, "ts"));

    // Parse order status - support both V2 and V3 API field names
    let status_str = data["orderStatus"] // V3 API
        .as_str()
        .or_else(|| data["status"].as_str()) // V2 API
        .or_else(|| data["state"].as_str()) // Alternative
        .unwrap_or("live");
    let status = parse_order_status(status_str);

    // Parse order side
    let side = match data["side"].as_str() {
        Some("buy" | "Buy" | "BUY") => OrderSide::Buy,
        Some("sell" | "Sell" | "SELL") => OrderSide::Sell,
        _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
    };

    // Parse order type
    let order_type = match data["orderType"].as_str().or_else(|| data["type"].as_str()) {
        Some("market" | "Market" | "MARKET") => OrderType::Market,
        Some("limit_maker" | "post_only") => OrderType::LimitMaker,
        _ => OrderType::Limit, // Default to limit
    };

    let price = parse_decimal(data, "price").or_else(|| parse_decimal(data, "priceAvg"));
    // V3 API uses 'amount' or 'qty' for order quantity, V2 uses 'size'
    let amount = parse_decimal(data, "amount")
        .or_else(|| parse_decimal(data, "qty")) // V3 API field
        .or_else(|| parse_decimal(data, "size"))
        .or_else(|| parse_decimal(data, "baseVolume"))
        .ok_or_else(|| Error::from(ParseError::missing_field("amount/size")))?;
    // V3 API uses 'cumExecQty' for filled quantity
    let filled = parse_decimal(data, "cumExecQty")
        .or_else(|| parse_decimal(data, "fillSize"))
        .or_else(|| parse_decimal(data, "baseVolume"));
    let remaining = match filled {
        Some(f) => Some(amount - f),
        None => Some(amount),
    };

    // V3 API uses 'cumExecValue' for filled value in USDT
    let cost = parse_decimal(data, "cumExecValue")
        .or_else(|| parse_decimal(data, "fillNotionalUsd"))
        .or_else(|| parse_decimal(data, "quoteVolume"));

    // V3 API uses 'avgPrice' for average fill price
    let average = parse_decimal(data, "avgPrice")
        .or_else(|| parse_decimal(data, "priceAvg"))
        .or_else(|| parse_decimal(data, "fillPrice"));

    Ok(Order {
        id,
        client_order_id: data["clientOid"]
            .as_str()
            .or_else(|| data["clientOrderId"].as_str())
            .map(ToString::to_string),
        timestamp,
        datetime: timestamp.and_then(timestamp_to_datetime),
        last_trade_timestamp: parse_timestamp(data, "uTime")
            .or_else(|| parse_timestamp(data, "updateTime"))
            .or_else(|| parse_timestamp(data, "updatedTime")), // V3 API
        status,
        symbol,
        order_type,
        time_in_force: data["timeInForce"]
            .as_str()
            .or_else(|| data["force"].as_str())
            .map(str::to_uppercase),
        side,
        price,
        average,
        amount,
        filled,
        remaining,
        cost,
        trades: None,
        fee: None,
        // Parse post_only from force field, timeInForce, or orderType
        // Note: Bitget may not return force/timeInForce in order query response
        // If orderType is "post_only", it's definitely post_only
        post_only: data["force"]
            .as_str()
            .map(|f| f == "post_only")
            .or(data["timeInForce"].as_str().map(|f| f == "PO"))
            .or(data["orderType"].as_str().map(|t| t == "post_only")),
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPrice"),
        stop_price: parse_decimal(data, "stopPrice")
            .or_else(|| parse_decimal(data, "presetStopLossPrice"))
            .or_else(|| parse_decimal(data, "stopLoss")), // V3 API
        take_profit_price: parse_decimal(data, "presetTakeProfitPrice")
            .or_else(|| parse_decimal(data, "takeProfit")), // V3 API
        stop_loss_price: parse_decimal(data, "presetStopLossPrice")
            .or_else(|| parse_decimal(data, "stopLoss")), // V3 API
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

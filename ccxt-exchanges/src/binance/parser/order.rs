#![allow(dead_code)]

use super::{parse_decimal, value_to_hashmap};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        Market, OcoOrder, OcoOrderInfo, Order, OrderReport, OrderSide, OrderStatus, OrderType,
        Symbol, TimeInForce,
    },
};
use rust_decimal::Decimal;
use serde_json::{Value, json};

/// Parse order data from Binance order response.
pub fn parse_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        Symbol::new_unchecked(
            data["symbol"]
                .as_str()
                .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?,
        )
    };

    let id = data["orderId"]
        .as_u64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?
        .to_string();

    let timestamp = data["time"]
        .as_i64()
        .or_else(|| data["transactTime"].as_i64());

    // status 字段在某些情况下可能缺失（如测试网的 LIMIT_MAKER 订单）
    // 如果缺失，默认为 Open（新订单）
    let status = if let Some(status_str) = data["status"].as_str() {
        match status_str {
            "FILLED" => OrderStatus::Closed,
            "CANCELED" => OrderStatus::Cancelled,
            "EXPIRED" => OrderStatus::Expired,
            "REJECTED" => OrderStatus::Rejected,
            _ => OrderStatus::Open,
        }
    } else {
        // 默认状态为 Pending（新创建的订单）
        OrderStatus::Pending
    };

    // side 字段在某些情况下可能缺失（如测试网响应）
    // 如果缺失，尝试从请求中推断或使用默认值
    let side = if let Some(side_str) = data["side"].as_str() {
        match side_str {
            "BUY" => OrderSide::Buy,
            "SELL" => OrderSide::Sell,
            _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
        }
    } else {
        // 如果 API 没有返回 side，这通常是个问题，但为了兼容性我们记录警告
        // 这里暂时默认为 Buy，但实际应该从请求中获取
        tracing::warn!("Binance API response missing 'side' field");
        OrderSide::Buy // 默认值，实际情况应该从请求上下文获取
    };

    let order_type = match data["type"].as_str() {
        Some("MARKET") => OrderType::Market,
        Some("STOP_LOSS") => OrderType::StopLoss,
        Some("STOP_LOSS_LIMIT") => OrderType::StopLossLimit,
        Some("TAKE_PROFIT") => OrderType::TakeProfit,
        Some("TAKE_PROFIT_LIMIT" | "TAKE_PROFIT_MARKET") => OrderType::TakeProfitLimit,
        Some("STOP_MARKET" | "STOP") => OrderType::StopMarket,
        Some("TRAILING_STOP_MARKET") => OrderType::TrailingStop,
        Some("LIMIT_MAKER") => OrderType::LimitMaker,
        _ => OrderType::Limit,
    };

    let time_in_force = match data["timeInForce"].as_str() {
        Some("GTC") => Some(TimeInForce::GTC),
        Some("IOC") => Some(TimeInForce::IOC),
        Some("FOK") => Some(TimeInForce::FOK),
        Some("GTX") => Some(TimeInForce::PO),
        _ => None,
    };

    let price = parse_decimal(data, "price");
    let amount = parse_decimal(data, "origQty");
    let filled = parse_decimal(data, "executedQty");
    let remaining = match (&amount, &filled) {
        (Some(a), Some(f)) => Some(*a - *f),
        _ => None,
    };

    let cost = parse_decimal(data, "cummulativeQuoteQty");

    let average = match (&cost, &filled) {
        (Some(c), Some(f)) if !f.is_zero() => Some(*c / *f),
        _ => None,
    };

    Ok(Order {
        id,
        client_order_id: data["clientOrderId"].as_str().map(ToString::to_string),
        timestamp,
        datetime: timestamp.map(|t| {
            chrono::DateTime::from_timestamp(t / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        last_trade_timestamp: data["updateTime"].as_i64(),
        status,
        symbol,
        order_type,
        time_in_force: time_in_force.map(|t| t.to_string()),
        side,
        price,
        average,
        amount: amount.ok_or_else(|| Error::from(ParseError::missing_field("amount")))?,
        filled,
        remaining,
        cost,
        trades: None,
        fee: None,
        post_only: None,
        reduce_only: data["reduceOnly"].as_bool(),
        trigger_price: parse_decimal(data, "triggerPrice"),
        stop_price: parse_decimal(data, "stopPrice"),
        take_profit_price: parse_decimal(data, "takeProfitPrice"),
        stop_loss_price: parse_decimal(data, "stopLossPrice"),
        trailing_delta: parse_decimal(data, "trailingDelta"),
        trailing_percent: super::parse_decimal_multi(data, &["trailingPercent", "callbackRate"]),
        activation_price: super::parse_decimal_multi(data, &["activationPrice", "activatePrice"]),
        callback_rate: parse_decimal(data, "callbackRate"),
        working_type: data["workingType"].as_str().map(ToString::to_string),
        fees: Some(Vec::new()),
        info: value_to_hashmap(data),
    })
}

/// Parse ACK response from Binance create_order endpoint.
///
/// Binance API supports three response modes: ACK, RESULT, FULL.
/// ACK mode returns minimal data for fastest response:
/// ```json
/// {
///     "symbol": "BTCUSDT",
///     "orderId": 28,
///     "orderListId": -1,
///     "clientOrderId": "6gCrw2kRUAF9CvJDGP16IP",
///     "transactTime": 1507725176595
/// }
/// ```
///
/// This method enriches the ACK response with request context data
/// and delegates to [`parse_order`] for unified parsing.
///
/// # Arguments
///
/// * `ack_data` - The raw ACK response from Binance API
/// * `request` - The original order request containing side, type, price, amount
/// * `market` - Market metadata for symbol resolution
///
/// # Returns
///
/// Returns a complete [`Order`] structure with enriched data.
pub fn parse_ack_order(
    ack_data: &Value,
    request: &ccxt_core::types::OrderRequest,
    market: &Market,
) -> Result<Order> {
    use ccxt_core::types::{AmountSpec, OrderSide, OrderType};
    use serde_json::json;

    // Enrich ACK response with request context
    let mut enriched_data = ack_data.clone();
    if let Some(obj) = enriched_data.as_object_mut() {
        // ACK response doesn't include status, default to NEW
        if !obj.contains_key("status") {
            obj.insert("status".to_string(), json!("NEW"));
        }

        // Add side from request
        if !obj.contains_key("side") {
            let side_str = match request.side {
                OrderSide::Buy => "BUY",
                OrderSide::Sell => "SELL",
            };
            obj.insert("side".to_string(), json!(side_str));
        }

        // Add order type from request
        if !obj.contains_key("type") {
            let type_str = match request.order_type {
                OrderType::Market => "MARKET",
                OrderType::Limit => "LIMIT",
                OrderType::LimitMaker => "LIMIT_MAKER",
                OrderType::StopLoss => "STOP_LOSS",
                OrderType::StopLossLimit => "STOP_LOSS_LIMIT",
                OrderType::TakeProfit => "TAKE_PROFIT",
                OrderType::TakeProfitLimit => "TAKE_PROFIT_LIMIT",
                OrderType::StopMarket => "STOP_MARKET",
                OrderType::StopLimit => "STOP_LIMIT",
                OrderType::TrailingStop => "TRAILING_STOP_MARKET",
            };
            obj.insert("type".to_string(), json!(type_str));
        }

        // Add quantity from request
        if !obj.contains_key("origQty") {
            let qty_str = match &request.amount {
                AmountSpec::Base(amt) => amt.to_string(),
                AmountSpec::Quote(amt) => amt.to_string(),
            };
            obj.insert("origQty".to_string(), json!(qty_str));
        }

        // Add price from request (for limit orders)
        if !obj.contains_key("price") {
            if let Some(price) = &request.price {
                obj.insert("price".to_string(), json!(price.to_string()));
            }
        }

        // Add timeInForce from request
        if !obj.contains_key("timeInForce") {
            if let Some(tif) = &request.time_in_force {
                let tif_str = tif.to_string();
                // Convert unified TimeInForce to Binance format
                let binance_tif = match tif_str.as_str() {
                    "GTC" => "GTC",
                    "IOC" => "IOC",
                    "FOK" => "FOK",
                    "PO" => "GTX", // Post-Only uses GTX in Binance
                    _ => "GTC",
                };
                obj.insert("timeInForce".to_string(), json!(binance_tif));
            }
        }
    }

    // Delegate to standard parse_order
    parse_order(&enriched_data, Some(market))
}

/// Parse OCO (One-Cancels-the-Other) order data from Binance.
pub fn parse_oco_order(data: &Value) -> Result<OcoOrder> {
    let order_list_id = data["orderListId"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderListId")))?;

    let list_client_order_id = data["listClientOrderId"].as_str().map(ToString::to_string);

    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let list_status = data["listStatusType"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("listStatusType")))?
        .to_string();

    let list_order_status = data["listOrderStatus"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("listOrderStatus")))?
        .to_string();

    let transaction_time = data["transactionTime"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("transactionTime")))?;

    let datetime = chrono::DateTime::from_timestamp(transaction_time / 1000, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    let mut orders = Vec::new();
    if let Some(orders_array) = data["orders"].as_array() {
        for order in orders_array {
            let order_info = OcoOrderInfo {
                symbol: order["symbol"].as_str().unwrap_or(&symbol).to_string(),
                order_id: order["orderId"]
                    .as_i64()
                    .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?,
                client_order_id: order["clientOrderId"].as_str().map(ToString::to_string),
            };
            orders.push(order_info);
        }
    }

    let order_reports = if let Some(reports_array) = data["orderReports"].as_array() {
        let mut reports = Vec::new();
        for report in reports_array {
            let order_report = OrderReport {
                symbol: report["symbol"].as_str().unwrap_or(&symbol).to_string(),
                order_id: report["orderId"]
                    .as_i64()
                    .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?,
                order_list_id: report["orderListId"].as_i64().unwrap_or(order_list_id),
                client_order_id: report["clientOrderId"].as_str().map(ToString::to_string),
                transact_time: report["transactTime"].as_i64().unwrap_or(transaction_time),
                price: report["price"].as_str().unwrap_or("0").to_string(),
                orig_qty: report["origQty"].as_str().unwrap_or("0").to_string(),
                executed_qty: report["executedQty"].as_str().unwrap_or("0").to_string(),
                cummulative_quote_qty: report["cummulativeQuoteQty"]
                    .as_str()
                    .unwrap_or("0")
                    .to_string(),
                status: report["status"].as_str().unwrap_or("NEW").to_string(),
                time_in_force: report["timeInForce"].as_str().unwrap_or("GTC").to_string(),
                type_: report["type"].as_str().unwrap_or("LIMIT").to_string(),
                side: report["side"].as_str().unwrap_or("SELL").to_string(),
                stop_price: report["stopPrice"].as_str().map(ToString::to_string),
            };
            reports.push(order_report);
        }
        Some(reports)
    } else {
        None
    };

    Ok(OcoOrder {
        info: Some(data.clone()),
        order_list_id,
        list_client_order_id,
        symbol,
        list_status,
        list_order_status,
        transaction_time,
        datetime,
        orders,
        order_reports,
    })
}

/// Parse edit order response from Binance cancelReplace endpoint.
pub fn parse_edit_order_result(data: &Value, market: Option<&Market>) -> Result<Order> {
    let new_order_data = data.get("newOrderResponse").ok_or_else(|| {
        Error::from(ParseError::invalid_format(
            "data",
            "Missing newOrderResponse field",
        ))
    })?;

    parse_order(new_order_data, market)
}

/// Parse multiple orders from Binance API response.
pub fn parse_orders(data: &Value, market: Option<&Market>) -> Result<Vec<Order>> {
    if let Some(array) = data.as_array() {
        array.iter().map(|item| parse_order(item, market)).collect()
    } else {
        Ok(vec![parse_order(data, market)?])
    }
}

// ============================================================================
// USDT-Margined Futures (U本位合约) Order Parser
// ============================================================================

/// Parse order data from Binance USDT-Margined Futures order response.
///
/// Handles futures-specific fields:
/// - positionSide: LONG/SHORT/BOTH
/// - reduceOnly: true/false
/// - workingType: CONTRACT_PRICE/MARK_PRICE
/// - priceProtect: conditional order protection
/// - closePosition: conditional close all position
/// - updateTime: order update timestamp (futures uses this instead of 'time')
/// - cumQuote: cumulative quote volume (futures-specific)
pub fn parse_futures_order(data: &Value, market: Option<&Market>) -> Result<Order> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        Symbol::new_unchecked(
            data["symbol"]
                .as_str()
                .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?,
        )
    };

    let id = data["orderId"]
        .as_u64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?
        .to_string();

    // Futures uses 'updateTime' instead of 'time'
    let timestamp = data["time"]
        .as_i64()
        .or_else(|| data["transactTime"].as_i64())
        .or_else(|| data["updateTime"].as_i64());

    // Status parsing
    let status = if let Some(status_str) = data["status"].as_str() {
        match status_str {
            "FILLED" => OrderStatus::Closed,
            "CANCELED" => OrderStatus::Cancelled,
            "EXPIRED" => OrderStatus::Expired,
            "REJECTED" => OrderStatus::Rejected,
            _ => OrderStatus::Open,
        }
    } else {
        OrderStatus::Pending
    };

    // Side parsing
    let side = if let Some(side_str) = data["side"].as_str() {
        match side_str {
            "BUY" => OrderSide::Buy,
            "SELL" => OrderSide::Sell,
            _ => return Err(Error::from(ParseError::invalid_format("data", "side"))),
        }
    } else {
        tracing::warn!("Binance Futures API response missing 'side' field");
        OrderSide::Buy
    };

    // Order type parsing (futures has different types)
    let order_type = match data["type"].as_str() {
        Some("MARKET") => OrderType::Market,
        Some("LIMIT") => OrderType::Limit,
        // Futures uses STOP instead of STOP_LOSS
        Some("STOP") => OrderType::StopLoss,
        Some("TAKE_PROFIT") => OrderType::TakeProfit,
        Some("STOP_MARKET") => OrderType::StopMarket,
        Some("TAKE_PROFIT_MARKET") => OrderType::TakeProfit,
        Some("TRAILING_STOP_MARKET") => OrderType::TrailingStop,
        _ => OrderType::Limit,
    };

    let time_in_force = match data["timeInForce"].as_str() {
        Some("GTC") => Some(TimeInForce::GTC),
        Some("IOC") => Some(TimeInForce::IOC),
        Some("FOK") => Some(TimeInForce::FOK),
        Some("GTX") => Some(TimeInForce::PO),
        Some("GTD") => Some(TimeInForce::GTD),
        _ => None,
    };

    let price = parse_decimal(data, "price");
    let amount = parse_decimal(data, "origQty");
    let filled = parse_decimal(data, "executedQty");
    let remaining = match (&amount, &filled) {
        (Some(a), Some(f)) => Some(*a - *f),
        _ => None,
    };

    // Futures uses 'cumQuote' for cumulative quote volume
    let cost =
        parse_decimal(data, "cumQuote").or_else(|| parse_decimal(data, "cummulativeQuoteQty"));

    let average = match (&cost, &filled) {
        (Some(c), Some(f)) if !f.is_zero() => Some(*c / *f),
        _ => None,
    };

    let stop_price = parse_decimal(data, "stopPrice");

    // Extract reduceOnly from response
    let reduce_only = data["reduceOnly"].as_bool();

    Ok(Order {
        id,
        symbol,
        client_order_id: data["clientOrderId"].as_str().map(|s| s.to_string()),
        timestamp,
        datetime: timestamp.and_then(ccxt_core::parser_utils::timestamp_to_datetime),
        last_trade_timestamp: data["updateTime"].as_i64(),
        status,
        order_type,
        time_in_force: time_in_force.map(|tif| tif.to_string()),
        side,
        price,
        average,
        amount: amount.unwrap_or(Decimal::ZERO),
        filled,
        remaining,
        cost,
        stop_price,
        trigger_price: None,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: data["workingType"].as_str().map(|s| s.to_string()),
        post_only: None,
        reduce_only,
        fee: None,
        fees: None,
        trades: None,
        info: value_to_hashmap(data),
    })
}

/// Parse ACK response from Binance futures create_order endpoint.
///
/// Binance FAPI supports ACK response mode which returns minimal data:
/// ```json
/// {
///     "clientOrderId": "testOrder",
///     "orderId": 28,
///     "symbol": "BTCUSDT"
/// }
/// ```
///
/// This method enriches the ACK response with request context data
/// and delegates to [`parse_futures_order`] for unified parsing.
pub fn parse_futures_ack_order(
    ack_data: &Value,
    request: &ccxt_core::types::OrderRequest,
    market: &Market,
) -> Result<Order> {
    use ccxt_core::types::{AmountSpec, OrderSide, OrderType};
    use serde_json::json;

    // Enrich ACK response with request context
    let mut enriched_data = ack_data.clone();
    if let Some(obj) = enriched_data.as_object_mut() {
        // ACK response doesn't include status, default to NEW
        if !obj.contains_key("status") {
            obj.insert("status".to_string(), json!("NEW"));
        }

        // Add side from request
        if !obj.contains_key("side") {
            let side_str = match request.side {
                OrderSide::Buy => "BUY",
                OrderSide::Sell => "SELL",
            };
            obj.insert("side".to_string(), json!(side_str));
        }

        // Add order type from request (map to futures format)
        if !obj.contains_key("type") {
            let type_str = match request.order_type {
                OrderType::Market => "MARKET",
                OrderType::Limit => "LIMIT",
                OrderType::StopLoss | OrderType::StopLimit => "STOP",
                OrderType::TakeProfit | OrderType::TakeProfitLimit => "TAKE_PROFIT",
                OrderType::StopMarket => "STOP_MARKET",
                OrderType::TrailingStop => "TRAILING_STOP_MARKET",
                _ => "LIMIT",
            };
            obj.insert("type".to_string(), json!(type_str));
        }

        // Add quantity from request
        if !obj.contains_key("origQty") {
            let qty_str = match &request.amount {
                AmountSpec::Base(amt) => amt.to_string(),
                AmountSpec::Quote(amt) => amt.to_string(),
            };
            obj.insert("origQty".to_string(), json!(qty_str));
        }

        // Add price from request (for limit orders)
        if !obj.contains_key("price") {
            if let Some(price) = &request.price {
                obj.insert("price".to_string(), json!(price.to_string()));
            }
        }

        // Add timeInForce from request
        if !obj.contains_key("timeInForce") {
            if let Some(tif) = &request.time_in_force {
                let tif_str = tif.to_string();
                let binance_tif = match tif_str.as_str() {
                    "GTC" => "GTC",
                    "IOC" => "IOC",
                    "FOK" => "FOK",
                    "PO" => "GTX",
                    "GTD" => "GTD",
                    _ => "GTC",
                };
                obj.insert("timeInForce".to_string(), json!(binance_tif));
            }
        }

        // Add positionSide from request (futures-specific)
        if !obj.contains_key("positionSide") {
            if let Some(extra) = &request.extra {
                if let Some(position_side) = extra.get("positionSide") {
                    obj.insert(
                        "positionSide".to_string(),
                        json!(position_side.as_str().unwrap_or("BOTH")),
                    );
                } else {
                    obj.insert("positionSide".to_string(), json!("BOTH"));
                }
            } else {
                obj.insert("positionSide".to_string(), json!("BOTH"));
            }
        }
    }

    // Delegate to standard futures parser
    parse_futures_order(&enriched_data, Some(market))
}

/// Parse a futures algo order response from Binance FAPI.
///
/// Algo Order API 返回格式与普通订单不同:
/// ```json
/// {
///     "algoId": 123456,
///     "clientAlgoId": "testAlgoOrder",
///     "symbol": "BTCUSDT",
///     "side": "BUY",
///     "algoStatus": "NEW",
///     "algoType": "CONDITIONAL",
///     "triggerPrice": "50000.00",
///     "price": "49000.00",
///     "quantity": "0.001",
///     "positionSide": "LONG",
///     "time": 1640991234567
/// }
/// ```
///
/// 该函数将 algo order 响应转换为标准的 Order 结构。
pub fn parse_futures_algo_order(
    data: &Value,
    request: &ccxt_core::types::OrderRequest,
    market: &Market,
) -> Result<Order> {
    use rust_decimal::prelude::FromStr;

    // 从响应中提取字段
    let algo_id = data
        .get("algoId")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .to_string();

    let client_algo_id = data
        .get("clientAlgoId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let symbol = market.symbol.clone();

    let side = match data.get("side").and_then(|v| v.as_str()) {
        Some("BUY") => ccxt_core::types::OrderSide::Buy,
        Some("SELL") => ccxt_core::types::OrderSide::Sell,
        _ => request.side,
    };

    let order_type = request.order_type;

    let amount = data
        .get("quantity")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .unwrap_or(Decimal::ZERO);

    let price = parse_decimal(data, "price");

    let stop_price = parse_decimal(data, "triggerPrice");

    let status = match data.get("algoStatus").and_then(|v| v.as_str()) {
        Some("NEW") => ccxt_core::types::OrderStatus::Open,
        Some("TRIGGERED") => ccxt_core::types::OrderStatus::Open,
        Some("CANCELLED") => ccxt_core::types::OrderStatus::Cancelled,
        Some("REJECTED") => ccxt_core::types::OrderStatus::Rejected,
        Some("EXPIRED") => ccxt_core::types::OrderStatus::Expired,
        _ => ccxt_core::types::OrderStatus::Open,
    };

    let timestamp = data
        .get("time")
        .and_then(|v| v.as_i64())
        .or_else(|| data.get("triggerTime").and_then(|v| v.as_i64()));

    let mut info = value_to_hashmap(data);

    // 存储 algoId 和 clientAlgoId
    info.insert("algoId".to_string(), json!(algo_id));
    if let Some(client_id) = &client_algo_id {
        info.insert("clientAlgoId".to_string(), json!(client_id));
    }

    let order = Order {
        id: algo_id,
        client_order_id: client_algo_id.or(request.client_order_id.clone()),
        timestamp,
        datetime: timestamp.and_then(ccxt_core::parser_utils::timestamp_to_datetime),
        last_trade_timestamp: None,
        symbol,
        status,
        order_type,
        time_in_force: None,
        side,
        price,
        average: None,
        amount,
        filled: Some(Decimal::ZERO),
        remaining: None,
        cost: None,
        stop_price,
        trigger_price: stop_price,
        take_profit_price: None,
        stop_loss_price: None,
        trailing_delta: None,
        trailing_percent: None,
        activation_price: None,
        callback_rate: None,
        working_type: None,
        post_only: None,
        reduce_only: None,
        fee: None,
        fees: None,
        trades: None,
        info,
    };

    Ok(order)
}

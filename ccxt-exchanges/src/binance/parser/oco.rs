//! Binance OCO (One-Cancels-the-Other) order parser.
//!
//! Converts Binance API OCO order responses into standardized CCXT format.

use ccxt_core::{
    Error, ParseError, Result,
    types::{OcoOrder, OcoOrderInfo, OrderReport},
};
use serde_json::Value;

/// Parse an OCO order from Binance API response.
///
/// # Arguments
///
/// * `data` - JSON response from Binance OCO order API.
///
/// # Returns
///
/// Returns an [`OcoOrder`] structure.
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

    let list_order_status = data["listStatus"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("listStatus")))?
        .to_string();

    let transaction_time = data["transactionTime"]
        .as_i64()
        .or_else(|| data["transactTime"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let datetime = chrono::DateTime::from_timestamp_millis(transaction_time)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    // Parse orders array
    let orders = if let Some(orders_array) = data["orders"].as_array() {
        orders_array
            .iter()
            .filter_map(|order_data| parse_oco_order_info(order_data).ok())
            .collect()
    } else {
        Vec::new()
    };

    // Parse order reports (present when creating order)
    let order_reports = data["orderReports"].as_array().map(|reports_array| {
        reports_array
            .iter()
            .filter_map(|report_data| parse_order_report(report_data).ok())
            .collect()
    });

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

/// Parse multiple OCO orders from Binance API response.
///
/// # Arguments
///
/// * `data` - JSON array response from Binance OCO orders API.
///
/// # Returns
///
/// Returns a vector of [`OcoOrder`] structures.
pub fn parse_oco_orders(data: &Value) -> Result<Vec<OcoOrder>> {
    let orders_array = data.as_array().ok_or_else(|| {
        Error::from(ParseError::invalid_format(
            "data",
            "expected array of OCO orders",
        ))
    })?;

    let mut orders = Vec::new();
    for order_data in orders_array {
        match parse_oco_order(order_data) {
            Ok(order) => orders.push(order),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse OCO order");
            }
        }
    }

    Ok(orders)
}

/// Parse OCO order info from Binance API response.
///
/// # Arguments
///
/// * `data` - JSON object containing OCO order info.
///
/// # Returns
///
/// Returns an [`OcoOrderInfo`] structure.
fn parse_oco_order_info(data: &Value) -> Result<OcoOrderInfo> {
    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let order_id = data["orderId"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?;

    let client_order_id = data["clientOrderId"].as_str().map(ToString::to_string);

    Ok(OcoOrderInfo {
        symbol,
        order_id,
        client_order_id,
    })
}

/// Parse order report from Binance API response.
///
/// # Arguments
///
/// * `data` - JSON object containing order report.
///
/// # Returns
///
/// Returns an [`OrderReport`] structure.
fn parse_order_report(data: &Value) -> Result<OrderReport> {
    let symbol = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    let order_id = data["orderId"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?;

    let order_list_id = data["orderListId"]
        .as_i64()
        .ok_or_else(|| Error::from(ParseError::missing_field("orderListId")))?;

    let client_order_id = data["clientOrderId"].as_str().map(ToString::to_string);

    let transact_time = data["transactTime"]
        .as_i64()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let price = data["price"]
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_default();

    let orig_qty = data["origQty"]
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_default();

    let executed_qty = data["executedQty"]
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_default();

    let cummulative_quote_qty = data["cummulativeQuoteQty"]
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_default();

    let status = data["status"]
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_default();

    let time_in_force = data["timeInForce"]
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_default();

    let type_ = data["type"]
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_default();

    let side = data["side"]
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_default();

    let stop_price = data["stopPrice"].as_str().map(ToString::to_string);

    Ok(OrderReport {
        symbol,
        order_id,
        order_list_id,
        client_order_id,
        transact_time,
        price,
        orig_qty,
        executed_qty,
        cummulative_quote_qty,
        status,
        time_in_force,
        type_,
        side,
        stop_price,
    })
}

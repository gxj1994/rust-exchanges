//! Binance Spot Order Builder
//!
//! This module contains pure functions for building Binance spot order request payloads.
//! These functions are designed to be testable in isolation without external dependencies.
//!
//! # Design Principles
//!
//! - **Pure Functions**: No access to `self` or external state
//! - **Testable**: Can be unit tested without mocking HTTP calls
//! - **Zero Performance Overhead**: Eligible for LLVM inlining
//!
//! # API Reference
//!
//! Based on Binance Spot API documentation:
//! - POST /api/v3/order (Trade)
//! - https://developers.binance.com/docs/binance-spot-api-docs/rest-api/trading-endpoints#new-order-trade

use ccxt_core::{
    Error, Result,
    types::{
        AmountSpec, Market, MarketType, OrderRequest, OrderSide, OrderType, TimeInForce,
        order::order_request::TrailingAmountType,
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Value};

// ============================================================================
// Public API: Payload Builders
// ============================================================================

/// Build create order payload for Binance Spot API.
///
/// # Arguments
///
/// * `request` - Unified order request from the builder pattern
/// * `market` - Market metadata for symbol conversion
///
/// # Returns
///
/// Returns a JSON Value ready to be sent to Binance Spot API.
///
/// # Binance Spot API Requirements
///
/// ## Required Parameters by Order Type
///
/// | Type | Required Parameters |
/// |------|-------------------|
/// | LIMIT | timeInForce, quantity, price |
/// | MARKET | quantity OR quoteOrderQty |
/// | STOP_LOSS | quantity, stopPrice |
/// | STOP_LOSS_LIMIT | timeInForce, quantity, price, stopPrice |
/// | TAKE_PROFIT | quantity, stopPrice |
/// | TAKE_PROFIT_LIMIT | timeInForce, quantity, price, stopPrice |
/// | LIMIT_MAKER | quantity, price |
///
/// ## Special Features
///
/// - **Iceberg Orders**: Use `icebergQty` parameter (must use GTC timeInForce)
/// - **Pegged Orders**: Use `pegPriceType`, `pegOffsetValue`, `pegOffsetType`
/// - **Self Trade Prevention**: Use `selfTradePreventionMode`
/// - **Strategy Orders**: Use `strategyId`, `strategyType` (>= 1000000)
///
/// # Important Notes
///
/// 1. **Spot Market BUY orders**: Can use `quoteOrderQty` to specify quote asset amount (USDT)
/// 2. **Spot Market SELL orders**: Use `quantity` for base asset amount (BTC)
/// 3. **Stop Price Rules**:
///    - STOP_LOSS BUY / TAKE_PROFIT SELL: stopPrice > current market price
///    - STOP_LOSS SELL / TAKE_PROFIT BUY: stopPrice < current market price
/// 4. **Iceberg Orders**: timeInForce MUST be GTC
/// 5. **Trailing Delta**: Expressed in basis points (1% = 100 basis points)
///
/// # Example
///
/// ```ignore
/// let request = OrderRequest::builder()
///     .symbol("BTC/USDT")
///     .side(OrderSide::Buy)
///     .order_type(OrderType::Limit)
///     .amount(Amount::new(dec!(0.001)))
///     .price(Price::new(dec!(50000)))
///     .time_in_force(TimeInForce::GTC)
///     .build()?;
///
/// let market = exchange.market("BTC/USDT").await?;
/// let payload = build_create_order_payload(&request, &market)?;
/// ```
pub fn build_create_order_payload(request: &OrderRequest, market: &Market) -> Result<Value> {
    // Validate order type support for spot
    validate_spot_order_type(request.order_type)?;

    let mut map = Map::new();

    // Basic required fields
    map.insert(
        "symbol".to_string(),
        serde_json::Value::String(market.id.clone()),
    );
    map.insert(
        "side".to_string(),
        serde_json::Value::String(map_side(request.side)),
    );
    let order_type_str = map_order_type(request.order_type)?;
    map.insert(
        "type".to_string(),
        serde_json::Value::String(order_type_str),
    );

    // Quantity handling (spot-specific logic)
    let quantity_params = build_quantity_params(request, &market.market_type)?;
    for (key, value) in quantity_params {
        map.insert(key, Value::String(value));
    }

    // Price (required for limit orders)
    if let Some(price) = request.price {
        if is_price_required(request.order_type) {
            map.insert("price".to_string(), Value::String(price.to_string()));
        }
    }

    // Time in force
    if let Some(tif) = request.time_in_force {
        map.insert(
            "timeInForce".to_string(),
            Value::String(map_time_in_force(tif)),
        );
    } else if is_time_in_force_required(request.order_type) {
        // Default to GTC for limit orders
        map.insert("timeInForce".to_string(), Value::String("GTC".to_string()));
    }

    // Stop price (for conditional orders)
    if let Some(stop_price) = request.stop_price {
        map.insert(
            "stopPrice".to_string(),
            Value::String(stop_price.to_string()),
        );
    }

    // Client order ID
    if let Some(client_id) = &request.client_order_id {
        map.insert(
            "newClientOrderId".to_string(),
            Value::String(client_id.clone()),
        );
    }

    // Iceberg quantity (spot feature) - passed via extra params
    if let Some(extra) = &request.extra {
        if let Some(iceberg_qty) = extra.get("icebergQty") {
            // Iceberg orders must use GTC
            if let Some(tif) = request.time_in_force {
                if tif != TimeInForce::GTC {
                    return Err(Error::InvalidOrder(
                        "Iceberg orders require timeInForce=GTC".into(),
                    ));
                }
            }
            map.insert(
                "icebergQty".to_string(),
                Value::String(iceberg_qty.as_str().unwrap_or("0").to_string()),
            );
        }
    }

    // Trailing delta (convert to basis points for spot)
    if request.order_type == OrderType::TrailingStop {
        if let Some(trailing_rate) = request.trailing_callback_rate {
            match request.trailing_amount_type {
                Some(TrailingAmountType::BasisPoints) => {
                    // Already in basis points
                    map.insert(
                        "trailingDelta".to_string(),
                        Value::String(trailing_rate.to_string()),
                    );
                }
                Some(TrailingAmountType::Percent) | None => {
                    // Convert percentage to basis points (e.g., 2.0% -> 200)
                    let basis_points = (trailing_rate * Decimal::from(100)).round();
                    let delta_int = basis_points.to_i64().ok_or_else(|| {
                        Error::InvalidOrder(
                            format!(
                                "Cannot convert trailing percent {} to basis points integer",
                                trailing_rate
                            )
                            .into(),
                        )
                    })?;
                    map.insert(
                        "trailingDelta".to_string(),
                        Value::String(delta_int.to_string()),
                    );
                }
            }
        }
    }

    // Pegged order parameters (spot feature) - passed via extra params
    if let Some(extra) = &request.extra {
        if let Some(peg_price_type) = extra.get("pegPriceType") {
            map.insert(
                "pegPriceType".to_string(),
                Value::String(peg_price_type.as_str().unwrap_or("").to_string()),
            );
        }
        if let Some(peg_offset_value) = extra.get("pegOffsetValue") {
            map.insert(
                "pegOffsetValue".to_string(),
                Value::String(peg_offset_value.as_str().unwrap_or("0").to_string()),
            );
        }
        if let Some(peg_offset_type) = extra.get("pegOffsetType") {
            map.insert(
                "pegOffsetType".to_string(),
                Value::String(peg_offset_type.as_str().unwrap_or("").to_string()),
            );
        }
    }

    // Self trade prevention mode - passed via extra params
    if let Some(extra) = &request.extra {
        if let Some(stp_mode) = extra.get("selfTradePreventionMode") {
            map.insert(
                "selfTradePreventionMode".to_string(),
                Value::String(stp_mode.as_str().unwrap_or("").to_string()),
            );
        }
    }

    // Strategy parameters - passed via extra params
    if let Some(extra) = &request.extra {
        if let Some(strategy_id) = extra.get("strategyId") {
            map.insert(
                "strategyId".to_string(),
                Value::String(strategy_id.as_str().unwrap_or("0").to_string()),
            );
        }
        if let Some(strategy_type) = extra.get("strategyType") {
            let stype = strategy_type
                .as_str()
                .unwrap_or("0")
                .parse::<i64>()
                .unwrap_or(0);
            if stype < 1000000 {
                return Err(Error::InvalidOrder(
                    "strategyType must be >= 1000000".into(),
                ));
            }
            map.insert("strategyType".to_string(), Value::String(stype.to_string()));
        }
    }

    // Response type - passed via extra params
    if let Some(extra) = &request.extra {
        if let Some(resp_type) = extra.get("newOrderRespType") {
            map.insert(
                "newOrderRespType".to_string(),
                Value::String(resp_type.as_str().unwrap_or("").to_string()),
            );
        }
    }

    // Note: Binance Spot does NOT support embedded TP/SL in create_order
    // TP/SL must be placed as separate orders
    // (No fields to check - TP/SL would use stop_price with appropriate order_type)
    if request.stop_price.is_some()
        && matches!(
            request.order_type,
            OrderType::TakeProfit
                | OrderType::TakeProfitLimit
                | OrderType::StopLoss
                | OrderType::StopLossLimit
        )
    {
        // This is expected for conditional orders, no warning needed
    }

    Ok(Value::Object(map))
}

/// Build cancel order payload for Binance Spot API.
///
/// # Arguments
///
/// * `order_id` - Order ID to cancel
/// * `market` - Market metadata
/// * `client_order_id` - Optional client order ID (alternative to order_id)
///
/// # Notes
///
/// Binance requires either `orderId` or `origClientOrderId`.
pub fn build_cancel_order_payload(
    order_id: Option<&str>,
    market: &Market,
    client_order_id: Option<&str>,
) -> Result<Value> {
    if order_id.is_none() && client_order_id.is_none() {
        return Err(Error::invalid_request(
            "Either order_id or client_order_id must be provided".to_string(),
        ));
    }

    let mut map = Map::new();
    map.insert("symbol".to_string(), Value::String(market.id.clone()));

    if let Some(oid) = order_id {
        map.insert("orderId".to_string(), Value::String(oid.to_string()));
    }
    if let Some(client_oid) = client_order_id {
        map.insert(
            "origClientOrderId".to_string(),
            Value::String(client_oid.to_string()),
        );
    }

    Ok(Value::Object(map))
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

/// Validate that the order type is supported by Binance Spot.
fn validate_spot_order_type(order_type: OrderType) -> Result<()> {
    match order_type {
        OrderType::Market
        | OrderType::Limit
        | OrderType::StopLoss
        | OrderType::StopLossLimit
        | OrderType::TakeProfit
        | OrderType::TakeProfitLimit
        | OrderType::LimitMaker => Ok(()),
        OrderType::StopMarket | OrderType::StopLimit | OrderType::TrailingStop => {
            Err(Error::InvalidOrder(
                format!(
                    "Binance Spot does not support {:?} orders. \
             Use STOP_LOSS, STOP_LOSS_LIMIT, TAKE_PROFIT, or TAKE_PROFIT_LIMIT instead.",
                    order_type
                )
                .into(),
            ))
        }
    }
}

/// Build quantity parameters based on market type and order specifications.
///
/// # Binance Spot Rules
///
/// - **Market BUY**: Can use `quoteOrderQty` (USDT amount) OR `quantity` (BTC amount)
/// - **Market SELL**: Use `quantity` (BTC amount)
/// - **Limit orders**: Use `quantity` (BTC amount)
fn build_quantity_params(
    request: &OrderRequest,
    market_type: &MarketType,
) -> Result<Vec<(String, String)>> {
    if *market_type != MarketType::Spot {
        return Err(Error::invalid_request(
            "This builder is for spot markets only".to_string(),
        ));
    }

    match request.order_type {
        OrderType::Market => {
            // Market orders: check if quote currency amount is specified
            if let AmountSpec::Quote(quote_amt) = &request.amount {
                // Use quoteOrderQty for market buy with quote amount
                Ok(vec![("quoteOrderQty".to_string(), quote_amt.to_string())])
            } else {
                // Use quantity for base currency amount
                Ok(vec![("quantity".to_string(), request.amount.to_string())])
            }
        }
        _ => {
            // Limit and conditional orders: always use quantity (base currency)
            Ok(vec![("quantity".to_string(), request.amount.to_string())])
        }
    }
}

/// Check if price is required for the given order type.
fn is_price_required(order_type: OrderType) -> bool {
    matches!(
        order_type,
        OrderType::Limit
            | OrderType::LimitMaker
            | OrderType::StopLossLimit
            | OrderType::TakeProfitLimit
    )
}

/// Check if timeInForce is required for the given order type.
fn is_time_in_force_required(order_type: OrderType) -> bool {
    matches!(
        order_type,
        OrderType::Limit | OrderType::StopLossLimit | OrderType::TakeProfitLimit
    )
}

// ============================================================================
// Mapping Functions (Internal)
// ============================================================================

/// Map unified OrderSide to Binance side string.
fn map_side(side: OrderSide) -> String {
    match side {
        OrderSide::Buy => "BUY".to_string(),
        OrderSide::Sell => "SELL".to_string(),
    }
}

/// Map unified OrderType to Binance order type string.
fn map_order_type(order_type: OrderType) -> Result<String> {
    match order_type {
        OrderType::Market => Ok("MARKET".to_string()),
        OrderType::Limit => Ok("LIMIT".to_string()),
        OrderType::StopLoss => Ok("STOP_LOSS".to_string()),
        OrderType::StopLossLimit => Ok("STOP_LOSS_LIMIT".to_string()),
        OrderType::TakeProfit => Ok("TAKE_PROFIT".to_string()),
        OrderType::TakeProfitLimit => Ok("TAKE_PROFIT_LIMIT".to_string()),
        OrderType::LimitMaker => Ok("LIMIT_MAKER".to_string()),
        other => Err(Error::InvalidOrder(
            format!("Unsupported order type for Binance Spot: {:?}", other).into(),
        )),
    }
}

/// Map unified TimeInForce to Binance timeInForce string.
fn map_time_in_force(tif: TimeInForce) -> String {
    match tif {
        TimeInForce::GTC => "GTC".to_string(),
        TimeInForce::IOC => "IOC".to_string(),
        TimeInForce::FOK => "FOK".to_string(),
        TimeInForce::PO => "GTX".to_string(), // Post-only maps to GTX on Binance
        TimeInForce::GTD => "GTD".to_string(), // Good Till Date
    }
}

// ============================================================================
// USDT-Margined Futures (U本位合约) Order Builder
// ============================================================================

/// Build create order payload for Binance USDT-Margined Futures (U本位合约).
///
/// # Arguments
///
/// * `request` - Unified order request from the builder pattern
/// * `market` - Market metadata for symbol conversion and contract size
///
/// # Returns
///
/// Returns a JSON Value ready to be sent to Binance FAPI.
///
/// # Binance FAPI Requirements
///
/// ## Required Parameters by Order Type
///
/// | Type | Required Parameters |
/// |------|-------------------|
/// | LIMIT | timeInForce, quantity, price |
/// | MARKET | quantity |
/// | STOP/TAKE_PROFIT | quantity, stopPrice, price (for limit), timeInForce |
/// | STOP_MARKET/TAKE_PROFIT_MARKET | quantity, stopPrice |
/// | TRAILING_STOP_MARKET | quantity, callbackRate |
///
/// # Futures-Specific Features
///
/// - **positionSide**: LONG/SHORT/BOTH (双向持仓必填)
/// - **reduceOnly**: true/false (仅减仓)
/// - **priceMatch**: OPPONENT/QUEUE 等盘口价格模式 (不能与price同时使用)
/// - **workingType**: CONTRACT_PRICE/MARK_PRICE (条件单触发类型)
/// - **priceProtect**: 条件单触发保护
/// - **goodTillDate**: GTD订单自动取消时间 (timeInForce=GTD时必填)
///
/// # Example
///
/// ```ignore
/// let request = OrderRequest::builder()
///     .symbol("BTC/USDT:USDT")
///     .side(OrderSide::Buy)
///     .order_type(OrderType::Limit)
///     .amount(Amount::new(dec!(0.001)))
///     .price(Price::new(dec!(50000)))
///     .build()?;
///
/// let market = exchange.market("BTC/USDT:USDT").await?;
/// let payload = build_futures_create_order_payload(&request, &market)?;
/// ```
pub fn build_futures_create_order_payload(
    request: &OrderRequest,
    market: &Market,
) -> Result<Value> {
    // 检查是否为条件单 (STOP, TAKE_PROFIT, STOP_MARKET, TAKE_PROFIT_MARKET)
    // 这些订单类型需要使用 Algo Order API (/fapi/v1/algoOrder)
    if is_futures_conditional_order(request.order_type) {
        return Err(Error::invalid_request(format!(
            "Order type {:?} is a conditional order and requires the Algo Order API (/fapi/v1/algoOrder). \
             Please use create_algo_order() method instead.",
            request.order_type
        )));
    }

    // Validate futures order type support
    validate_futures_order_type(request.order_type)?;

    let mut map = Map::new();

    // Basic required fields
    map.insert(
        "symbol".to_string(),
        serde_json::Value::String(market.id.clone()),
    );
    map.insert(
        "side".to_string(),
        serde_json::Value::String(map_side(request.side)),
    );
    let order_type_str = map_futures_order_type(request.order_type)?;
    map.insert(
        "type".to_string(),
        serde_json::Value::String(order_type_str),
    );

    // positionSide (futures-specific)
    // Default to BOTH for one-way position mode
    if let Some(extra) = &request.extra {
        if let Some(position_side) = extra.get("positionSide") {
            map.insert(
                "positionSide".to_string(),
                Value::String(position_side.as_str().unwrap_or("BOTH").to_string()),
            );
        } else {
            map.insert(
                "positionSide".to_string(),
                Value::String("BOTH".to_string()),
            );
        }
    } else {
        map.insert(
            "positionSide".to_string(),
            Value::String("BOTH".to_string()),
        );
    }

    // reduceOnly (futures-specific)
    if let Some(extra) = &request.extra {
        if let Some(reduce_only) = extra.get("reduceOnly") {
            map.insert(
                "reduceOnly".to_string(),
                Value::String(reduce_only.as_str().unwrap_or("false").to_string()),
            );
        }
    }

    // Quantity handling (futures: always use quantity, no quoteOrderQty support)
    let quantity = request.amount.to_string();
    map.insert("quantity".to_string(), Value::String(quantity));

    // Price (required for limit orders)
    if let Some(price) = request.price {
        if is_price_required_for_futures(request.order_type) {
            map.insert("price".to_string(), Value::String(price.to_string()));
        }
    }

    // Time in force
    if let Some(tif) = request.time_in_force {
        map.insert(
            "timeInForce".to_string(),
            Value::String(map_time_in_force(tif)),
        );

        // goodTillDate (required when timeInForce=GTD)
        if tif == TimeInForce::GTD {
            if let Some(extra) = &request.extra {
                if let Some(good_till_date) = extra.get("goodTillDate") {
                    map.insert(
                        "goodTillDate".to_string(),
                        Value::String(good_till_date.as_str().unwrap_or("0").to_string()),
                    );
                }
            }
        }
    } else if is_time_in_force_required(request.order_type) {
        // Default to GTC for limit orders
        map.insert("timeInForce".to_string(), Value::String("GTC".to_string()));
    }

    // Stop price (for conditional orders)
    if let Some(stop_price) = request.stop_price {
        map.insert(
            "stopPrice".to_string(),
            Value::String(stop_price.to_string()),
        );
    }

    // Client order ID
    if let Some(client_id) = &request.client_order_id {
        map.insert(
            "newClientOrderId".to_string(),
            Value::String(client_id.clone()),
        );
    }

    // priceMatch (futures-specific: opponent/queue price matching)
    // Cannot be used with price simultaneously
    if let Some(extra) = &request.extra {
        if let Some(price_match) = extra.get("priceMatch") {
            if request.price.is_some() {
                return Err(Error::InvalidOrder(
                    "priceMatch and price cannot be used together".into(),
                ));
            }
            map.insert(
                "priceMatch".to_string(),
                Value::String(price_match.as_str().unwrap_or("").to_string()),
            );
        }
    }

    // workingType (futures-specific: condition price trigger type)
    if let Some(extra) = &request.extra {
        if let Some(working_type) = extra.get("workingType") {
            map.insert(
                "workingType".to_string(),
                Value::String(
                    working_type
                        .as_str()
                        .unwrap_or("CONTRACT_PRICE")
                        .to_string(),
                ),
            );
        }
    }

    // priceProtect (futures-specific: conditional order protection)
    if let Some(extra) = &request.extra {
        if let Some(price_protect) = extra.get("priceProtect") {
            map.insert(
                "priceProtect".to_string(),
                Value::String(price_protect.as_str().unwrap_or("FALSE").to_string()),
            );
        }
    }

    // Trailing delta (for TRAILING_STOP_MARKET)
    if request.order_type == OrderType::TrailingStop {
        if let Some(trailing_rate) = request.trailing_callback_rate {
            match request.trailing_amount_type {
                Some(TrailingAmountType::BasisPoints) => {
                    // Already in basis points
                    map.insert(
                        "callbackRate".to_string(),
                        Value::String(trailing_rate.to_string()),
                    );
                }
                Some(TrailingAmountType::Percent) | None => {
                    // Convert percentage to basis points (e.g., 2.0% -> 200)
                    let basis_points = (trailing_rate * Decimal::from(100)).round();
                    let delta_int = basis_points.to_i64().ok_or_else(|| {
                        Error::InvalidOrder(
                            format!(
                                "Cannot convert trailing percent {} to basis points integer",
                                trailing_rate
                            )
                            .into(),
                        )
                    })?;
                    map.insert(
                        "callbackRate".to_string(),
                        Value::String(delta_int.to_string()),
                    );
                }
            }
        }
    }

    // Self trade prevention mode
    if let Some(extra) = &request.extra {
        if let Some(stp_mode) = extra.get("selfTradePreventionMode") {
            map.insert(
                "selfTradePreventionMode".to_string(),
                Value::String(stp_mode.as_str().unwrap_or("").to_string()),
            );
        }
    }

    // Response type
    if let Some(extra) = &request.extra {
        if let Some(resp_type) = extra.get("newOrderRespType") {
            map.insert(
                "newOrderRespType".to_string(),
                Value::String(resp_type.as_str().unwrap_or("ACK").to_string()),
            );
        }
    }

    Ok(Value::Object(map))
}

/// Check if the order type is a conditional order that requires Algo Order API.
///
/// Conditional orders (STOP, TAKE_PROFIT, STOP_MARKET, TAKE_PROFIT_MARKET) must use
/// POST /fapi/v1/algoOrder instead of POST /fapi/v1/order.
fn is_futures_conditional_order(order_type: OrderType) -> bool {
    matches!(
        order_type,
        OrderType::StopLoss
            | OrderType::StopLossLimit
            | OrderType::TakeProfit
            | OrderType::TakeProfitLimit
            | OrderType::StopMarket
    )
}

/// Build create algo order payload for Binance Futures Algo Order API.
///
/// This is used for conditional orders (STOP, TAKE_PROFIT, etc.) which must use
/// POST /fapi/v1/algoOrder instead of POST /fapi/v1/order.
///
/// # API Reference
///
/// POST /fapi/v1/algoOrder
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/trade/rest-api/New-Algo-Order
///
/// # Required Parameters
///
/// | Parameter | Type | Required | Description |
/// |-----------|------|----------|-------------|
/// | symbol | STRING | YES | Trading pair |
/// | side | ENUM | YES | BUY/SELL |
/// | algoType | ENUM | YES | CONDITIONAL (for stop/take-profit) |
/// | positionSide | ENUM | NO | LONG/SHORT/BOTH |
/// | quantity | DECIMAL | YES | Order quantity |
/// | triggerPrice | DECIMAL | YES | Trigger price |
/// | price | DECIMAL | NO | Order price (for limit orders) |
/// | workingType | ENUM | NO | CONTRACT_PRICE/MARK_PRICE |
/// | priceProtect | BOOLEAN | NO | "true"/"false" |
/// | clientAlgoId | STRING | NO | Client algo order ID |
///
/// # Returns
///
/// Returns a JSON Value ready to be sent to Binance FAPI Algo Order endpoint.
pub fn build_futures_algo_order_payload(request: &OrderRequest, market: &Market) -> Result<Value> {
    let mut map = Map::new();

    // symbol - use market ID (e.g., "BTCUSDT")
    map.insert("symbol".to_string(), Value::String(market.id.clone()));

    // side - BUY/SELL
    let side_str = match request.side {
        OrderSide::Buy => "BUY",
        OrderSide::Sell => "SELL",
    };
    map.insert("side".to_string(), Value::String(side_str.to_string()));

    // type - 订单类型 (STOP, TAKE_PROFIT, STOP_MARKET, TAKE_PROFIT_MARKET)
    let order_type_str = map_futures_order_type(request.order_type)?;
    map.insert("type".to_string(), Value::String(order_type_str));

    // algoType - CONDITIONAL (for stop/take-profit orders)
    map.insert(
        "algoType".to_string(),
        Value::String("CONDITIONAL".to_string()),
    );

    // quantity - contract amount
    let quantity = request.amount.to_string();
    map.insert("quantity".to_string(), Value::String(quantity));

    // triggerPrice - 触发价格 (required for all conditional orders)
    let trigger_price = request.stop_price.ok_or_else(|| {
        Error::invalid_request("triggerPrice (stop_price) is required for algo orders".to_string())
    })?;
    map.insert(
        "triggerPrice".to_string(),
        Value::String(trigger_price.to_string()),
    );

    // price - 委托价格 (optional, for limit orders)
    if let Some(price) = request.price {
        map.insert("price".to_string(), Value::String(price.to_string()));
    }

    // positionSide - LONG/SHORT/BOTH
    if let Some(extra) = &request.extra {
        if let Some(position_side) = extra.get("positionSide") {
            let ps_str = position_side.as_str().unwrap_or("BOTH").to_string();
            map.insert("positionSide".to_string(), Value::String(ps_str));
        }
    }

    // workingType - CONTRACT_PRICE/MARK_PRICE (optional)
    if let Some(extra) = &request.extra {
        if let Some(working_type) = extra.get("workingType") {
            if let Some(wt) = working_type.as_str() {
                map.insert("workingType".to_string(), Value::String(wt.to_string()));
            }
        }
    }

    // priceProtect - 条件单触发保护 (optional)
    if let Some(extra) = &request.extra {
        if let Some(price_protect) = extra.get("priceProtect") {
            if let Some(pp) = price_protect.as_bool() {
                map.insert(
                    "priceProtect".to_string(),
                    Value::String(if pp { "true" } else { "false" }.to_string()),
                );
            }
        }
    }

    // clientAlgoId - 客户端自定义订单号 (optional)
    if let Some(client_algo_id) = request.client_order_id.as_deref() {
        map.insert(
            "clientAlgoId".to_string(),
            Value::String(client_algo_id.to_string()),
        );
    }

    Ok(Value::Object(map))
}

/// Validate that the order type is supported by Binance Futures.
fn validate_futures_order_type(order_type: OrderType) -> Result<()> {
    match order_type {
        OrderType::Market
        | OrderType::Limit
        | OrderType::StopLoss
        | OrderType::StopLossLimit
        | OrderType::TakeProfit
        | OrderType::TakeProfitLimit
        | OrderType::StopMarket
        | OrderType::TrailingStop => Ok(()),
        OrderType::LimitMaker => {
            Err(Error::InvalidOrder(
                "Binance Futures does not support LIMIT_MAKER orders. Use LIMIT with timeInForce=GTX instead.".into(),
            ))
        }
        _ => Err(Error::InvalidOrder(
            format!("Binance Futures does not support {:?} orders", order_type).into(),
        )),
    }
}

/// Map unified OrderType to Binance Futures order type string.
fn map_futures_order_type(order_type: OrderType) -> Result<String> {
    match order_type {
        OrderType::Market => Ok("MARKET".to_string()),
        OrderType::Limit => Ok("LIMIT".to_string()),
        // Futures uses STOP instead of STOP_LOSS
        OrderType::StopLoss | OrderType::StopLimit => Ok("STOP".to_string()),
        OrderType::StopLossLimit => Ok("STOP".to_string()),
        OrderType::TakeProfit => Ok("TAKE_PROFIT".to_string()),
        OrderType::TakeProfitLimit => Ok("TAKE_PROFIT".to_string()),
        OrderType::StopMarket => Ok("STOP_MARKET".to_string()),
        OrderType::TrailingStop => Ok("TRAILING_STOP_MARKET".to_string()),
        other => Err(Error::InvalidOrder(
            format!("Unsupported order type for Binance Futures: {:?}", other).into(),
        )),
    }
}

/// Check if price is required for the given futures order type.
fn is_price_required_for_futures(order_type: OrderType) -> bool {
    matches!(
        order_type,
        OrderType::Limit | OrderType::StopLossLimit | OrderType::TakeProfitLimit
    )
}

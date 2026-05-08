//! Gate.io order builder helper functions.
//!
//! These functions are responsible for converting unified OrderRequest
//! into Gate.io specific JSON format.
//!
//! # Design Principles
//!
//! - Pure functions: No dependency on exchange instance, only uses passed parameters
//! - Testable: Each function can be unit tested independently
//! - Clear naming: Function names clearly indicate their purpose
//! - Complete documentation: Explains Gate API specific requirements

use ccxt_core::Error;
use ccxt_core::types::market::Market;
use ccxt_core::types::{OrderRequest, OrderSide, OrderType, TimeInForce};
use serde_json::{Map, Value};

/// Convert unified symbol format (BTC/USDT) to Gate format (BTC_USDT).
#[inline]
fn to_exchange_symbol(symbol: &str) -> String {
    symbol.replace('/', "_")
}

// ============================================================================
// Create Order (Spot)
// ============================================================================

/// Build create order request body for SPOT trading
///
/// # Gate API v4 Requirements
///
/// - `currency_pair`: Trading pair (e.g., "BTC_USDT")
/// - `side`: Buy or sell ("buy" / "sell")
/// - `type`: Order type ("limit" / "market")
/// - `amount`: Order amount
/// - `price`: Required for limit orders
/// - `time_in_force`: Optional (gtc/ioc/fok/poc)
/// - `text`: Optional client order ID (prefix with "t-")
///
/// # Parameters
///
/// - `request`: Unified order request
/// - `market`: Market information
///
/// # Errors
///
/// - If order type is not supported, returns `Error::InvalidOrder`
/// - If limit order without price, returns `Error::InvalidOrder`
pub fn build_create_order_payload(request: &OrderRequest, market: &Market) -> Result<Value, Error> {
    // Validate 1: Unsupported order types
    if !matches!(request.order_type, OrderType::Market | OrderType::Limit) {
        return Err(Error::InvalidOrder(
            format!("Gate does not support {:?} orders", request.order_type).into(),
        ));
    }

    // Validate 2: Limit orders require price
    if matches!(request.order_type, OrderType::Limit) && request.price.is_none() {
        return Err(Error::InvalidOrder(
            "Limit orders require price parameter".into(),
        ));
    }

    let mut map = Map::new();

    // Required fields
    map.insert(
        "currency_pair".to_string(),
        Value::String(to_exchange_symbol(&market.id)),
    );
    map.insert("side".to_string(), Value::String(map_side(request.side)));
    map.insert(
        "type".to_string(),
        Value::String(map_order_type(request.order_type)),
    );
    map.insert(
        "amount".to_string(),
        Value::String(request.amount.as_decimal().to_string()),
    );

    // Price for limit orders
    if let Some(price) = request.price {
        map.insert(
            "price".to_string(),
            Value::String(price.as_decimal().to_string()),
        );
    }

    // Optional: Client order ID
    if let Some(ref client_order_id) = request.client_order_id {
        map.insert(
            "text".to_string(),
            Value::String(format!("t-{}", client_order_id)),
        );
    }

    // Optional: Time in force
    // Gate API requirement: Market orders MUST use "ioc" time_in_force
    let effective_tif = if request.order_type == OrderType::Market {
        // Force IOC for market orders
        Some("ioc")
    } else if let Some(tif) = request.time_in_force {
        Some(map_time_in_force(tif))
    } else {
        None
    };

    if let Some(tif_str) = effective_tif {
        map.insert(
            "time_in_force".to_string(),
            Value::String(tif_str.to_string()),
        );
    }

    let result = Value::Object(map);
    Ok(result)
}

// ============================================================================
// Cancel Order
// ============================================================================

/// Build cancel order request body
///
/// Gate cancel order uses query parameters, not body.
/// This function returns an empty JSON object as placeholder.
pub fn build_cancel_order_payload(_id: &str, _market: &Market) -> Value {
    // Gate uses path parameter for order ID
    // and query parameter for currency_pair
    serde_json::json!({})
}

// ============================================================================
// Create Contract Order (Swap/Futures)
// ============================================================================

/// Build create order request body for CONTRACT (swap/futures) trading
///
/// # Gate API v4 Requirements
///
/// ## Required Fields
///
/// - `contract`: Contract name (e.g., "BTC_USDT")
/// - `size`: Order size (positive for long, negative for short)
/// - `price`: Required for limit orders
///
/// ## Optional Fields
///
/// - `price`: Order price (limit orders)
/// - `tif`: Time in force (gtc/ioc/fok/poc)
/// - `reduce_only`: True for closing positions only
/// - `close`: True to close entire position (ignores size)
/// - `text`: Client order ID (prefix with "t-")
///
/// # Important Notes
///
/// 1. **Size Sign Convention**:
///    - Positive size → Buy/Long order
///    - Negative size → Sell/Short order
///
/// 2. **Close Order**:
///    - Set `close=true` to close entire position
///    - When close=true, size field is ignored
///
/// 3. **Market Orders**:
///    - MUST use tif="ioc" (Immediate or Cancel)
///
/// # Parameters
///
/// - `request`: Unified order request
/// - `market`: Market information
///
/// # Errors
///
/// - If order type is not supported, returns `Error::InvalidOrder`
/// - If limit order without price, returns `Error::InvalidOrder`
pub fn build_create_contract_order_payload(
    request: &OrderRequest,
    market: &Market,
) -> Result<Value, Error> {
    // Validate 1: Only support market and limit orders for now
    // TODO: Add support for stop_loss, take_profit, etc.
    if !matches!(request.order_type, OrderType::Market | OrderType::Limit) {
        return Err(Error::InvalidOrder(
            format!(
                "Gate contract does not support {:?} orders yet",
                request.order_type
            )
            .into(),
        ));
    }

    // Validate 2: Limit orders require price
    if matches!(request.order_type, OrderType::Limit) && request.price.is_none() {
        return Err(Error::InvalidOrder(
            "Limit orders require price parameter".into(),
        ));
    }

    let mut map = Map::new();

    // Required: Contract name (use market.id directly, already in exchange format)
    map.insert("contract".to_string(), Value::String(market.id.clone()));

    // Required: Size with sign (positive=buy, negative=sell)
    // Gate contract API uses size sign to determine direction, not a separate side field
    let size_decimal = request.amount.as_decimal();
    let size_with_sign = if request.side == OrderSide::Buy {
        size_decimal
    } else {
        -size_decimal
    };
    map.insert(
        "size".to_string(),
        Value::String(size_with_sign.to_string()),
    );

    // Price: required for all orders
    // - Limit orders: actual price
    // - Market orders: "0" (indicates market order)
    let price_str = if request.order_type == OrderType::Market {
        "0".to_string()
    } else if let Some(price) = request.price {
        price.as_decimal().to_string()
    } else {
        return Err(Error::InvalidOrder(
            "Limit orders require price parameter".into(),
        ));
    };
    map.insert("price".to_string(), Value::String(price_str));

    // Time in force
    // Market orders MUST use "ioc" (Immediate-Or-Cancel)
    // Limit orders: use specified tif or default to "gtc"
    let tif_str = if request.order_type == OrderType::Market {
        "ioc".to_string()
    } else if let Some(tif) = request.time_in_force {
        map_time_in_force(tif).to_string()
    } else {
        "gtc".to_string()
    };
    map.insert("tif".to_string(), Value::String(tif_str));

    // Reduce only (for closing positions)
    if request.reduce_only.unwrap_or(false) {
        map.insert("reduce_only".to_string(), Value::Bool(true));
    }

    // Optional: Client order ID (Gate requires "t-" prefix)
    if let Some(ref client_order_id) = request.client_order_id {
        map.insert(
            "text".to_string(),
            Value::String(format!("t-{}", client_order_id)),
        );
    }

    // Gate contract regular orders do NOT support take_profit/stop_loss
    // TP/SL must be created using price-triggered orders API:
    // POST /api/v4/futures/{settle}/price_orders
    if request.tp_limit_price.is_some() {
        return Err(Error::InvalidOrder(
            "Gate contract regular orders do not support tp_limit_price. \
             Use price-triggered orders API: POST /futures/{settle}/price_orders"
                .into(),
        ));
    }
    if request.stop_price.is_some() {
        return Err(Error::InvalidOrder(
            "Gate contract regular orders do not support stop_price. \
             Use price-triggered orders API: POST /futures/{settle}/price_orders"
                .into(),
        ));
    }

    Ok(Value::Object(map))
}

// ============================================================================
// Cancel Contract Order
// ============================================================================

/// Build cancel contract order request body
///
/// Contract cancel uses path parameter for order ID, not body.
pub fn build_cancel_contract_order_payload() -> Value {
    serde_json::json!({})
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Map OrderSide to Gate API format
fn map_side(side: OrderSide) -> String {
    match side {
        OrderSide::Buy => "buy".to_string(),
        OrderSide::Sell => "sell".to_string(),
    }
}

/// Map OrderType to Gate API format
fn map_order_type(order_type: OrderType) -> String {
    match order_type {
        OrderType::Market => "market".to_string(),
        OrderType::Limit => "limit".to_string(),
        _ => unreachable!(), // Already validated in build_create_order_payload
    }
}

/// Map TimeInForce to Gate API format
fn map_time_in_force(tif: TimeInForce) -> &'static str {
    match tif {
        TimeInForce::GTC => "gtc",
        TimeInForce::IOC => "ioc",
        TimeInForce::FOK => "fok",
        TimeInForce::PO => "poc",
        TimeInForce::GTD => "gtc", // Gate doesn't support GTD, use GTC
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_side() {
        assert_eq!(map_side(OrderSide::Buy), "buy");
        assert_eq!(map_side(OrderSide::Sell), "sell");
    }

    #[test]
    fn test_map_order_type() {
        assert_eq!(map_order_type(OrderType::Market), "market");
        assert_eq!(map_order_type(OrderType::Limit), "limit");
    }

    #[test]
    fn test_map_time_in_force() {
        assert_eq!(map_time_in_force(TimeInForce::GTC), "gtc");
        assert_eq!(map_time_in_force(TimeInForce::IOC), "ioc");
        assert_eq!(map_time_in_force(TimeInForce::FOK), "fok");
        assert_eq!(map_time_in_force(TimeInForce::PO), "poc");
    }
}

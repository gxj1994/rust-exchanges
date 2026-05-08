//! Bybit Take Profit / Stop Loss Order Tests
//!
//! Tests for TP/SL order types:
//! - Market orders with TP/SL
//! - Limit orders with TP/SL
//! - Position TP/SL modification
//!
//! ## Prerequisites
//!
//! - Bybit API credentials configured
//! - Sufficient balance for test orders
//! - Can run on testnet (set BYBIT_TESTNET=true)

use crate::bybit::support::{create_auth_bybit, price_to_decimal};
use crate::support::should_skip_private_tests;
use ccxt_core::types::order::PositionSide;
use ccxt_core::types::{
    Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

macro_rules! skip_if_no_credentials {
    () => {
        if should_skip_private_tests("bybit") {
            println!("SKIPPED: No Bybit credentials configured");
            return;
        }
    };
}

// ============================================================================
// Market Order with TP/SL
// ============================================================================

/// Test: Market order with take profit
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_market_order_with_tp() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let current_price: Decimal = price_to_decimal(last_price);

    // Market order with take profit
    let tp_price = current_price * dec!(1.05); // 5% above
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.001)))
        .tp_limit_price(Price::new(tp_price))
        .position_side(PositionSide::Long)
        .build()
        .expect("Failed to build request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with TP");

    assert_eq!(order.status, OrderStatus::Closed);
    println!("✅ Market order with TP created: {}", order.id);
}

/// Test: Market order with stop loss
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_market_order_with_sl() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let current_price: Decimal = price_to_decimal(last_price);

    // Market order with stop loss
    let sl_price = current_price * dec!(0.95); // 5% below
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.001)))
        .sl_limit_price(Price::new(sl_price))
        .position_side(PositionSide::Long)
        .build()
        .expect("Failed to build request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with SL");

    assert_eq!(order.status, OrderStatus::Closed);
    println!("✅ Market order with SL created: {}", order.id);
}

/// Test: Market order with both TP and SL
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_market_order_with_tp_and_sl() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let current_price: Decimal = price_to_decimal(last_price);

    // Market order with TP and SL
    let tp_price = current_price * dec!(1.05);
    let sl_price = current_price * dec!(0.95);
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.001)))
        .tp_limit_price(Price::new(tp_price))
        .sl_limit_price(Price::new(sl_price))
        .position_side(PositionSide::Long)
        .build()
        .expect("Failed to build request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with TP/SL");

    assert_eq!(order.status, OrderStatus::Closed);
    println!("✅ Market order with TP/SL created: {}", order.id);
}

// ============================================================================
// Limit Order with TP/SL
// ============================================================================

/// Test: Limit order with take profit
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_limit_order_with_tp() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let current_price: Decimal = price_to_decimal(last_price);

    let entry_price = current_price * dec!(0.98); // 2% below market
    let tp_price = current_price * dec!(1.05);

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.001)))
        .price(Price::new(entry_price))
        .time_in_force(TimeInForce::GTC)
        .tp_limit_price(Price::new(tp_price))
        .position_side(PositionSide::Long)
        .build()
        .expect("Failed to build request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create limit order with TP");

    assert_eq!(order.status, OrderStatus::Open);

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
}

/// Test: Limit order with stop loss
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_limit_order_with_sl() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let current_price: Decimal = price_to_decimal(last_price);

    let entry_price = current_price * dec!(0.98);
    let sl_price = current_price * dec!(0.93);

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.001)))
        .price(Price::new(entry_price))
        .time_in_force(TimeInForce::GTC)
        .sl_limit_price(Price::new(sl_price))
        .position_side(PositionSide::Long)
        .build()
        .expect("Failed to build request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create limit order with SL");

    assert_eq!(order.status, OrderStatus::Open);

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
}

/// Test: Limit order with both TP and SL
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_limit_order_with_tp_and_sl() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let current_price: Decimal = price_to_decimal(last_price);

    let entry_price = current_price * dec!(0.98);
    let tp_price = current_price * dec!(1.05);
    let sl_price = current_price * dec!(0.93);

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.001)))
        .price(Price::new(entry_price))
        .time_in_force(TimeInForce::GTC)
        .tp_limit_price(Price::new(tp_price))
        .sl_limit_price(Price::new(sl_price))
        .position_side(PositionSide::Long)
        .build()
        .expect("Failed to build request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create limit order with TP/SL");

    assert_eq!(order.status, OrderStatus::Open);

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
}

// ============================================================================
// Trigger Price Type Tests
// ============================================================================

/// Test: TP/SL with mark price trigger
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_tp_sl_mark_price_trigger() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let current_price: Decimal = price_to_decimal(last_price);

    use ccxt_core::types::order::order_request::TriggerPriceType;

    // Market order with TP/SL triggered by mark price
    let tp_price = current_price * dec!(1.05);
    let sl_price = current_price * dec!(0.95);
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.001)))
        .tp_limit_price(Price::new(tp_price))
        .sl_limit_price(Price::new(sl_price))
        .trigger_price_type(TriggerPriceType::MarkPrice)
        .position_side(PositionSide::Long)
        .build()
        .expect("Failed to build request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with mark price trigger");

    assert_eq!(order.status, OrderStatus::Closed);
    println!("✅ Order with mark price trigger created: {}", order.id);
}

/// Test: TP/SL with index price trigger
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_tp_sl_index_price_trigger() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let current_price: Decimal = price_to_decimal(last_price);

    use ccxt_core::types::order::order_request::TriggerPriceType;

    let tp_price = current_price * dec!(1.05);
    let sl_price = current_price * dec!(0.95);
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.001)))
        .tp_limit_price(Price::new(tp_price))
        .sl_limit_price(Price::new(sl_price))
        .trigger_price_type(TriggerPriceType::IndexPrice)
        .position_side(PositionSide::Long)
        .build()
        .expect("Failed to build request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with index price trigger");

    assert_eq!(order.status, OrderStatus::Closed);
    println!("✅ Order with index price trigger created: {}", order.id);
}

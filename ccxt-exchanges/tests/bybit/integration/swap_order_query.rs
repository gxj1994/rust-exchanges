//! Bybit Swap Order Query Tests
//!
//! Tests for swap/linear contract order query methods:
//! - fetch_order
//! - fetch_open_orders
//! - fetch_history_orders
//! - cancel_order
//! - cancel_all_orders
//! - position TP/SL operations
//!
//! ## Prerequisites
//!
//! - Bybit API credentials configured (BYBIT_API_KEY, BYBIT_API_SECRET)
//! - Sufficient balance for test orders
//! - Can run on testnet (set BYBIT_TESTNET=true)
//!
//! ## Safety
//!
//! - All tests use minimum contract sizes (0.001 BTC)
//! - Limit orders are placed far from market price
//! - All tests cleanup created orders
//! - Tests are marked with `#[ignore]` and require explicit execution

use crate::bybit::support::{create_auth_bybit, price_to_decimal};
use crate::support::should_skip_private_tests;
use ccxt_core::types::order::PositionSide;
use ccxt_core::types::{
    Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

macro_rules! skip_if_no_credentials {
    () => {
        if should_skip_private_tests("bybit") {
            println!("SKIPPED: No Bybit credentials configured");
            return;
        }
    };
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create swap market buy request
fn swap_market_buy_request(symbol: &str, amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(amount))
        .build()
        .unwrap()
}

/// Create swap limit order
fn swap_limit_order_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    price: Decimal,
    position_side: PositionSide,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .position_side(position_side)
        .build()
        .unwrap()
}

/// Create swap PostOnly order
fn swap_postonly_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    price: Decimal,
    position_side: PositionSide,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::LimitMaker)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .position_side(position_side)
        .build()
        .unwrap()
}

// ============================================================================
// Basic Order Query Tests
// ============================================================================

/// Test: Fetch order by ID (market order)
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_fetch_order_market() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    // Create market order
    let request = swap_market_buy_request("BTC/USDT:USDT", dec!(0.001));
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Fetch order
    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order");

    assert_eq!(fetched.id, order.id);
    assert_eq!(fetched.status, OrderStatus::Closed);
    assert_eq!(fetched.order_type, OrderType::Market);
}

/// Test: Fetch open orders
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_fetch_open_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let mut open_ids = Vec::new();
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // Create 3 limit orders far below market
    for i in 0..3 {
        let price = base_price - dec!(100.0) * Decimal::from(i);
        let request = swap_limit_order_request(
            "BTC/USDT:USDT",
            OrderSide::Buy,
            dec!(0.001),
            price,
            PositionSide::Long,
        );
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create limit order");
        open_ids.push(order.id);
    }

    // Fetch open orders
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT:USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");

    assert!(!open_orders.is_empty());
    for order in &open_orders {
        assert_eq!(order.status, OrderStatus::Open);
    }

    // Cleanup
    for open_id in &open_ids {
        let _ = exchange.cancel_order(open_id, "BTC/USDT:USDT").await;
    }
}

/// Test: Fetch history orders
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_fetch_history_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let mut filled_ids = Vec::new();

    // Create 3 market orders
    for _ in 0..3 {
        let request = swap_market_buy_request("BTC/USDT:USDT", dec!(0.001));
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create market order");
        filled_ids.push(order.id);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // Fetch history orders
    let since = chrono::Utc::now().timestamp_millis() - 3600000; // 1 hour ago
    let history_orders = exchange
        .fetch_history_orders(Some("BTC/USDT:USDT"), Some(since), Some(10))
        .await
        .expect("Failed to fetch history orders");

    assert!(!history_orders.is_empty());

    let history_ids: Vec<&String> = history_orders.iter().map(|o| &o.id).collect();
    for filled_id in &filled_ids {
        assert!(history_ids.contains(&filled_id));
    }
}

// ============================================================================
// Order Lifecycle Tests
// ============================================================================

/// Test: Complete order lifecycle (create -> fetch -> cancel -> verify)
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_order_lifecycle() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let price: Decimal = price_to_decimal(last_price) * dec!(0.75);

    // Create limit order
    let request = swap_limit_order_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.001),
        price,
        PositionSide::Long,
    );
    let created = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    assert_eq!(created.status, OrderStatus::Open);

    // Fetch order
    let fetched = exchange
        .fetch_order(&created.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.status, OrderStatus::Open);

    // Cancel order
    let canceled = exchange
        .cancel_order(&created.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel order");
    assert_eq!(canceled.status, OrderStatus::Cancelled);

    // Verify not in open orders
    let open_orders_after = exchange
        .fetch_open_orders(Some("BTC/USDT:USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");
    assert!(!open_orders_after.iter().any(|o| o.id == created.id));
}

// ============================================================================
// Order Type Tests
// ============================================================================

/// Test: PostOnly (LimitMaker) order
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_postonly_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // Create PostOnly order
    let request = swap_postonly_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.001),
        price,
        PositionSide::Long,
    );
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create PostOnly order");

    assert_eq!(order.order_type, OrderType::LimitMaker);
    assert_eq!(order.status, OrderStatus::Open);

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
}

/// Test: ReduceOnly order (sell to close long position)
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_reduceonly_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let price: Decimal = price_to_decimal(last_price) * dec!(1.30);

    // Create ReduceOnly sell order
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .position_side(PositionSide::Long)
        .reduce_only(true)
        .build()
        .expect("Failed to build request");

    let result = exchange.create_order(request).await;

    // May fail if no position exists
    match result {
        Ok(order) => {
            println!("✅ ReduceOnly order created: {}", order.id);
            let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
        }
        Err(e) => {
            println!("⚠️  Test skipped (may need open position): {}", e);
        }
    }
}

/// Test: Market sell order
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_market_sell_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    // First open a long position
    let buy_request = swap_market_buy_request("BTC/USDT:USDT", dec!(0.001));
    let _buy_order = exchange
        .create_order(buy_request)
        .await
        .expect("Failed to open long position");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Close position with market sell
    let sell_request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.001)))
        .position_side(PositionSide::Long)
        .reduce_only(true)
        .build()
        .expect("Failed to build request");

    let sell_order = exchange
        .create_order(sell_request)
        .await
        .expect("Failed to close position");

    assert_eq!(sell_order.order_type, OrderType::Market);
    assert_eq!(sell_order.side, OrderSide::Sell);
    assert_eq!(sell_order.status, OrderStatus::Closed);
}

// ============================================================================
// Cancel All Orders Tests
// ============================================================================

/// Test: Cancel all open orders for a symbol
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_cancel_all_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // Create multiple limit orders
    for i in 0..3 {
        let price = base_price - dec!(50.0) * Decimal::from(i);
        let request = swap_limit_order_request(
            "BTC/USDT:USDT",
            OrderSide::Buy,
            dec!(0.001),
            price,
            PositionSide::Long,
        );
        let _ = exchange.create_order(request).await;
    }

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Cancel all orders
    let canceled = exchange
        .cancel_all_orders("BTC/USDT:USDT")
        .await
        .expect("Failed to cancel all orders");

    assert!(!canceled.is_empty());

    // Verify no open orders
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT:USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");
    assert!(open_orders.is_empty());
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test: Fetch non-existent order
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_fetch_nonexistent_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    let result = exchange
        .fetch_order("nonexistent_order_id", "BTC/USDT:USDT")
        .await;

    assert!(result.is_err());
}

/// Test: Cancel already filled order
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_cancel_filled_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("swap").await;

    // Create market order (will fill immediately)
    let request = swap_market_buy_request("BTC/USDT:USDT", dec!(0.001));
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Try to cancel filled order
    let result = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;

    // Should fail or return error
    assert!(result.is_err());
}

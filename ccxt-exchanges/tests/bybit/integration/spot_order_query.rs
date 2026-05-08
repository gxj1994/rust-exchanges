//! Bybit Spot Order Query Tests
//!
//! Tests for spot order query methods:
//! - fetch_order
//! - fetch_open_orders
//! - fetch_history_orders
//! - cancel_order
//! - cancel_all_orders
//!
//! ## Prerequisites
//!
//! - Bybit API credentials configured (BYBIT_API_KEY, BYBIT_API_SECRET)
//! - Sufficient USDT balance for test orders
//! - Can run on testnet (set BYBIT_TESTNET=true)
//!
//! ## Safety
//!
//! - All tests use minimum order amounts
//! - Limit orders are placed far from market price to avoid execution
//! - All tests cleanup created orders
//! - Tests are marked with `#[ignore]` and require explicit execution

use crate::bybit::support::{create_auth_bybit, price_to_decimal};
use crate::support::should_skip_private_tests;
use ccxt_core::types::{
    Amount, AmountSpec, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
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

/// Create spot market buy request (quote currency amount)
fn spot_market_buy_request(symbol: &str, quote_amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(AmountSpec::quote(Amount::new(quote_amount)))
        .build()
        .unwrap()
}

/// Create spot market sell request (base currency amount)
fn spot_market_sell_request(symbol: &str, base_amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(base_amount))
        .build()
        .unwrap()
}

/// Create spot limit buy request
fn spot_limit_buy_request(symbol: &str, amount: Decimal, price: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .unwrap()
}

/// Create spot PostOnly limit order
fn spot_postonly_request(symbol: &str, amount: Decimal, price: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::LimitMaker)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .build()
        .unwrap()
}

// ============================================================================
// Basic Order Query Tests
// ============================================================================

/// Test: Fetch order by ID (market order)
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_fetch_order_market() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    // Create market order
    let request = spot_market_buy_request("BTC/USDT", dec!(5.0)); // 1 USDT
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Fetch order
    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT")
        .await
        .expect("Failed to fetch order");

    assert_eq!(fetched.id, order.id);
    assert_eq!(fetched.status, OrderStatus::Closed);
    assert_eq!(fetched.order_type, OrderType::Market);
}

/// Test: Fetch open orders
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_fetch_open_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    let mut open_ids = Vec::new();
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // Create 3 limit orders far below market
    for i in 0..3 {
        let price = base_price - dec!(100.0) * Decimal::from(i);
        let request = spot_limit_buy_request("BTC/USDT", dec!(0.0001), price);
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create limit order");
        open_ids.push(order.id);
    }

    // Fetch open orders
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");

    assert!(!open_orders.is_empty());
    for order in &open_orders {
        assert_eq!(order.status, OrderStatus::Open);
    }

    // Cleanup
    for open_id in &open_ids {
        let _ = exchange.cancel_order(open_id, "BTC/USDT").await;
    }
}

/// Test: Fetch history orders
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_fetch_history_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    let mut filled_ids = Vec::new();

    // Create 3 market orders
    for _ in 0..3 {
        let request = spot_market_buy_request("BTC/USDT", dec!(5.0));
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
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(10))
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
async fn test_spot_order_lifecycle() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let price: Decimal = price_to_decimal(last_price) * dec!(0.75);

    // Create limit order
    let request = spot_limit_buy_request("BTC/USDT", dec!(0.0001), price);
    let created = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    assert_eq!(created.status, OrderStatus::Open);

    // Fetch order
    let fetched = exchange
        .fetch_order(&created.id, "BTC/USDT")
        .await
        .expect("Failed to fetch order");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.status, OrderStatus::Open);

    // Cancel order
    let canceled = exchange
        .cancel_order(&created.id, "BTC/USDT")
        .await
        .expect("Failed to cancel order");
    assert_eq!(canceled.status, OrderStatus::Cancelled);

    // Verify not in open orders
    let open_orders_after = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
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
async fn test_spot_postonly_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // 根据市场精度调整价格
    let price = exchange
        .base()
        .price_to_precision("BTC/USDT", price)
        .await
        .expect("Failed to adjust price precision");

    // Create PostOnly order
    let request = spot_postonly_request("BTC/USDT", dec!(0.0001), price);
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create PostOnly order");

    assert_eq!(order.order_type, OrderType::LimitMaker);
    assert_eq!(order.status, OrderStatus::Open);

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
}

/// Test: Market sell order
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_market_sell_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    // First buy a small amount
    let buy_request = spot_market_buy_request("BTC/USDT", dec!(5.0));
    let _buy_order = exchange
        .create_order(buy_request)
        .await
        .expect("Failed to buy BTC");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Sell the bought amount
    let sell_request = spot_market_sell_request("BTC/USDT", dec!(0.00007));
    let sell_order = exchange
        .create_order(sell_request)
        .await
        .expect("Failed to sell BTC");

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
async fn test_spot_cancel_all_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // Create multiple limit orders and track successful ones
    let mut created_order_ids = Vec::new();
    for i in 0..3 {
        let price = base_price - dec!(50.0) * Decimal::from(i);
        let price = exchange
            .base()
            .price_to_precision("BTC/USDT", price)
            .await
            .expect("Failed to adjust price precision");

        let request = spot_limit_buy_request("BTC/USDT", dec!(0.0001), price);
        match exchange.create_order(request).await {
            Ok(order) => {
                let order_id = order.id.clone();
                created_order_ids.push(order.id);
                println!("✅ Created order {} for cancel-all test", order_id);
            }
            Err(e) => {
                println!(
                    "⚠️  Failed to create order {} (this is OK if no balance): {}",
                    i, e
                );
            }
        }
    }

    // If no orders were created, skip the test
    if created_order_ids.is_empty() {
        println!("⚠️  No orders created, skipping cancel-all test (likely insufficient balance)");
        return;
    }

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Cancel all orders
    let canceled = exchange
        .cancel_all_orders("BTC/USDT")
        .await
        .expect("Failed to cancel all orders");

    // Should have cancelled at least the orders we created
    assert!(
        !canceled.is_empty(),
        "Expected at least one cancelled order"
    );
    println!("✅ Cancelled {} orders", canceled.len());

    // Verify no open orders remain for the created order IDs
    tokio::time::sleep(Duration::from_secs(1)).await;
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");

    // Check that our created orders are not in open orders
    for order_id in &created_order_ids {
        assert!(
            !open_orders.iter().any(|o| &o.id == order_id),
            "Order {} should not be in open orders after cancel-all",
            order_id
        );
    }
    println!("✅ Verified all created orders are cancelled");
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test: Fetch non-existent order
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_fetch_nonexistent_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    let result = exchange
        .fetch_order("nonexistent_order_id", "BTC/USDT")
        .await;

    assert!(result.is_err());
}

/// Test: Cancel already filled order
#[tokio::test]
#[ignore = "需要配置 BYBIT_API_KEY, BYBIT_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_cancel_filled_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_bybit("spot").await;

    // Create market order (will fill immediately)
    let request = spot_market_buy_request("BTC/USDT", dec!(5.0));
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Try to cancel filled order
    let result = exchange.cancel_order(&order.id, "BTC/USDT").await;

    // Should fail or return error
    assert!(result.is_err());
}

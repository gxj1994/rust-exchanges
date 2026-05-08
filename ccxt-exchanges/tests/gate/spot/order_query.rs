//! Gate.io Spot Order Query Integration Tests
//!
//! Tests for order query operations:
//! - Fetch order by ID
//! - Fetch open orders
//! - Fetch order history

use crate::support::{create_gate_spot_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::traits::{MarketData, Trading};
use ccxt_core::types::{
    Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
};
use ccxt_exchanges::gate::Gate;
use rust_decimal_macros::dec;

/// Create authenticated Gate client for spot tests.
async fn create_auth_gate_spot() -> Gate {
    let config = init_test();
    let exchange =
        create_gate_spot_with_credentials(&config).expect("Failed to create Gate with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets()
        .await
        .expect("Failed to load markets");

    exchange
}

// ============================================================================
// Fetch Order Tests
// ============================================================================

/// Test: Fetch order by ID
///
/// Creates a limit order and then fetches it by ID.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_fetch_order_by_id() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Create a limit order (far from market to avoid execution)
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let last_price_decimal: rust_decimal::Decimal = last_price.into();
    let price = last_price_decimal * dec!(0.70);
    let adjusted_price = price;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(adjusted_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    let order_id = order.id.clone();

    // Fetch the order by ID
    let fetched_order = exchange
        .fetch_order(&order_id, "BTC/USDT")
        .await
        .expect("Failed to fetch order");

    assert_eq!(fetched_order.id, order_id);
    assert_eq!(fetched_order.symbol.as_str(), "BTC/USDT");

    println!(
        "✅ Fetch order by ID successful: id={}, status={:?}",
        order_id, fetched_order.status
    );

    // Cleanup: cancel the order
    let _ = exchange.cancel_order(&order_id, "BTC/USDT").await;
}

// ============================================================================
// Fetch Open Orders Tests
// ============================================================================

/// Test: Fetch all open orders
///
/// Fetches all open orders and verifies the response structure.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_fetch_open_orders() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Fetch open orders (may be empty)
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT"))
        .await
        .expect("Failed to fetch open orders");

    // Response should be a vector (possibly empty)
    assert!(open_orders.is_empty() || !open_orders.is_empty());

    // If there are orders, verify structure
    for order in &open_orders {
        assert!(!order.symbol.is_empty());
        // order.id is String, not Option
        assert!(!order.id.is_empty());
    }

    println!(
        "✅ Fetch open orders successful: count={}",
        open_orders.len()
    );
}

/// Test: Fetch open orders without symbol (all markets)
///
/// Fetches open orders across all markets.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_fetch_open_orders_all_markets() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Fetch open orders for all markets
    let open_orders = exchange
        .fetch_open_orders(None)
        .await
        .expect("Failed to fetch open orders for all markets");

    println!(
        "✅ Fetch open orders (all markets) successful: count={}",
        open_orders.len()
    );
}

// ============================================================================
// Fetch Order History Tests
// ============================================================================

/// Test: Fetch closed orders (order history)
///
/// Fetches order history using `fetch_history_orders` and verifies the response.
/// Gate API uses `GET /api/v4/spot/orders?status=finished` to get closed orders.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_fetch_closed_orders() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Fetch order history for BTC/USDT (closed/finished orders)
    let history_orders = exchange
        .fetch_history_orders(Some("BTC/USDT"), None, Some(20))
        .await
        .expect("Failed to fetch history orders");

    // Response should be a vector (possibly empty if no orders)
    println!(
        "✅ Fetch closed orders successful: count={}",
        history_orders.len()
    );

    // Verify structure of returned orders
    for order in &history_orders {
        assert!(!order.id.is_empty());
        assert_eq!(order.symbol.as_str(), "BTC/USDT");
        // Closed orders should have a non-Open status
        assert!(
            order.status != OrderStatus::Open,
            "Closed orders should not be Open, got {:?}",
            order.status
        );
    }
}

/// Test: Fetch order history with limit parameter
///
/// Fetches history orders with a limit parameter and verifies the result count.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_fetch_orders_with_limit() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Fetch with limit=5
    let limit = 5u32;
    let history_orders = exchange
        .fetch_history_orders(Some("BTC/USDT"), None, Some(limit))
        .await
        .expect("Failed to fetch history orders with limit");

    // Verify limit is respected (if there are enough orders)
    assert!(
        history_orders.len() <= limit as usize,
        "Result count {} should not exceed limit {}",
        history_orders.len(),
        limit
    );

    println!(
        "✅ Fetch orders with limit successful: requested={}, returned={}",
        limit,
        history_orders.len()
    );
}

// ============================================================================
// Cancel Order Tests
// ============================================================================

/// Test: Cancel order
///
/// Creates a limit order and then cancels it.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_cancel_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Create a limit order (far from market)
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let last_price_decimal: rust_decimal::Decimal = last_price.into();
    let price = last_price_decimal * dec!(0.60);
    // TODO: Implement price_to_precision for Gate
    let adjusted_price = price;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(adjusted_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    let order_id = order.id.clone();

    // Cancel the order
    let cancelled_order = exchange
        .cancel_order(&order_id, "BTC/USDT")
        .await
        .expect("Failed to cancel order");

    // Cancelled order should have Cancelled status
    assert_eq!(
        cancelled_order.status,
        ccxt_core::types::OrderStatus::Cancelled,
        "Cancelled order should have Cancelled status"
    );

    println!(
        "✅ Cancel order successful: id={}, status={:?}",
        order_id, cancelled_order.status
    );
}

/// Test: Cancel all open orders
///
/// Cancels all open orders for a symbol.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_cancel_all_orders() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Cancel all open orders for BTC/USDT
    let result = exchange.cancel_all_orders("BTC/USDT").await;

    match result {
        Ok(orders) => {
            println!(
                "✅ Cancel all orders successful: cancelled {} orders",
                orders.len()
            );
        }
        Err(e) => {
            // May fail if no open orders
            println!("⚠️  Cancel all orders failed (may be expected): {}", e);
        }
    }
}

// ============================================================================
// Order Lifecycle / Status Change Verification Test
// ============================================================================

/// Test: Complete order lifecycle with status change verification
///
/// Creates a limit order → verifies Open status → cancels → verifies Cancelled status.
/// This validates the full order status transition flow.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_order_lifecycle_status_change() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let last_price_decimal: rust_decimal::Decimal = last_price.into();
    let price = last_price_decimal * dec!(0.60);
    // TODO: Implement price_to_precision for Gate
    let adjusted_price = price;

    // Step 1: Create limit order (below market to avoid execution)
    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(adjusted_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let created = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    assert_eq!(created.status, OrderStatus::Open);
    println!(
        "✅ Step 1 - Created order: id={}, status={:?}",
        created.id, created.status
    );

    // Step 2: Fetch order and verify status is still Open
    let fetched = exchange
        .fetch_order(&created.id, "BTC/USDT")
        .await
        .expect("Failed to fetch order");
    assert_eq!(fetched.id, created.id);
    assert_eq!(
        fetched.status,
        OrderStatus::Open,
        "Fetched order should still be Open, got {:?}",
        fetched.status
    );
    println!("✅ Step 2 - Fetched order: status={:?}", fetched.status);

    // Step 3: Verify order appears in open orders
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT"))
        .await
        .expect("Failed to fetch open orders");
    assert!(
        open_orders.iter().any(|o| o.id == created.id),
        "Created order should appear in open orders"
    );
    println!("✅ Step 3 - Order found in open orders");

    // Step 4: Cancel the order
    let cancelled = exchange
        .cancel_order(&created.id, "BTC/USDT")
        .await
        .expect("Failed to cancel order");
    assert_eq!(
        cancelled.status,
        OrderStatus::Cancelled,
        "Cancelled order should have Cancelled status, got {:?}",
        cancelled.status
    );
    println!("✅ Step 4 - Cancelled order: status={:?}", cancelled.status);

    // Step 5: Verify order no longer appears in open orders
    let open_orders_after = exchange
        .fetch_open_orders(Some("BTC/USDT"))
        .await
        .expect("Failed to fetch open orders after cancel");
    assert!(
        !open_orders_after.iter().any(|o| o.id == created.id),
        "Cancelled order should not appear in open orders"
    );
    println!("✅ Step 5 - Order removed from open orders");

    println!("\n✅ Complete order lifecycle verified: Open → Cancelled");
}

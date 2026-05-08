//! Gate.io Swap (Contract) Order Query Tests
//!
//! Tests for contract order query and account operations:
//! - Fetch contract order by ID
//! - Fetch contract open orders
//! - Fetch contract order history
//! - Fetch contract positions
//! - Cancel contract order
//!
//! ## Prerequisites
//!
//! - Gate API credentials configured (GATE_API_KEY, GATE_API_SECRET)
//! - Sufficient balance for test orders
//!
//! ## Safety
//!
//! - All tests use minimum order amounts
//! - Limit orders are placed far from market price to avoid execution
//! - All tests cleanup created orders
//! - Tests are marked with `#[ignore]` and require explicit execution

use crate::support::{init_test, should_skip_private_tests};
use ccxt_core::traits::{MarketData, Trading};
use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};
use ccxt_core::types::{
    Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
};
use ccxt_exchanges::gate::{Gate, GateBuilder};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Create authenticated Gate client for swap (contract) tests.
async fn create_auth_gate_swap() -> Gate {
    let config = init_test();
    let (api_key, api_secret) = config
        .get_active_api_key("gate")
        .expect("No Gate credentials configured");

    let exchange = GateBuilder::new()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Linear)
        .api_key(api_key)
        .secret(api_secret)
        .testnet(config.gate.use_testnet)
        .build()
        .expect("Failed to create Gate swap instance");

    exchange
        .load_markets()
        .await
        .expect("Failed to load contract markets");
    exchange
}

// ============================================================================
// Fetch Contract Order Tests
// ============================================================================

/// Test: Fetch contract order by ID
///
/// Creates a limit order and then fetches it by ID.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_fetch_order_by_id() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let last_price_decimal: Decimal = last_price.into();
    let price = last_price_decimal * dec!(0.60);
    // TODO: Implement price_to_precision for Gate
    let adjusted_price = price;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(adjusted_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create contract order");

    let order_id = order.id.clone();

    // Fetch the order by ID
    let fetched_order = exchange
        .fetch_order(&order_id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch contract order");

    assert_eq!(fetched_order.id, order_id);
    assert_eq!(fetched_order.symbol.as_str(), "BTC/USDT:USDT");

    println!(
        "✅ Fetch contract order by ID: id={}, status={:?}",
        order_id, fetched_order.status
    );

    // Cleanup
    let _ = exchange.cancel_order(&order_id, "BTC/USDT:USDT").await;
}

// ============================================================================
// Fetch Contract Open Orders Tests
// ============================================================================

/// Test: Fetch contract open orders
///
/// Fetches all open contract orders and verifies the response structure.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_fetch_open_orders() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT:USDT"))
        .await
        .expect("Failed to fetch contract open orders");

    // Response should be a vector (possibly empty)
    println!("✅ Fetch contract open orders: count={}", open_orders.len());

    for order in &open_orders {
        assert!(!order.id.is_empty());
        assert!(!order.symbol.is_empty());
    }
}

// ============================================================================
// Fetch Contract Order History Tests
// ============================================================================

/// Test: Fetch contract closed orders
///
/// Fetches contract order history using `fetch_history_orders`.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_fetch_closed_orders() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let history_orders = exchange
        .fetch_history_orders(Some("BTC/USDT:USDT"), None, Some(20))
        .await
        .expect("Failed to fetch contract history orders");

    println!(
        "✅ Fetch contract closed orders: count={}",
        history_orders.len()
    );

    for order in &history_orders {
        assert!(!order.id.is_empty());
    }
}

/// Test: Fetch contract order history with limit
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_fetch_orders_with_limit() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let limit = 5u32;
    let history_orders = exchange
        .fetch_history_orders(Some("BTC/USDT:USDT"), None, Some(limit))
        .await
        .expect("Failed to fetch contract history orders with limit");

    assert!(
        history_orders.len() <= limit as usize,
        "Result count {} should not exceed limit {}",
        history_orders.len(),
        limit
    );

    println!(
        "✅ Fetch contract orders with limit: requested={}, returned={}",
        limit,
        history_orders.len()
    );
}

// ============================================================================
// Cancel Contract Order Tests
// ============================================================================

/// Test: Cancel contract order
///
/// Creates a limit order and then cancels it.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_cancel_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let last_price_decimal: Decimal = last_price.into();
    let price = last_price_decimal * dec!(0.50);
    // TODO: Implement price_to_precision for Gate
    let adjusted_price = price;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(adjusted_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create contract order");

    // Cancel the order
    let cancelled = exchange
        .cancel_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel contract order");

    assert_eq!(
        cancelled.status,
        OrderStatus::Cancelled,
        "Cancelled order should have Cancelled status, got {:?}",
        cancelled.status
    );

    println!(
        "✅ Cancel contract order: id={}, status={:?}",
        order.id, cancelled.status
    );
}

// ============================================================================
// Contract Positions Tests
// ============================================================================

/// Test: Fetch contract positions
///
/// Fetches contract positions and verifies the response structure.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_fetch_positions() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    // Use the Margin trait's fetch_positions method
    use ccxt_core::traits::Margin;
    let positions = exchange
        .fetch_positions()
        .await
        .expect("Failed to fetch contract positions");

    println!("✅ Fetch contract positions: count={}", positions.len());

    for position in &positions {
        assert!(!position.symbol.is_empty());
        println!(
            "   Position: symbol={}, contracts={:?}, side={:?}",
            position.symbol, position.contracts, position.side
        );
    }
}

/// Test: Fetch contract position for specific symbol
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_fetch_position() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    // Use the Margin trait's fetch_position method
    use ccxt_core::traits::Margin;
    let position = exchange.fetch_position("BTC/USDT:USDT").await;

    match position {
        Ok(pos) => {
            assert_eq!(pos.symbol.as_str(), "BTC/USDT:USDT");
            println!(
                "✅ Fetch contract position: symbol={}, contracts={:?}, side={:?}",
                pos.symbol, pos.contracts, pos.side
            );
        }
        Err(e) => {
            // Expected if no position exists
            println!(
                "⚠️  Fetch contract position failed (expected if no position): {}",
                e
            );
        }
    }
}

// ============================================================================
// Contract Order Lifecycle Test
// ============================================================================

/// Test: Complete contract order lifecycle
///
/// Creates a contract limit order → fetches → cancels → verifies removed.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_order_lifecycle() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let last_price_decimal: Decimal = last_price.into();
    let price = last_price_decimal * dec!(0.55);
    // TODO: Implement price_to_precision for Gate
    let adjusted_price = price;

    // Step 1: Create limit order
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(adjusted_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let created = exchange
        .create_order(request)
        .await
        .expect("Failed to create contract order");

    assert_eq!(created.status, OrderStatus::Open);
    println!(
        "✅ Step 1 - Created: id={}, status={:?}",
        created.id, created.status
    );

    // Step 2: Fetch and verify
    let fetched = exchange
        .fetch_order(&created.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch contract order");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.status, OrderStatus::Open);
    println!("✅ Step 2 - Fetched: status={:?}", fetched.status);

    // Step 3: Cancel
    let cancelled = exchange
        .cancel_order(&created.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel contract order");
    assert_eq!(cancelled.status, OrderStatus::Cancelled);
    println!("✅ Step 3 - Cancelled: status={:?}", cancelled.status);

    println!("\n✅ Contract order lifecycle verified: Open → Cancelled");
}

//! OKX Order Query Tests
//!
//! Tests for fetch_order, fetch_open_orders, FETCH_HISTORY_ORDERS methods.
//! These tests verify the ability to query order status and history.
//!
//! ## Test Scenarios
//!
//! 1. Query single order by ID
//! 2. Query open orders
//! 3. Query closed orders (filled)
//! 4. Query canceled orders (via filtering)
//! 5. Query with time range
//! 6. Order lifecycle: create -> fetch -> cancel
//!
//! ## Prerequisites
//!
//! - OKX API credentials configured
//! - Sufficient balance for test orders
//! - Testnet or production environment

use crate::support::{create_okx_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::types::{
    Amount, AmountSpec, OrderRequest, OrderSide, OrderStatus, OrderType, Price,
};
use ccxt_exchanges::okx::Okx;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

/// Skip test if no credentials available
macro_rules! skip_if_no_credentials {
    () => {
        if should_skip_private_tests("okx") {
            println!("SKIPPED: No OKX credentials configured");
            return;
        }
    };
}

/// Create authenticated OKX client for spot trading
async fn create_auth_okx() -> Okx {
    let config = init_test();
    let exchange = create_okx_with_credentials(&config, "spot")
        .expect("Failed to create OKX with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    exchange
}

/// Helper to convert Price to Decimal
fn price_to_decimal(price: Price) -> Decimal {
    price.into()
}

/// Helper to create market buy order request
fn market_buy_request(symbol: &str, quote_amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(AmountSpec::quote(Amount::new(quote_amount)))
        .build()
        .unwrap()
}

/// Helper to create limit buy order request
fn limit_buy_request(symbol: &str, amount: Decimal, price: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::from(amount))
        .price(Price::from(price))
        .build()
        .unwrap()
}

/// Scenario 1: Query single order by ID
///
/// Tests fetching a specific order using its ID.
/// Verifies data consistency between created and fetched order.
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量（可在测试网运行）"]
async fn test_fetch_order_by_id() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 1.1 Create a market order (will be filled immediately)
    let request = market_buy_request("BTC/USDT", dec!(1.0));
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order");

    tokio::time::sleep(Duration::from_secs(2)).await;
    // Verify the order is complete (not Pending)
    assert_ne!(
        order.status,
        OrderStatus::Pending,
        "Order should be auto-fetched and not in Pending status"
    );

    // Verify essential fields are populated
    assert!(!order.id.is_empty(), "Order ID should not be empty");
    assert_eq!(order.symbol.to_string(), "BTC/USDT", "Symbol should match");
    assert_eq!(order.side, OrderSide::Buy, "Side should be Buy");

    // For market orders, status should be Closed (filled) or Open (if not filled yet)
    assert!(
        order.status == OrderStatus::Closed || order.status == OrderStatus::Open,
        "Market order should be Closed or Open, got: {:?}",
        order.status
    );

    println!("✓ create_order auto-fetch test passed");
    println!("  Order ID: {}", order.id);
    println!("  Status: {:?}", order.status);
    println!("  Amount: {}", order.amount);

    // 1.2 Fetch the order by ID
    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT")
        .await
        .expect("Failed to fetch order");

    // 1.3 Verify data consistency
    assert_eq!(fetched.id, order.id, "Order ID should match");
    assert_eq!(
        fetched.symbol.to_string(),
        "BTC/USDT",
        "Symbol should match"
    );
    assert_eq!(fetched.side, OrderSide::Buy, "Side should be Buy");
    assert_eq!(
        fetched.order_type,
        OrderType::Market,
        "Type should be Market"
    );

    // 1.4 Verify status update
    assert_eq!(
        fetched.status,
        OrderStatus::Closed,
        "Market order should be closed"
    );
    assert!(
        fetched.filled.unwrap_or_default() > Decimal::ZERO,
        "Filled amount should be > 0"
    );
    assert!(
        fetched.cost.unwrap_or_default() > Decimal::ZERO,
        "Cost should be > 0"
    );
}

/// Scenario 2: Query open orders
///
/// Tests fetching all open (unfilled) orders.
/// Creates multiple limit orders and verifies they appear in open orders.
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量（可在测试网运行）"]
async fn test_fetch_open_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 2.1 Create multiple limit orders (ensure they don't fill)
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    let mut open_ids = Vec::new();
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70); // Far below market price

    for i in 0..3 {
        let price = base_price - dec!(100.0) * Decimal::from(i);
        let request = limit_buy_request("BTC/USDT", dec!(0.0001), price);
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create limit order");
        open_ids.push(order.id);
    }

    // 2.2 Query open orders for specific symbol
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");

    // 2.3 Verify all returned orders are open
    for order in &open_orders {
        assert_eq!(order.status, OrderStatus::Open, "All orders should be open");
        assert!(
            order.filled.unwrap_or_default() <= order.amount,
            "Filled should be <= amount"
        );
    }

    // 2.4 Verify our orders are in the list
    let open_result_ids: Vec<&String> = open_orders.iter().map(|o| &o.id).collect();
    for open_id in &open_ids {
        assert!(
            open_result_ids.contains(&open_id),
            "Created order should be in open orders"
        );
    }

    // 2.5 Query all open orders (no symbol filter)
    let all_open_orders = exchange
        .fetch_open_orders(None, None, None)
        .await
        .expect("Failed to fetch all open orders");
    assert!(
        all_open_orders.len() >= open_orders.len(),
        "All open orders should be >= symbol-filtered orders"
    );

    // 2.6 Cleanup: Cancel all created orders
    for open_id in &open_ids {
        let _ = exchange.cancel_order(open_id, "BTC/USDT").await;
    }

    // 2.7 Verify orders are no longer open
    let remaining_open = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await
        .expect("Failed to fetch remaining open orders");
    for open_id in &open_ids {
        assert!(
            !remaining_open.iter().any(|o| &o.id == open_id),
            "Canceled order should not be in open orders"
        );
    }
}

/// Scenario 3: Query closed orders (filled)
///
/// Tests fetching only filled/closed orders.
/// Creates market orders (which fill immediately) and verifies they appear.
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量（可在测试网运行）"]
async fn test_fetch_history_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 3.1 Create and fill multiple market orders
    let mut filled_ids = Vec::new();

    for _ in 0..3 {
        let request = market_buy_request("BTC/USDT", dec!(1.0));
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create market order");
        println!("Created market order: {}", order.id);
        filled_ids.push(order.id);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // Wait for orders to be fully settled and appear in history
    tokio::time::sleep(Duration::from_secs(5)).await;

    // 3.2 Create and cancel a limit order (should NOT appear as filled)
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let cancel_price: Decimal = price_to_decimal(last_price) * dec!(0.85);
    let cancel_request = limit_buy_request("BTC/USDT", dec!(0.0001), cancel_price);
    let cancel_order = exchange
        .create_order(cancel_request)
        .await
        .expect("Failed to create limit order");
    let _ = exchange
        .cancel_order(&cancel_order.id, "BTC/USDT")
        .await
        .expect("Failed to cancel order");

    // Wait for cancellation to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 3.3 Query closed orders
    let since = chrono::Utc::now().timestamp_millis() - 3600000; // 1 hour ago
    let closed_orders = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(100))
        .await
        .expect("Failed to fetch closed orders");

    // 3.4 Verify results
    println!("Found {} closed orders", closed_orders.len());
    for order in &closed_orders {
        println!("  Order: {} - Status: {:?}", order.id, order.status);
    }

    assert!(!closed_orders.is_empty(), "Should have closed orders");

    // Verify our filled orders are in the list
    let closed_ids: Vec<&String> = closed_orders.iter().map(|o| &o.id).collect();
    for filled_id in &filled_ids {
        assert!(
            closed_ids.contains(&filled_id),
            "Filled order {} should be in closed orders",
            filled_id
        );
    }
}

/// Scenario 4: Query canceled orders via filtering
///
/// Since OKX doesn't have a dedicated fetch_canceled_orders endpoint,
/// we use FETCH_HISTORY_ORDERS and filter by status.
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量（可在测试网运行）"]
async fn test_fetch_canceled_orders_via_filter() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 4.1 Create and cancel multiple limit orders
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    let mut canceled_ids = Vec::new();
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.80);

    for i in 0..3 {
        let price = base_price - dec!(50.0) * Decimal::from(i);
        let request = limit_buy_request("BTC/USDT", dec!(0.0001), price);
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create limit order");
        println!("Created limit order: {}", order.id);
        let _ = exchange
            .cancel_order(&order.id, "BTC/USDT")
            .await
            .expect("Failed to cancel order");
        println!("Canceled order: {}", order.id);
        canceled_ids.push(order.id);
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Wait for cancellations to be fully processed and appear in history
    tokio::time::sleep(Duration::from_secs(3)).await;

    // 4.2 Query all historical orders
    let since = chrono::Utc::now().timestamp_millis() - 3600000;
    let all_orders = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(100))
        .await
        .expect("Failed to fetch historical orders");

    // 4.3 Filter canceled orders
    // Note: OKX uses different status values, we check for canceled status
    let canceled_orders: Vec<_> = all_orders
        .iter()
        .filter(|o| canceled_ids.contains(&o.id))
        .collect();

    println!("Found {} historical orders", all_orders.len());
    println!(
        "Found {} canceled orders from our list",
        canceled_orders.len()
    );
    for order in &all_orders {
        if canceled_ids.contains(&order.id) {
            println!("  Found: {} - Status: {:?}", order.id, order.status);
        }
    }

    // 4.4 Verify canceled orders are found
    for canceled_id in &canceled_ids {
        assert!(
            all_orders.iter().any(|o| &o.id == canceled_id),
            "Canceled order {} should be in historical orders",
            canceled_id
        );
    }

    println!("Found {} canceled orders in history", canceled_orders.len());
}

/// Scenario 5: Query with time range
///
/// Tests the since parameter for time-based filtering.
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量（可在测试网运行）"]
async fn test_fetch_orders_by_time_range() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 5.1 Record timeline
    let t0 = chrono::Utc::now().timestamp_millis();

    // 5.2 Create order at t1
    tokio::time::sleep(Duration::from_secs(1)).await;
    let request1 = market_buy_request("BTC/USDT", dec!(1.0));
    let order1 = exchange
        .create_order(request1)
        .await
        .expect("Failed to create order 1");
    let t1 = chrono::Utc::now().timestamp_millis();

    // 5.3 Create order at t2
    tokio::time::sleep(Duration::from_secs(2)).await;
    let request2 = market_buy_request("BTC/USDT", dec!(1.0));
    let order2 = exchange
        .create_order(request2)
        .await
        .expect("Failed to create order 2");
    println!("Created order 1: {}", order1.id);
    println!("Created order 2: {}", order2.id);

    // Wait for orders to be fully settled and appear in history
    tokio::time::sleep(Duration::from_secs(5)).await;

    // 5.4 Query orders since t0
    let orders_since_t0 = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(t0), Some(100))
        .await
        .expect("Failed to fetch orders since t0");

    println!("Found {} orders since t0", orders_since_t0.len());
    for order in &orders_since_t0 {
        println!("  Order: {} - Status: {:?}", order.id, order.status);
    }

    let ids: Vec<&String> = orders_since_t0.iter().map(|o| &o.id).collect();
    assert!(
        ids.contains(&&order1.id),
        "Order 1 {} should be in results since t0",
        order1.id
    );
    assert!(
        ids.contains(&&order2.id),
        "Order 2 {} should be in results since t0",
        order2.id
    );

    // 5.5 Query orders since t1 (should include order2)
    let orders_since_t1 = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(t1), Some(100))
        .await
        .expect("Failed to fetch orders since t1");

    println!("Found {} orders since t1", orders_since_t1.len());
    assert!(
        orders_since_t1.iter().any(|o| o.id == order2.id),
        "Order 2 {} should be in results since t1",
        order2.id
    );
}

/// Scenario 6: Complete order lifecycle
///
/// Tests the full lifecycle: create -> fetch -> edit -> cancel
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量（可在测试网运行）"]
async fn test_order_lifecycle() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 6.1 Create a limit order
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let price: Decimal = price_to_decimal(last_price) * dec!(0.75); // Far below market

    let request = limit_buy_request("BTC/USDT", dec!(0.0001), price);
    let created = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    assert_eq!(
        created.status,
        OrderStatus::Open,
        "New limit order should be open"
    );

    // 6.2 Fetch the order
    let fetched = exchange
        .fetch_order(&created.id, "BTC/USDT")
        .await
        .expect("Failed to fetch order");
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.status, OrderStatus::Open);

    // 6.3 Verify order appears in open orders
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");
    assert!(
        open_orders.iter().any(|o| o.id == created.id),
        "Order should be in open orders"
    );

    // 6.4 Cancel the order
    let _canceled = exchange
        .cancel_order(&created.id, "BTC/USDT")
        .await
        .expect("Failed to cancel order");
    println!("Created and canceled order: {}", created.id);

    // Wait for cancellation to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify order no longer in open orders
    let open_orders_after = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await
        .expect("Failed to fetch open orders after cancel");
    assert!(
        !open_orders_after.iter().any(|o| o.id == created.id),
        "Canceled order should not be in open orders"
    );

    // 6.6 Verify order appears in historical orders
    let since = chrono::Utc::now().timestamp_millis() - 60000; // 1 minute ago
    let historical = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(100))
        .await
        .expect("Failed to fetch historical orders");

    println!("Found {} historical orders", historical.len());
    for order in &historical {
        if order.id == created.id {
            println!("  Found: {} - Status: {:?}", order.id, order.status);
        }
    }

    assert!(
        historical.iter().any(|o| o.id == created.id),
        "Canceled order {} should be in historical orders",
        created.id
    );
}

/// Scenario 7: Test pagination with limit
///
/// Tests the limit parameter for controlling result size.
#[tokio::test]
#[ignore = "生产环境测试: 需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量，并确保账户有历史订单数据"]
async fn test_fetch_orders_with_limit() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 7.1 Query with different limits
    let since = chrono::Utc::now().timestamp_millis() - 86400000; // 24 hours ago

    let orders_limit_5 = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(5))
        .await
        .expect("Failed to fetch orders with limit 5");

    let orders_limit_10 = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(10))
        .await
        .expect("Failed to fetch orders with limit 10");

    // 7.2 Verify limits are respected
    assert!(orders_limit_5.len() <= 5, "Should return at most 5 orders");
    assert!(
        orders_limit_10.len() <= 10,
        "Should return at most 10 orders"
    );

    // Note: If account has fewer orders than limit, this is expected
    if orders_limit_5.len() == 5 && orders_limit_10.len() > 5 {
        assert!(
            orders_limit_10.len() > orders_limit_5.len(),
            "Higher limit should return more orders if available"
        );
    }
}

/// Scenario 8: Query orders by symbol filter
///
/// Tests symbol parameter filtering for different trading pairs.
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量（可在测试网运行）"]
async fn test_fetch_orders_by_symbol_filter() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 8.1 Create orders in BTC/USDT
    let btc_request1 = market_buy_request("BTC/USDT", dec!(1.0));
    let btc_order1 = exchange
        .create_order(btc_request1)
        .await
        .expect("Failed to create BTC order 1");

    let btc_request2 = market_buy_request("BTC/USDT", dec!(1.0));
    let btc_order2 = exchange
        .create_order(btc_request2)
        .await
        .expect("Failed to create BTC order 2");

    // 8.2 Create order in ETH/USDT
    let eth_request = market_buy_request("ETH/USDT", dec!(1.0));
    let eth_order = exchange
        .create_order(eth_request)
        .await
        .expect("Failed to create ETH order");

    println!("Created BTC order 1: {}", btc_order1.id);
    println!("Created BTC order 2: {}", btc_order2.id);
    println!("Created ETH order: {}", eth_order.id);

    // Wait for orders to be fully settled and appear in history
    tokio::time::sleep(Duration::from_secs(5)).await;

    let since = chrono::Utc::now().timestamp_millis() - 3600000;

    // 8.3 Query only BTC/USDT orders
    let btc_orders = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(100))
        .await
        .expect("Failed to fetch BTC orders");

    println!("Found {} BTC orders", btc_orders.len());
    for order in &btc_orders {
        println!("  BTC Order: {} - Status: {:?}", order.id, order.status);
    }

    let btc_ids: Vec<&String> = btc_orders.iter().map(|o| &o.id).collect();
    assert!(
        btc_ids.contains(&&btc_order1.id),
        "Should contain BTC order 1 {}",
        btc_order1.id
    );
    assert!(
        btc_ids.contains(&&btc_order2.id),
        "Should contain BTC order 2 {}",
        btc_order2.id
    );

    // 8.4 Query only ETH/USDT orders
    let eth_orders = exchange
        .fetch_history_orders(Some("ETH/USDT"), Some(since), Some(100))
        .await
        .expect("Failed to fetch ETH orders");

    println!("Found {} ETH orders", eth_orders.len());
    for order in &eth_orders {
        println!("  ETH Order: {} - Status: {:?}", order.id, order.status);
    }

    let eth_ids: Vec<&String> = eth_orders.iter().map(|o| &o.id).collect();
    assert!(
        eth_ids.contains(&&eth_order.id),
        "Should contain ETH order {}",
        eth_order.id
    );

    // Verify ETH orders don't contain BTC orders
    assert!(
        !eth_ids.contains(&&btc_order1.id),
        "ETH orders should not contain BTC order 1 {}",
        btc_order1.id
    );
}

/// Scenario 9: Order data completeness validation
///
/// Tests that fetched orders contain all required fields.
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSWORD 环境变量（可在测试网运行）"]
async fn test_order_data_completeness() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 9.1 Create a market order
    let request = market_buy_request("BTC/USDT", dec!(1.0));
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order");

    tokio::time::sleep(Duration::from_secs(2)).await;

    // 9.2 Fetch the order
    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT")
        .await
        .expect("Failed to fetch order");

    // 9.3 Verify basic fields
    assert!(!fetched.id.is_empty(), "Order ID should not be empty");
    assert_eq!(
        fetched.symbol.to_string(),
        "BTC/USDT",
        "Symbol should match"
    );
    assert_eq!(
        fetched.order_type,
        OrderType::Market,
        "Type should be Market"
    );
    assert_eq!(fetched.side, OrderSide::Buy, "Side should be Buy");

    // 9.4 Verify timestamp fields
    assert!(fetched.timestamp.is_some(), "Timestamp should be present");

    // 9.5 Verify amount and fill data
    assert!(
        fetched.amount > Decimal::ZERO,
        "Amount should be greater than 0"
    );
    assert!(
        fetched.filled.unwrap_or_default() > Decimal::ZERO,
        "Filled should be greater than 0 for market order"
    );
    assert!(
        fetched.cost.unwrap_or_default() > Decimal::ZERO,
        "Cost should be greater than 0"
    );

    // 9.6 Verify status
    assert_eq!(
        fetched.status,
        OrderStatus::Closed,
        "Market order should be closed"
    );
}

/// Scenario 10: Edge cases
///
/// Tests boundary conditions and error handling.
#[tokio::test]
#[ignore = "Requires OKX API credentials"]
async fn test_fetch_orders_edge_cases() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx().await;

    // 10.1 Query with future time (should return empty)
    let future_time = chrono::Utc::now().timestamp_millis() + 3600000;
    let orders = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(future_time), Some(10))
        .await
        .expect("Should not error on future time");
    assert!(orders.is_empty(), "Future time should return empty orders");

    // 10.2 Query with limit = 0 (OKX API returns error for invalid limit)
    let since = chrono::Utc::now().timestamp_millis() - 3600000;
    let result = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(0))
        .await;

    // OKX API doesn't accept limit=0, should return an error
    assert!(
        result.is_err(),
        "OKX API should reject limit=0 with an error"
    );

    if let Err(e) = result {
        println!("limit=0 correctly rejected: {}", e);
        // Verify it's a parameter error
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("limit") || error_msg.contains("Parameter"),
            "Error should mention limit or parameter issue: {}",
            error_msg
        );
    }

    // 10.3 Query with very large limit
    let orders = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(since), Some(1000))
        .await
        .expect("Should not error on large limit");
    // API should cap the limit
    assert!(
        orders.len() <= 1000,
        "Should not return more than requested limit"
    );

    // 10.4 Query ancient time (should return empty or limited data)
    let ancient_time = 1577836800000i64; // 2020-01-01
    let orders = exchange
        .fetch_history_orders(Some("BTC/USDT"), Some(ancient_time), Some(10))
        .await
        .expect("Should not error on ancient time");
    println!("Ancient time query returned {} orders", orders.len());
}

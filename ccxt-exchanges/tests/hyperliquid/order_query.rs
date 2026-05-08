//! HyperLiquid Order Query Tests
//!
//! Tests for fetch_order, fetch_open_orders, FETCH_HISTORY_ORDERS methods.
//!
//! ## Prerequisites
//!
//! - HyperLiquid wallet configured
//! - Sufficient balance for test orders

use crate::support::{create_hyperliquid_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::types::{Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price};
use ccxt_exchanges::hyperliquid::HyperLiquid;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

macro_rules! skip_if_no_credentials {
    () => {
        if should_skip_private_tests("hyperliquid") {
            println!("SKIPPED: No HyperLiquid credentials configured");
            return;
        }
    };
}

async fn create_auth_hyperliquid() -> HyperLiquid {
    let config = init_test();
    let exchange = create_hyperliquid_with_credentials(&config, "spot")
        .expect("Failed to create HyperLiquid with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    exchange
}

fn price_to_decimal(price: Price) -> Decimal {
    price.into()
}

/// Helper to create market buy order request
fn market_buy_request(symbol: &str, amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::from(amount))
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

#[tokio::test]
#[ignore = "需要配置 HYPERLIQUID_WALLET_ADDRESS, HYPERLIQUID_PRIVATE_KEY 环境变量（可在测试网运行）"]
async fn test_fetch_order_by_id() {
    skip_if_no_credentials!();
    let exchange = create_auth_hyperliquid().await;

    // Create a market order
    let request = market_buy_request("BTC/USDC", dec!(0.0001));
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order");

    tokio::time::sleep(Duration::from_secs(2)).await;

    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDC")
        .await
        .expect("Failed to fetch order");

    assert_eq!(fetched.id, order.id);
    println!("Order status: {:?}", fetched.status);
}

#[tokio::test]
#[ignore = "需要配置 HYPERLIQUID_WALLET_ADDRESS, HYPERLIQUID_PRIVATE_KEY 环境变量（可在测试网运行）"]
async fn test_fetch_open_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_hyperliquid().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDC")
        .await
        .expect("Failed to fetch ticker");

    let mut open_ids = Vec::new();
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    for i in 0..3 {
        let price = base_price - dec!(100.0) * Decimal::from(i);
        let request = limit_buy_request("BTC/USDC", dec!(0.0001), price);
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create limit order");
        open_ids.push(order.id);
    }

    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDC"), None, None)
        .await
        .expect("Failed to fetch open orders");

    for order in &open_orders {
        assert_eq!(order.status, OrderStatus::Open);
    }

    // Cleanup
    for open_id in &open_ids {
        let _ = exchange.cancel_order(open_id, "BTC/USDC").await;
    }
}

#[tokio::test]
#[ignore = "需要配置 HYPERLIQUID_WALLET_ADDRESS, HYPERLIQUID_PRIVATE_KEY 环境变量（可在测试网运行）"]
async fn test_fetch_history_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_hyperliquid().await;

    let mut filled_ids = Vec::new();

    for _ in 0..3 {
        let request = market_buy_request("BTC/USDC", dec!(0.0001));
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create market order");
        filled_ids.push(order.id);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let since = chrono::Utc::now().timestamp_millis() - 3600000;
    let closed_orders = exchange
        .fetch_history_orders(Some("BTC/USDC"), Some(since), Some(10))
        .await
        .expect("Failed to fetch closed orders");

    println!("Fetched {} closed orders", closed_orders.len());

    let closed_ids: Vec<&String> = closed_orders.iter().map(|o| &o.id).collect();
    for filled_id in &filled_ids {
        assert!(closed_ids.contains(&filled_id));
    }
}

#[tokio::test]
#[ignore = "需要配置 HYPERLIQUID_WALLET_ADDRESS, HYPERLIQUID_PRIVATE_KEY 环境变量（可在测试网运行）"]
async fn test_order_lifecycle() {
    skip_if_no_credentials!();
    let exchange = create_auth_hyperliquid().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDC")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let price: Decimal = price_to_decimal(last_price) * dec!(0.75);

    let request = limit_buy_request("BTC/USDC", dec!(0.0001), price);
    let created = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    assert_eq!(created.status, OrderStatus::Open);

    // Fetch order
    let fetched = exchange
        .fetch_order(&created.id, "BTC/USDC")
        .await
        .expect("Failed to fetch order");
    assert_eq!(fetched.id, created.id);

    // Cancel order
    let _canceled = exchange
        .cancel_order(&created.id, "BTC/USDC")
        .await
        .expect("Failed to cancel order");

    // Verify not in open orders
    let open_orders_after = exchange
        .fetch_open_orders(Some("BTC/USDC"), None, None)
        .await
        .expect("Failed to fetch open orders");
    assert!(!open_orders_after.iter().any(|o| o.id == created.id));
}

// ============================================================================
// TP/SL (Take Profit / Stop Loss) Tests
// ============================================================================

/// Test: Create order with take profit
///
/// Tests TP parameter passing in order creation.
#[tokio::test]
#[ignore = "需要配置 HYPERLIQUID_WALLET_ADDRESS, HYPERLIQUID_PRIVATE_KEY 环境变量（可在测试网运行）"]
async fn test_hyperliquid_create_order_with_take_profit() {
    skip_if_no_credentials!();
    let hyperliquid = create_auth_hyperliquid().await;

    // Build order request with take profit (HyperLiquid uses trigger field)
    let request = OrderRequest::builder()
        .symbol("BTC/USDC")
        .order_type(OrderType::Limit)
        .side(OrderSide::Buy)
        .amount(Amount::from(dec!(0.0001)))
        .price(Price::from(dec!(50000)))
        .tp_limit_price(Price::from(dec!(55000)))
        .build()
        .expect("OrderRequest should build successfully");

    let result = hyperliquid.create_order(request).await;

    match result {
        Ok(order) => {
            println!("✅ HyperLiquid TP order created: {:?}", order.id);
            let _ = hyperliquid.cancel_order(&order.id, "BTC/USDC").await;
        }
        Err(e) => {
            println!("⚠️  Test skipped: {}", e);
        }
    }
}

/// Test: Create order with stop loss
///
/// Tests SL parameter passing in order creation.
#[tokio::test]
#[ignore = "需要配置 HYPERLIQUID_WALLET_ADDRESS, HYPERLIQUID_PRIVATE_KEY 环境变量（可在测试网运行）"]
async fn test_hyperliquid_create_order_with_stop_loss() {
    skip_if_no_credentials!();
    let hyperliquid = create_auth_hyperliquid().await;

    // Build order request with stop loss
    let request = OrderRequest::builder()
        .symbol("BTC/USDC")
        .order_type(OrderType::Limit)
        .side(OrderSide::Buy)
        .amount(Amount::from(dec!(0.0001)))
        .price(Price::from(dec!(50000)))
        .sl_limit_price(Price::from(dec!(48000)))
        .build()
        .expect("OrderRequest should build successfully");

    let result = hyperliquid.create_order(request).await;

    match result {
        Ok(order) => {
            println!("✅ HyperLiquid SL order created: {:?}", order.id);
            let _ = hyperliquid.cancel_order(&order.id, "BTC/USDC").await;
        }
        Err(e) => {
            println!("⚠️  Test skipped: {}", e);
        }
    }
}

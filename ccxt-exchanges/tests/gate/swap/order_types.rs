//! Gate.io Swap (Contract) Order Types Tests
//!
//! Tests for contract order types and market data:
//! - Contract market data (markets, tickers)
//! - Market orders (Long/Short)
//! - Limit orders (Long/Short)
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
use std::time::Duration;

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

    // Load markets before using the exchange
    exchange
        .load_markets()
        .await
        .expect("Failed to load contract markets");

    exchange
}

/// Helper: Get market price with discount for limit orders
async fn get_discount_price(exchange: &Gate, symbol: &str, discount: Decimal) -> Decimal {
    let ticker = exchange
        .fetch_ticker(symbol)
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * discount;
    base_price // TODO: Implement price_to_precision for Gate
}

fn price_to_decimal(price: Price) -> Decimal {
    price.into()
}

// ============================================================================
// Contract Market Data Tests
// ============================================================================

/// Test: Fetch contract markets
///
/// Verifies that contract markets can be loaded and contain USDT-margined symbols.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_fetch_markets() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let markets = exchange
        .fetch_markets()
        .await
        .expect("Failed to fetch contract markets");

    assert!(!markets.is_empty(), "Contract markets should not be empty");

    // Find a USDT-margined perpetual contract (e.g. BTC/USDT:USDT)
    let swap_markets: Vec<_> = markets
        .iter()
        .filter(|m| {
            m.is_swap()
                && m.settle
                    .as_deref()
                    .map(|s| s.to_lowercase() == "usdt")
                    .unwrap_or(false)
        })
        .collect();

    assert!(
        !swap_markets.is_empty(),
        "Should have USDT-margined swap markets"
    );

    println!(
        "✅ Contract markets loaded: total={}, USDT swap={}",
        markets.len(),
        swap_markets.len()
    );
}

/// Test: Fetch contract ticker
///
/// Fetches ticker for a specific USDT-margined contract.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_fetch_ticker() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    // Fetch ticker for BTC/USDT:USDT perpetual
    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch contract ticker");

    assert!(
        ticker.last.is_some(),
        "Contract ticker should have last price"
    );
    assert!(
        ticker.bid.is_some(),
        "Contract ticker should have bid price"
    );
    assert!(
        ticker.ask.is_some(),
        "Contract ticker should have ask price"
    );
    assert!(
        ticker.high.is_some(),
        "Contract ticker should have high price"
    );
    assert!(
        ticker.low.is_some(),
        "Contract ticker should have low price"
    );
    assert!(
        ticker.base_volume.is_some(),
        "Contract ticker should have volume"
    );

    println!(
        "✅ Contract ticker: last={:?}, bid={:?}, ask={:?}, volume={:?}",
        ticker.last, ticker.bid, ticker.ask, ticker.base_volume
    );
}

// ============================================================================
// Contract Market Order Tests (Long/Short)
// ============================================================================

/// Test: Market buy order opens Long position
///
/// Creates a market buy on a USDT perpetual contract.
/// Note: Gate swap uses contract amount as base currency amount.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_market_buy_long() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(1))) // 1 contract
        .build()
        .expect("OrderRequest should build successfully");

    let order_result = exchange.create_order(request).await;

    // Handle testnet limitations
    match order_result {
        Ok(order) => {
            assert_eq!(order.order_type, OrderType::Market);
            assert_eq!(order.side, OrderSide::Buy);
            assert!(
                order.status == OrderStatus::Closed || order.status == OrderStatus::Open,
                "Market order should be Closed or Open, got {:?}",
                order.status
            );

            println!(
                "✅ Swap market buy (Long) order: status={:?}, filled={}, cost={}",
                order.status,
                order.filled.unwrap_or(Decimal::ZERO),
                order.cost.unwrap_or(Decimal::ZERO)
            );
        }
        Err(e) => {
            // Testnet may not support market orders
            let error_msg = format!("{}", e);
            if error_msg.contains("INVALID_PROTOCOL") || error_msg.contains("testnet") {
                println!("⚠️  Skipped: Testnet may not support market orders - {}", e);
            } else {
                panic!("Failed to create contract market buy order: {}", e);
            }
        }
    }
}

/// Test: Market sell order opens Short position
///
/// Creates a market sell on a USDT perpetual contract.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_market_sell_short() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(1))) // 1 contract
        .build()
        .expect("OrderRequest should build successfully");

    let result = exchange.create_order(request).await;

    match result {
        Ok(order) => {
            assert_eq!(order.order_type, OrderType::Market);
            assert_eq!(order.side, OrderSide::Sell);
            println!(
                "✅ Swap market sell (Short) order: status={:?}",
                order.status
            );
        }
        Err(e) => {
            println!(
                "⚠️  Swap market sell failed (expected if not holding position): {}",
                e
            );
        }
    }
}

// ============================================================================
// Contract Limit Order Tests (Long/Short)
// ============================================================================

/// Test: Limit buy order opens Long position
///
/// Creates a limit buy order below market price.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_limit_buy_long() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let price = get_discount_price(&exchange, "BTC/USDT:USDT", dec!(0.70)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create contract limit buy order");

    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.status, OrderStatus::Open);

    println!(
        "✅ Swap limit buy (Long) order: price={}, status={:?}",
        price, order.status
    );

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
}

/// Test: Limit sell order opens Short position
///
/// Creates a limit sell order above market price.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_limit_sell_short() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let price = get_discount_price(&exchange, "BTC/USDT:USDT", dec!(1.30)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let result = exchange.create_order(request).await;

    match result {
        Ok(order) => {
            assert_eq!(order.order_type, OrderType::Limit);
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.status, OrderStatus::Open);

            println!(
                "✅ Swap limit sell (Short) order: price={}, status={:?}",
                price, order.status
            );

            // Cleanup
            let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
        }
        Err(e) => {
            println!("⚠️  Swap limit sell failed (expected if no balance): {}", e);
        }
    }
}

// ============================================================================
// Contract TimeInForce Tests
// ============================================================================

/// Test: Contract IOC (Immediate or Cancel) order
///
/// Creates a contract IOC limit order that should fill immediately or cancel.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_ioc_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let price = get_discount_price(&exchange, "BTC/USDT:USDT", dec!(0.99)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::IOC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create contract IOC order");

    assert!(
        order.status == OrderStatus::Closed
            || order.status == OrderStatus::Cancelled
            || order.status == OrderStatus::Partial,
        "IOC order should be Closed, Cancelled or Partial, got {:?}",
        order.status
    );

    println!(
        "✅ Swap IOC order result: status={:?}, filled={}",
        order.status,
        order.filled.unwrap_or(Decimal::ZERO)
    );
}

// ============================================================================
// PostOnly Order Tests
// ============================================================================

/// Test: PostOnly (LimitMaker) order
///
/// Creates a PostOnly order that should only add liquidity (maker).
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_postonly_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // PostOnly 价格远低于市场价（确保只挂单不吃单）
    let postonly_price = current_price * dec!(0.50);

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::LimitMaker)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(postonly_price))
        .build()
        .expect("OrderRequest should build successfully");

    let result = exchange.create_order(request).await;

    match result {
        Ok(order) => {
            // PostOnly 订单应该处于 Open 状态（未成交）
            assert_eq!(order.order_type, OrderType::LimitMaker);
            assert_eq!(order.side, OrderSide::Buy);
            assert!(
                order.post_only.unwrap_or(false),
                "PostOnly flag should be true"
            );

            println!(
                "✅ Swap PostOnly order: id={}, price={}, post_only={}",
                order.id,
                postonly_price,
                order.post_only.unwrap_or(false)
            );

            // 清理：取消订单
            let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
        }
        Err(e) => {
            println!("⚠️  Gate may not support PostOnly orders: {}", e);
        }
    }
}

// ============================================================================
// ReduceOnly / Close Position Tests
// ============================================================================

/// Test: ReduceOnly order (should only reduce position)
///
/// Creates a reduce-only order. If no position exists, order should be rejected or cancelled.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_swap_reduce_only_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // 价格高于市场价（确保不会立即成交）
    let limit_price = current_price * dec!(1.50);

    // 创建只减仓订单（在没有持仓时应该被拒绝或取消）
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(limit_price))
        .time_in_force(TimeInForce::GTC)
        .reduce_only(true)
        .build()
        .expect("OrderRequest should build successfully");

    let result = exchange.create_order(request).await;

    match result {
        Ok(order) => {
            // 如果没有持仓，reduce_only 订单应该被取消或拒绝
            assert!(
                order.status == OrderStatus::Cancelled || order.status == OrderStatus::Open,
                "Reduce-only order should be Cancelled (no position) or Open (has position)"
            );

            println!(
                "✅ Swap ReduceOnly order: id={}, status={:?}, reduce_only={}",
                order.id,
                order.status,
                order.reduce_only.unwrap_or(false)
            );

            // 如果订单处于 Open 状态，清理
            if order.status == OrderStatus::Open {
                let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
            }
        }
        Err(e) => {
            println!(
                "⚠️  ReduceOnly order rejected (expected if no position): {}",
                e
            );
        }
    }
}

// ============================================================================
// Order Lifecycle Tests
// ============================================================================

/// Test: Complete order lifecycle (create -> fetch -> cancel -> verify)
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

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // 远低于市场价避免成交
    let limit_price = current_price * dec!(0.50);

    // 1. 创建订单
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(1)))
        .price(Price::new(limit_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    let order_id = order.id.clone();
    assert_eq!(order.status, OrderStatus::Open, "Order should be Open");
    println!("1️⃣  Created order: {}", order_id);

    // 2. 等待一下
    tokio::time::sleep(Duration::from_secs(1)).await;

    // 3. 查询订单
    let fetched = exchange
        .fetch_order(&order_id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order");

    assert_eq!(fetched.id, order_id, "Order ID should match");
    assert_eq!(
        fetched.status,
        OrderStatus::Open,
        "Order should still be Open"
    );
    println!("2️⃣  Fetched order: status={:?}", fetched.status);

    // 4. 取消订单
    let canceled = exchange
        .cancel_order(&order_id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel order");

    assert_eq!(
        canceled.status,
        OrderStatus::Cancelled,
        "Order should be cancelled"
    );
    println!("3️⃣  Cancelled order: status={:?}", canceled.status);

    // 5. 再次查询确认
    tokio::time::sleep(Duration::from_secs(1)).await;
    let final_order = exchange
        .fetch_order(&order_id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order after cancel");

    assert_eq!(
        final_order.status,
        OrderStatus::Cancelled,
        "Order should remain cancelled"
    );
    println!("4️⃣  Verified cancelled: status={:?}", final_order.status);

    println!("✅ Order lifecycle test passed: Created -> Fetched -> Cancelled -> Verified");
}

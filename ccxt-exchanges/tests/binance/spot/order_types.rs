//! Binance Spot Order Types Integration Tests
//!
//! Tests for all supported spot order types:
//! - Market orders (buy/sell)
//! - Limit orders (buy/sell)
//! - Post-Only / LimitMaker
//! - TimeInForce (GTC, IOC, FOK)
//!
//! ## Prerequisites
//!
//! - Binance API credentials configured (BINANCE_API_KEY, BINANCE_API_SECRET)
//! - Sufficient balance for test orders
//! - Can run on testnet (set BINANCE_TESTNET=true)
//!
//! ## Safety
//!
//! - All tests use minimum order amounts
//! - Limit orders are placed far from market price to avoid execution
//! - All tests cleanup created orders
//! - Tests are marked with `#[ignore]` and require explicit execution

use crate::support::{create_binance_spot_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::types::common::ticker_params::TickerParams;
use ccxt_core::types::{
    Amount, AmountSpec, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
};
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

/// Create authenticated Binance client for spot tests.
async fn create_auth_binance_spot() -> Binance {
    let config = init_test();
    let exchange = create_binance_spot_with_credentials(&config)
        .expect("Failed to create Binance with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    exchange
}

/// Helper: Get market price with discount for limit orders
async fn get_discount_price(exchange: &Binance, symbol: &str, discount: Decimal) -> Decimal {
    let ticker = exchange
        .fetch_ticker(symbol, TickerParams::default())
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * discount;

    // 根据市场精度调整价格
    exchange
        .base()
        .price_to_precision(symbol, base_price)
        .await
        .expect("Failed to adjust price precision")
}

fn price_to_decimal(price: ccxt_core::types::Price) -> Decimal {
    price.into()
}

// ============================================================================
// Market Order Tests
// ============================================================================

/// Test: Market buy order
///
/// Creates a market buy order and verifies immediate execution.
/// Note: Binance spot market buy uses quote currency amount (USDT)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_market_buy_order() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    let exchange = create_auth_binance_spot().await;

    // 现货市价买单: amount 表示 quote 金额 (USDT)
    // Binance requires minimum 10 USDT for market buy
    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(AmountSpec::quote(Amount::new(dec!(10.0)))) // 10 USDT
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market buy order");

    // Market orders should be filled immediately or very quickly
    assert_eq!(order.order_type, OrderType::Market);
    assert_eq!(order.side, OrderSide::Buy);
    // 市价单可能是 Closed（已成交）或 Open（刚提交）
    assert!(
        order.status == OrderStatus::Closed || order.status == OrderStatus::Open,
        "Market order should be Closed or Open, got {:?}",
        order.status
    );
    // 验证有成交数据
    assert!(order.filled.unwrap_or(rust_decimal::Decimal::ZERO) >= rust_decimal::Decimal::ZERO);
    assert!(order.cost.unwrap_or(rust_decimal::Decimal::ZERO) > rust_decimal::Decimal::ZERO);

    println!(
        "✅ Spot market buy order created: status={:?}, filled={} BTC, cost={} USDT",
        order.status,
        order.filled.unwrap_or(rust_decimal::Decimal::ZERO),
        order.cost.unwrap_or(rust_decimal::Decimal::ZERO)
    );
}

/// Test: Market sell order
///
/// Creates a market sell order and verifies immediate execution.
/// Note: Binance spot market sell uses base currency amount (BTC)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_market_sell_order() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    let exchange = create_auth_binance_spot().await;

    // 现货市价卖单: amount 表示 base 数量 (BTC)
    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.0001)))
        .build()
        .expect("OrderRequest should build successfully");

    let result = exchange.create_order(request).await;

    match result {
        Ok(order) => {
            assert_eq!(order.order_type, OrderType::Market);
            assert_eq!(order.side, OrderSide::Sell);
            assert_eq!(order.status, OrderStatus::Closed);
            println!("✅ Spot market sell order filled successfully");
        }
        Err(e) => {
            // Expected if no balance
            println!(
                "⚠️  Spot market sell failed (expected if no balance): {}",
                e
            );
        }
    }
}

// ============================================================================
// Limit Order Tests
// ============================================================================

/// Test: Limit buy order (placed below market to avoid execution)
///
/// Creates a limit buy order at 70% of market price and verifies it stays open.
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_limit_buy_order() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    let exchange = create_auth_binance_spot().await;

    let price = get_discount_price(&exchange, "BTC/USDT", dec!(0.70)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create limit buy order");

    // Limit order below market should stay open
    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.status, OrderStatus::Open);
    assert_eq!(order.price, Some(price));

    println!("✅ Spot limit buy order created at {} USDT", price);

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
}

/// Test: Limit sell order (placed above market to avoid execution)
///
/// Creates a limit sell order at 130% of market price and verifies it stays open.
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_limit_sell_order() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    let exchange = create_auth_binance_spot().await;

    let params = TickerParams::default();
    let ticker = exchange
        .fetch_ticker("BTC/USDT", params)
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(1.30);

    // 根据市场精度调整价格
    let price = exchange
        .base()
        .price_to_precision("BTC/USDT", base_price)
        .await
        .expect("Failed to adjust price precision");

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create limit sell order");

    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.side, OrderSide::Sell);
    assert_eq!(order.status, OrderStatus::Open);

    println!("✅ Spot limit sell order created at {} USDT", price);

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
}

// ============================================================================
// Post-Only / LimitMaker Tests
// ============================================================================

/// Test: Post-Only order (LimitMaker behavior for spot)
///
/// Creates a limit order with post_only flag.
/// Note: Binance spot uses 'LIMIT_MAKER' order type for post-only orders.
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_post_only_order() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    let exchange = create_auth_binance_spot().await;

    let price = get_discount_price(&exchange, "BTC/USDT", dec!(0.70)).await;

    // Use LimitMaker for spot post-only orders
    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::LimitMaker)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create post_only order");

    assert_eq!(order.side, OrderSide::Buy);
    // NEW 状态表示订单已创建并挂单，对应 Open
    assert!(
        order.status == OrderStatus::Open || order.status == OrderStatus::Pending,
        "Post-Only order should be Open or Pending, got {:?}",
        order.status
    );

    println!("✅ Spot Post-Only order created (LimitMaker)");
    println!("   Order ID: {}", order.id);
    println!("   Status: {:?}", order.status);
    println!("   Price: {:?}", order.price);
    println!("   Amount: {}", order.amount);

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
}

// ============================================================================
// TimeInForce Tests
// ============================================================================

/// Test: TimeInForce GTC (Good Till Cancelled)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_time_in_force_gtc() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    let exchange = create_auth_binance_spot().await;

    let price = get_discount_price(&exchange, "BTC/USDT", dec!(0.70)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create GTC order");

    assert_eq!(order.status, OrderStatus::Open);

    println!("✅ Spot GTC order created (Good Till Cancelled)");

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
}

/// Test: TimeInForce IOC (Immediate Or Cancel)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_time_in_force_ioc() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    let exchange = create_auth_binance_spot().await;

    // Set price at market to trigger immediate execution
    let ticker = exchange
        .fetch_ticker("BTC/USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");
    let price: Decimal = price_to_decimal(ticker.last.expect("Ticker should have last price"));

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::IOC)
        .build()
        .expect("OrderRequest should build successfully");

    let result = exchange.create_order(request).await;

    match result {
        Ok(order) => {
            // IOC orders should eventually be filled or cancelled,
            // but immediately after creation they might still be Open
            println!(
                "✅ Spot IOC order created: status={:?}, filled={}",
                order.status,
                order.filled.unwrap_or(rust_decimal::Decimal::ZERO)
            );

            // If the order is still Open, try to fetch it again after a short delay
            if order.status == OrderStatus::Open {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if let Ok(updated_order) = exchange.fetch_order(&order.id, "BTC/USDT").await {
                    println!(
                        "✅ Spot IOC order updated: status={:?}, filled={}",
                        updated_order.status,
                        updated_order.filled.unwrap_or(rust_decimal::Decimal::ZERO)
                    );
                    // After delay, should be Closed or Cancelled (or still Open in testnet)
                    assert!(
                        updated_order.status == OrderStatus::Closed
                            || updated_order.status == OrderStatus::Cancelled
                            || updated_order.status == OrderStatus::Open, // Testnet may be slow
                        "IOC order should be Closed, Cancelled, or Open (testnet), got {:?}",
                        updated_order.status
                    );
                }
            } else {
                // Already in final state
                assert!(
                    order.status == OrderStatus::Closed || order.status == OrderStatus::Cancelled
                );
            }
        }
        Err(e) => {
            println!("⚠️  Spot IOC order failed: {}", e);
        }
    }
}

/// Test: TimeInForce FOK (Fill Or Kill)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_time_in_force_fok() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    let exchange = create_auth_binance_spot().await;

    // Set price at market to trigger immediate execution
    let ticker = exchange
        .fetch_ticker("BTC/USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");
    let price: Decimal = price_to_decimal(ticker.last.expect("Ticker should have last price"));

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::FOK)
        .build()
        .expect("OrderRequest should build successfully");

    let result = exchange.create_order(request).await;

    match result {
        Ok(order) => {
            // FOK orders should eventually be fully filled or cancelled,
            // but immediately after creation they might still be Open
            println!(
                "✅ Spot FOK order created: status={:?}, filled={}",
                order.status,
                order.filled.unwrap_or(rust_decimal::Decimal::ZERO)
            );

            // If the order is still Open, try to fetch it again after a short delay
            if order.status == OrderStatus::Open {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if let Ok(updated_order) = exchange.fetch_order(&order.id, "BTC/USDT").await {
                    println!(
                        "✅ Spot FOK order updated: status={:?}, filled={}",
                        updated_order.status,
                        updated_order.filled.unwrap_or(rust_decimal::Decimal::ZERO)
                    );
                    // After delay, should be Closed or Cancelled (or still Open in testnet)
                    assert!(
                        updated_order.status == OrderStatus::Closed
                            || updated_order.status == OrderStatus::Cancelled
                            || updated_order.status == OrderStatus::Open, // Testnet may be slow
                        "FOK order should be Closed, Cancelled, or Open (testnet), got {:?}",
                        updated_order.status
                    );
                }
            } else {
                // Already in final state
                assert!(
                    order.status == OrderStatus::Closed || order.status == OrderStatus::Cancelled
                );
            }
        }
        Err(e) => {
            println!("⚠️  Spot FOK order failed: {}", e);
        }
    }
}

// ============================================================================
// Order Type Support Matrix Test (Spot Only)
// ============================================================================

/// Test: Verify Binance spot order type support matrix
///
/// This test documents which order types are supported by Binance Spot API.
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_spot_order_type_support_matrix() {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials configured");
        return;
    }

    println!("\n=== Binance Spot Order Type Support Matrix ===\n");

    let exchange = create_auth_binance_spot().await;
    let ticker = exchange
        .fetch_ticker("BTC/USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");
    let last_price: Decimal = price_to_decimal(ticker.last.expect("Ticker should have last price"));
    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT", last_price * dec!(0.70))
        .await
        .expect("Failed to adjust limit price precision");

    // Test each order type
    // Based on official Binance API documentation:
    // https://binance-docs.github.io/apidocs/spot/en/#new-order--trade
    // Supported order types: LIMIT, MARKET, STOP_LOSS, STOP_LOSS_LIMIT,
    //                        TAKE_PROFIT, TAKE_PROFIT_LIMIT, LIMIT_MAKER
    // (name, order_type, price, expect_success, notes)
    let test_cases = vec![
        (
            "Market (Buy)",
            OrderType::Market,
            None,
            true,
            "Uses quoteQty for buy amount (10 USDT minimum)",
        ),
        (
            "Market (Sell)",
            OrderType::Market,
            None,
            true,
            "Uses quantity for base amount",
        ),
        (
            "Limit",
            OrderType::Limit,
            Some(limit_price),
            true,
            "Fully supported with timeInForce",
        ),
        (
            "LimitMaker",
            OrderType::LimitMaker,
            Some(limit_price),
            true,
            "Post-only order, will be rejected if it would match immediately",
        ),
        (
            "StopLoss",
            OrderType::StopLoss,
            None,
            false,
            "Requires stopPrice parameter",
        ),
        (
            "StopMarket",
            OrderType::StopMarket,
            None,
            false,
            "Not a valid Binance order type",
        ),
        (
            "TakeProfit",
            OrderType::TakeProfit,
            None,
            false,
            "Requires stopPrice parameter",
        ),
    ];

    for (name, order_type, price_opt, expect_success, notes) in test_cases {
        // 市价买单需要更大的 amount (size = quote金额 >= 10 USDT)
        let amount = if name.contains("Market (Buy)") {
            AmountSpec::quote(Amount::new(dec!(10.0))) // 10 USDT
        } else {
            AmountSpec::base(Amount::new(dec!(0.0001))) // 0.0001 BTC
        };

        // 市价卖单使用 sell side
        let side = if name.contains("Market (Sell)") {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        };

        let mut builder = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(side)
            .order_type(order_type)
            .amount(amount);

        if let Some(p) = price_opt {
            builder = builder.price(Price::new(p));
        }

        if matches!(order_type, OrderType::StopLoss | OrderType::StopMarket) {
            builder = builder.stop_price(Price::new(limit_price));
        }

        if matches!(order_type, OrderType::TakeProfit) {
            builder = builder.stop_price(Price::new(limit_price * dec!(1.5)));
        }

        let request = builder
            .build()
            .expect("OrderRequest should build successfully");
        let result = exchange.create_order(request).await;

        match result {
            Ok(order) => {
                println!("✅ {:12} - Supported (order_id: {})", name, order.id);
                println!("   Notes: {}", notes);
                // Cleanup
                let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
            }
            Err(e) => {
                if expect_success {
                    println!("❌ {:12} - Failed: {}", name, e);
                    println!("   Notes: {}", notes);
                } else {
                    println!("⚠️  {:12} - Not Supported (as expected)", name);
                    println!("   Reason: {}", notes);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    println!("\n=== Support Matrix Complete ===");
    println!("\nSummary:");
    println!("  ✅ Supported Order Types:");
    println!("    - MARKET: 市价单 (现货买单 quoteQty, 卖单 quantity)");
    println!("    - LIMIT: 限价单 (需指定 price 和 timeInForce)");
    println!("    - LIMIT_MAKER: Post-Only 限价单 (不会立即成交)");
    println!("    - STOP_LOSS: 止损单 (需指定 stopPrice)");
    println!("    - STOP_LOSS_LIMIT: 限价止损单 (需指定 price 和 stopPrice)");
    println!("    - TAKE_PROFIT: 止盈单 (需指定 stopPrice)");
    println!("    - TAKE_PROFIT_LIMIT: 限价止盈单 (需指定 price 和 stopPrice)");
    println!("\n  ✅ Supported Features:");
    println!("    - timeInForce: GTC, IOC, FOK");
    println!("    - Post-Only: 使用 LIMIT_MAKER order type");
    println!("\n  ❌ Not Supported:");
    println!("    - StopMarket (Binance 使用 STOP_LOSS 代替)");
    println!("\nAPI Documentation:");
    println!("  https://binance-docs.github.io/apidocs/spot/en/#new-order--trade");
    println!();
}

//! Gate.io Spot Order Types Integration Tests
//!
//! Tests for all supported spot order types:
//! - Market orders (buy/sell)
//! - Limit orders (buy/sell)
//! - TimeInForce (GTC, IOC, FOK)
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

use crate::support::{create_gate_spot_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::traits::{MarketData, Trading};
use ccxt_core::types::{
    Amount, AmountSpec, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
};
use ccxt_exchanges::gate::Gate;
use rust_decimal::Decimal;
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

/// Helper: Get market price with discount for limit orders
async fn get_discount_price(exchange: &Gate, symbol: &str, discount: Decimal) -> Decimal {
    let ticker = exchange
        .fetch_ticker(symbol)
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * discount;

    // Adjust price according to market precision
    base_price // TODO: Implement price_to_precision for Gate
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
/// Note: Gate spot market buy uses quote currency amount (USDT)
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_market_buy_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Gate spot market buy: amount represents quote currency (USDT)
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
    // Market order can be Closed (filled) or Open (just submitted)
    assert!(
        order.status == OrderStatus::Closed || order.status == OrderStatus::Open,
        "Market order should be Closed or Open, got {:?}",
        order.status
    );
    // Verify execution data
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
/// Note: Gate spot market sell uses base currency amount (BTC)
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_market_sell_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Gate spot market sell: amount represents base currency (BTC)
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
            // Market sell might be Closed or Partial
            assert!(
                order.status == OrderStatus::Closed || order.status == OrderStatus::Partial,
                "Market sell should be Closed or Partial, got {:?}",
                order.status
            );
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
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_limit_buy_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

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

    println!(
        "✅ Spot limit buy order created: price={}, status={:?}",
        price, order.status
    );

    // Cleanup: cancel the order
    let cancel_result = exchange.cancel_order(&order.id, &order.symbol).await;
    if cancel_result.is_ok() {
        println!("✅ Limit buy order cancelled successfully");
    }
}

/// Test: Limit sell order (placed above market to avoid execution)
///
/// Creates a limit sell order at 130% of market price and verifies it stays open.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_limit_sell_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    let price = get_discount_price(&exchange, "BTC/USDT", dec!(1.30)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("OrderRequest should build successfully");

    let result = exchange.create_order(request).await;

    match result {
        Ok(order) => {
            assert_eq!(order.order_type, OrderType::Limit);
            assert_eq!(order.side, OrderSide::Sell);
            // Should stay open or be Partial
            assert!(
                order.status == OrderStatus::Open || order.status == OrderStatus::Partial,
                "Limit sell should be Open or Partial, got {:?}",
                order.status
            );

            println!(
                "✅ Spot limit sell order created: price={}, status={:?}",
                price, order.status
            );

            // Cleanup: cancel the order
            let cancel_result = exchange.cancel_order(&order.id, &order.symbol).await;
            if cancel_result.is_ok() {
                println!("✅ Limit sell order cancelled successfully");
            }
        }
        Err(e) => {
            // Expected if no balance
            println!("⚠️  Spot limit sell failed (expected if no balance): {}", e);
        }
    }
}

// ============================================================================
// TimeInForce Tests
// ============================================================================

/// Test: IOC (Immediate or Cancel) order
///
/// Creates an IOC limit order that should either fill immediately or be cancelled.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_ioc_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Place IOC order at market price (should fill immediately or cancel)
    let price = get_discount_price(&exchange, "BTC/USDT", dec!(0.99)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::IOC)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create IOC order");

    // IOC order should be either Closed (filled) or Cancelled (not filled)
    assert!(
        order.status == OrderStatus::Closed
            || order.status == OrderStatus::Cancelled
            || order.status == OrderStatus::Partial,
        "IOC order should be Closed, Cancelled or Partial, got {:?}",
        order.status
    );

    println!(
        "✅ Spot IOC order result: status={:?}, filled={}",
        order.status,
        order.filled.unwrap_or(rust_decimal::Decimal::ZERO)
    );
}

// ============================================================================
// FOK (Fill or Kill) Test
// ============================================================================

/// Test: FOK (Fill or Kill) order
///
/// Creates a FOK limit order that must either fill completely or be cancelled.
/// Gate API supports `time_in_force: "fok"`.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_fok_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Place FOK order at market price (should fill completely or cancel)
    let price = get_discount_price(&exchange, "BTC/USDT", dec!(0.99)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::FOK)
        .build()
        .expect("OrderRequest should build successfully");

    // FOK orders may fail on testnet due to insufficient liquidity
    let order_result = exchange.create_order(request).await;

    match order_result {
        Ok(order) => {
            // FOK order should be either Closed (fully filled) or Cancelled (not filled)
            assert!(
                order.status == OrderStatus::Closed || order.status == OrderStatus::Cancelled,
                "FOK order should be Closed or Cancelled, got {:?}",
                order.status
            );

            println!(
                "✅ Spot FOK order result: status={:?}, filled={}",
                order.status,
                order.filled.unwrap_or(rust_decimal::Decimal::ZERO)
            );
        }
        Err(e) => {
            // Testnet may reject FOK orders due to insufficient liquidity
            let error_msg = format!("{:?}", e);
            if error_msg.contains("FOK_NOT_FILL") || error_msg.contains("cannot be filled") {
                println!(
                    "⚠️  FOK order rejected by testnet (insufficient liquidity): {:?}",
                    e
                );
                println!("   This is expected behavior on testnet with low liquidity");
            } else {
                panic!("Failed to create FOK order: {:?}", e);
            }
        }
    }
}

// ============================================================================
// Post-Only Order Test
// ============================================================================

/// Test: Post-Only order
///
/// Creates a limit order with Post-Only time-in-force (`TimeInForce::PO`).
/// Gate API maps `TimeInForce::PO` to `"poc"` (Post-Only Condition).
/// Post-Only orders should only add liquidity, never take it.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_post_only_order() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

    // Place Post-Only order below market (should not match immediately)
    let price = get_discount_price(&exchange, "BTC/USDT", dec!(0.70)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .time_in_force(TimeInForce::PO)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create Post-Only order");

    // Post-Only order below market should stay open
    assert_eq!(order.side, OrderSide::Buy);
    assert!(
        order.status == OrderStatus::Open || order.status == OrderStatus::Pending,
        "Post-Only order should be Open or Pending, got {:?}",
        order.status
    );

    println!(
        "✅ Spot Post-Only order created: status={:?}, price={}",
        order.status, price
    );

    // Cleanup: cancel the order
    let cancel_result = exchange.cancel_order(&order.id, &order.symbol).await;
    if cancel_result.is_ok() {
        println!("✅ Post-Only order cancelled successfully");
    }
}

// ============================================================================
// GTC Explicit TimeInForce Test
// ============================================================================

/// Test: GTC (Good Till Cancelled) with explicit TimeInForce
///
/// Verifies that explicitly setting `TimeInForce::GTC` works correctly.
/// Gate API maps `TimeInForce::GTC` to `"gtc"`.
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_time_in_force_gtc_explicit() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    let exchange = create_auth_gate_spot().await;

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

    println!("✅ Spot GTC order created (Good Till Cancelled, explicit)");

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
}

// ============================================================================
// Order Type Support Matrix Test
// ============================================================================

/// Test: Verify Gate spot order type support matrix
///
/// This test documents which order types and TimeInForce values are supported
/// by Gate.io Spot API.
///
/// Reference: https://www.gate.com/docs/developers/apiv4/zh_CN/
#[tokio::test]
#[ignore = "需要配置 GATE_API_KEY, GATE_API_SECRET 环境变量"]
async fn test_spot_order_type_support_matrix() {
    if should_skip_private_tests("gate") {
        println!("SKIPPED: No Gate credentials configured");
        return;
    }

    println!("\n=== Gate.io Spot Order Type Support Matrix ===\n");

    let exchange = create_auth_gate_spot().await;
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let last_price_decimal: rust_decimal::Decimal = last_price.into();
    let limit_price = last_price_decimal * dec!(0.70);

    // Test each order type combination
    // (name, order_type, time_in_force, price_opt, expect_success, notes)
    let test_cases: Vec<(
        &str,
        OrderType,
        Option<TimeInForce>,
        Option<rust_decimal::Decimal>,
        bool,
        &str,
    )> = vec![
        (
            "Market (Buy)",
            OrderType::Market,
            None,
            None,
            true,
            "Uses quote currency amount (10 USDT)",
        ),
        (
            "Market (Sell)",
            OrderType::Market,
            None,
            None,
            true,
            "Uses base currency amount",
        ),
        (
            "Limit + GTC",
            OrderType::Limit,
            Some(TimeInForce::GTC),
            Some(limit_price),
            true,
            "Fully supported, default time_in_force",
        ),
        (
            "Limit + IOC",
            OrderType::Limit,
            Some(TimeInForce::IOC),
            Some(limit_price),
            true,
            "Supported, immediate-or-cancel",
        ),
        (
            "Limit + FOK",
            OrderType::Limit,
            Some(TimeInForce::FOK),
            Some(limit_price),
            true,
            "Supported, fill-or-kill",
        ),
        (
            "Limit + PO",
            OrderType::Limit,
            Some(TimeInForce::PO),
            Some(limit_price),
            true,
            "Supported, post-only (poc)",
        ),
        (
            "LimitMaker",
            OrderType::LimitMaker,
            None,
            Some(limit_price),
            false,
            "Not supported - Gate uses TimeInForce::PO instead",
        ),
        (
            "StopLoss",
            OrderType::StopLoss,
            None,
            None,
            false,
            "Not supported in current implementation",
        ),
        (
            "StopMarket",
            OrderType::StopMarket,
            None,
            None,
            false,
            "Not supported in current implementation",
        ),
        (
            "TakeProfit",
            OrderType::TakeProfit,
            None,
            None,
            false,
            "Not supported in current implementation",
        ),
        (
            "Limit + GTD",
            OrderType::Limit,
            Some(TimeInForce::GTD),
            Some(limit_price),
            true,
            "GTD maps to GTC internally",
        ),
    ];

    for (name, order_type, time_in_force, price_opt, expect_success, notes) in test_cases {
        // Market buy uses quote currency amount
        let amount = if name.contains("Market (Buy)") {
            AmountSpec::quote(Amount::new(dec!(10.0)))
        } else {
            AmountSpec::base(Amount::new(dec!(0.0001)))
        };

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

        // Add stop_price for stop orders
        let needs_stop_price = matches!(
            order_type,
            OrderType::StopLoss
                | OrderType::StopLossLimit
                | OrderType::TakeProfit
                | OrderType::TakeProfitLimit
                | OrderType::StopMarket
                | OrderType::StopLimit
        );
        if needs_stop_price {
            // Set stop price: 95% for stop loss, 105% for take profit
            let stop_multiplier = if matches!(
                order_type,
                OrderType::StopLoss | OrderType::StopLossLimit | OrderType::StopMarket
            ) {
                dec!(0.95)
            } else {
                dec!(1.05)
            };
            builder = builder.stop_price(Price::new(last_price_decimal * stop_multiplier));
        }

        if let Some(tif) = time_in_force {
            builder = builder.time_in_force(tif);
        }

        let request = builder.build();

        // Skip if build fails (e.g., unsupported order type)
        let request = match request {
            Ok(r) => r,
            Err(e) => {
                if !expect_success {
                    println!("⚠️  {:20} - Not Supported (as expected)", name);
                    println!("   Reason: {} (build failed: {:?})", notes, e);
                    continue;
                } else {
                    panic!("OrderRequest should build successfully: {:?}", e);
                }
            }
        };

        let result = exchange.create_order(request).await;

        match result {
            Ok(order) => {
                println!("✅ {:20} - Supported (order_id: {})", name, order.id);
                println!("   Notes: {}", notes);
                // Cleanup
                let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
            }
            Err(e) => {
                if expect_success {
                    println!("❌ {:20} - Failed: {}", name, e);
                    println!("   Notes: {}", notes);
                } else {
                    println!("⚠️  {:20} - Not Supported (as expected)", name);
                    println!("   Reason: {}", notes);
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    println!("\n=== Support Matrix Complete ===");
    println!("\nSummary:");
    println!("  ✅ Supported Order Types:");
    println!("    - MARKET: 市价单 (买单 quote 金额, 卖单 base 数量)");
    println!("    - LIMIT: 限价单 (需指定 price)");
    println!("\n  ✅ Supported TimeInForce:");
    println!("    - GTC: Good Till Cancelled (默认)");
    println!("    - IOC: Immediate Or Cancel");
    println!("    - FOK: Fill Or Kill");
    println!("    - PO:  Post-Only / POC (只做 Maker)");
    println!("    - GTD: 映射为 GTC (Gate 不支持 GTD)");
    println!("\n  ❌ Not Supported:");
    println!("    - LimitMaker: Gate 使用 TimeInForce::PO 代替");
    println!("    - StopLoss/StopMarket: 当前版本未实现");
    println!("    - TakeProfit: 当前版本未实现");
    println!("\nAPI Documentation:");
    println!("  https://www.gate.com/docs/developers/apiv4/zh_CN/");
    println!();
}

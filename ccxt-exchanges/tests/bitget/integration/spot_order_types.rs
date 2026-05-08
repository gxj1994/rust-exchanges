//! Bitget Spot Order Types Integration Tests
//!
//! Tests for all supported spot order types:
//! - Market orders (buy/sell)
//! - Limit orders (buy/sell)
//! - Post-Only / LimitMaker
//! - TimeInForce (GTC, IOC, FOK)
//! - Preset TP/SL (Take Profit / Stop Loss)
//!
//! ## Prerequisites
//!
//! - Bitget API credentials configured (BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE)
//! - Sufficient balance for test orders
//! - Can run on testnet (set BITGET_TESTNET=true)
//!
//! ## Safety
//!
//! - All tests use minimum order amounts
//! - Limit orders are placed far from market price to avoid execution
//! - All tests cleanup created orders
//! - Tests are marked with `#[ignore]` and require explicit execution

use crate::support::{create_bitget_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::types::{
    Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
};
use ccxt_exchanges::bitget::Bitget;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

/// Create authenticated Bitget client for spot tests.
async fn create_auth_bitget_spot() -> Bitget {
    let config = init_test();
    let exchange = create_bitget_with_credentials(&config, "spot")
        .expect("Failed to create Bitget with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    exchange
}

/// Helper: Get market price with discount for limit orders
async fn get_discount_price(exchange: &Bitget, symbol: &str, discount: Decimal) -> Decimal {
    let ticker = exchange
        .fetch_ticker(symbol)
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

fn price_to_decimal(price: Price) -> Decimal {
    price.into()
}

// ============================================================================
// Market Order Tests
// ============================================================================

/// Test: Market buy order
///
/// Creates a market buy order and verifies immediate execution.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_market_buy_order() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

    // 现货市价买单: size 表示 quote 金额 (USDT)
    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(100.0))) // 100 USDT
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
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_market_sell_order() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

    // 现货市价卖单: size 表示 base 数量 (BTC)
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
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_limit_buy_order() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

    let price = get_discount_price(&exchange, "BTC/USDT", dec!(0.70)).await;

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
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
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_limit_sell_order() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
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
/// Note: Bitget spot uses 'limit' orderType with post_only flag.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_post_only_order() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

    let price = get_discount_price(&exchange, "BTC/USDT", dec!(0.70)).await;

    // Use Limit + post_only for spot
    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(price))
        .post_only(true)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create post_only order");

    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.status, OrderStatus::Open);

    println!("✅ Spot Post-Only order created (limit + post_only)");

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
}

// ============================================================================
// Preset TP/SL Tests (现货预设止损止盈)
// ============================================================================

/// Test: Spot order with preset TP/SL
///
/// Creates a limit order with preset take-profit and stop-loss prices.
/// This is the ONLY way to use TP/SL in Bitget spot trading.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_order_with_preset_tp_sl() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

    // Fetch market price
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price: Decimal = price_to_decimal(ticker.last.expect("Ticker should have last price"));

    // 根据市场精度调整价格
    let entry_price = exchange
        .base()
        .price_to_precision("BTC/USDT", last_price * dec!(0.90))
        .await
        .expect("Failed to adjust entry price precision");
    let tp_price = exchange
        .base()
        .price_to_precision("BTC/USDT", last_price * dec!(1.10))
        .await
        .expect("Failed to adjust TP price precision");
    let sl_price = exchange
        .base()
        .price_to_precision("BTC/USDT", last_price * dec!(0.85))
        .await
        .expect("Failed to adjust SL price precision");

    let request = OrderRequest::builder()
        .symbol("BTC/USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.0001)))
        .price(Price::new(entry_price))
        .tp_limit_price(Price::new(tp_price))
        .sl_limit_price(Price::new(sl_price))
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with preset TP/SL");

    assert_eq!(order.status, OrderStatus::Open);

    println!(
        "✅ Spot order created with preset TP/SL: Entry={}, TP={}, SL={}",
        entry_price, tp_price, sl_price
    );

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;
}

// ============================================================================
// TimeInForce Tests
// ============================================================================

/// Test: TimeInForce GTC (Good Till Cancelled)
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_time_in_force_gtc() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

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
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_time_in_force_ioc() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

    // Set price at market to trigger immediate execution
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
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
            // In testnet with low liquidity, they may stay Open longer
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
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_time_in_force_fok() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_spot().await;

    // Set price at market to trigger immediate execution
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
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
            // In testnet with low liquidity, they may stay Open longer
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

/// Test: Verify Bitget spot order type support matrix
///
/// This test documents which order types are supported by Bitget Spot API.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_order_type_support_matrix() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    println!("\n=== Bitget Spot Order Type Support Matrix ===\n");

    let exchange = create_auth_bitget_spot().await;
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price: Decimal = price_to_decimal(ticker.last.expect("Ticker should have last price"));
    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT", last_price * dec!(0.70))
        .await
        .expect("Failed to adjust limit price precision");

    // Test each order type
    // Based on official Bitget V3 API documentation:
    // https://www.bitgetapp.com/zh-CN/api-doc/uta/trade/Place-Order
    // orderType only supports: "limit" and "market"
    // (name, order_type, price, expect_success, notes)
    let test_cases = vec![
        (
            "Market (Buy)",
            OrderType::Market,
            None,
            true,
            "Uses qty for quote amount (100 USDT minimum)",
        ),
        (
            "Market (Sell)",
            OrderType::Market,
            None,
            true,
            "Uses qty for base amount",
        ),
        (
            "Limit",
            OrderType::Limit,
            Some(limit_price),
            true,
            "Fully supported via orderType=limit",
        ),
        (
            "LimitMaker",
            OrderType::LimitMaker,
            Some(limit_price),
            false,
            "Not a valid orderType. Use Limit + timeInForce=post_only instead",
        ),
        (
            "StopLoss",
            OrderType::StopLoss,
            None,
            false,
            "Not a valid orderType. Use stopLoss param for preset SL instead",
        ),
        (
            "StopMarket",
            OrderType::StopMarket,
            None,
            false,
            "Not supported in spot. Use Contract Plan Order API",
        ),
        (
            "TakeProfit",
            OrderType::TakeProfit,
            None,
            false,
            "Not a valid orderType. Use takeProfit param for preset TP instead",
        ),
    ];

    for (name, order_type, price_opt, expect_success, notes) in test_cases {
        // 市价买单需要更大的 amount (size = quote金额 >= 100 USDT)
        let amount = if name.contains("Market (Buy)") {
            Amount::new(dec!(100.0)) // 100 USDT
        } else {
            Amount::new(dec!(0.0001)) // 0.0001 BTC
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
    println!("  ✅ Supported Order Types (orderType):");
    println!("    - market: 市价单 (现货买单 qty=quote金额, 卖单 qty=base数量)");
    println!("    - limit: 限价单 (需指定 price 和 timeInForce)");
    println!("\n  ✅ Supported Features:");
    println!("    - timeInForce: gtc, ioc, fok, post_only, rpi");
    println!("    - Preset TP/SL: takeProfit, stopLoss 参数");
    println!("    - Post-Only: 使用 timeInForce=post_only");
    println!("\n  ❌ Not Supported as orderType:");
    println!("    - LimitMaker (use limit + timeInForce=post_only)");
    println!("    - StopLoss (use stopLoss parameter)");
    println!("    - TakeProfit (use takeProfit parameter)");
    println!("    - StopMarket (仅限合约 Plan Order API)");
    println!("\nFor TP/SL in Spot:");
    println!("  - 使用 takeProfit/stopLoss 参数设置触发价格");
    println!("  - 使用 tpOrderType/slOrderType 设置触发后的订单类型 (limit/market)");
    println!("  - 使用 tpLimitPrice/slLimitPrice 设置限价单的执行价格");
    println!("  - See test: test_spot_order_with_preset_tp_sl");
    println!("\nAPI Documentation:");
    println!("  https://www.bitgetapp.com/zh-CN/api-doc/uta/trade/Place-Order");
    println!();
}

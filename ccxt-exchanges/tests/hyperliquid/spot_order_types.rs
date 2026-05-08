//! HyperLiquid 现货订单类型集成测试
//!
//! 这些测试需要连接 HyperLiquid 测试网,默认标记为 `#[ignore]`。
//!
//! ## 运行前准备
//!
//! 1. 在 HyperLiquid 测试网注册钱包: https://app.hyperliquid-testnet.xyz
//! 2. 设置环境变量:
//!    - HYPERLIQUID_TESTNET=true
//!    - HYPERLIQUID_PRIVATE_KEY (测试网钱包的私钥)
//!
//! ## 测试网现货交易对
//!
//! 测试网可用的现货交易对包括:
//! - PURR/USDC (id: 10000)
//! - 其他交易对可通过 load_markets() 查询

use ccxt_core::types::financial::{Amount, Price};
use ccxt_core::types::{OrderRequest, OrderSide, OrderType, TimeInForce};
use ccxt_exchanges::hyperliquid::HyperLiquid;
use rust_decimal_macros::dec;

use crate::support::{create_hyperliquid_with_credentials, init_test};

/// 创建带认证的 HyperLiquid 测试网实例
async fn create_authenticated_exchange() -> HyperLiquid {
    let config = init_test();
    let exchange = create_hyperliquid_with_credentials(&config, "spot").expect(
        "Failed to create authenticated HyperLiquid exchange (check HYPERLIQUID_PRIVATE_KEY)",
    );

    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");
    exchange
}

// ============================================================================
// 现货市价单测试
// ============================================================================
// 注意: HyperLiquid 没有原生市价单，使用 IOC 限价单 + price="0" 模拟
// 但测试网对现货 price="0" 支持有限，以下测试使用当前市场价格
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_spot_market_buy() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("PURR/USDC")
        .await
        .expect("Failed to get ticker");
    let price = ticker.last.expect("Ticker price is missing");

    // 现货限价买单（使用接近市价的价格）
    let request = OrderRequest::builder()
        .symbol("PURR/USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(5)))
        .price(price)
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;

    // 验证订单创建成功
    assert!(
        order.is_ok(),
        "Failed to create spot limit buy order: {:?}",
        order.err()
    );

    let order = order.unwrap();
    println!("Created spot market buy order: {}", order.id);

    // 清理: 市价单通常立即成交,无需取消
}

#[tokio::test]
#[ignore]
async fn test_spot_order_invalid_price_error() {
    let exchange = create_authenticated_exchange().await;

    // 使用无效价格（0）来测试错误处理
    let request = OrderRequest::builder()
        .symbol("PURR/USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(5)))
        .price(Price::new(dec!(0.0))) // 无效价格
        .build()
        .expect("Valid order request");

    let result = exchange.create_order(request).await;

    // 应该返回错误而不是 panic
    assert!(result.is_err(), "Should fail with invalid price error");

    let error = result.err().unwrap();
    let error_msg = error.to_string();

    // 验证错误信息包含价格相关提示
    assert!(
        error_msg.contains("price") || error_msg.contains("Invalid"),
        "Error message should mention price or invalid parameter: {}",
        error_msg
    );
}

#[tokio::test]
#[ignore]
async fn test_spot_market_sell() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("PURR/USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    let slippage = rust_decimal::Decimal::new(20, 2); // 0.2
    let aggressive_price = market_price * (rust_decimal::Decimal::new(1, 0) - slippage);

    // 使用当前市场价格（市价单效果）
    // 注意: HyperLiquid 要求市价单也必须提供价格
    // 这里使用市场价，配合 IOC TIF 实现市价单效果
    let request = OrderRequest::builder()
        .symbol("PURR/USDC")
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(5)))
        .price(ccxt_core::types::Price::new(aggressive_price))
        .build()
        .expect("Valid order request");

    let result = exchange.create_order(request).await;

    // 验证订单（测试网可能流动性不足）
    match result {
        Ok(order) => {
            println!("Created spot market sell order: {}", order.id);
            // 市价单通常立即成交（Closed）或部分成交（Partial）
            assert!(
                order.status == ccxt_core::types::OrderStatus::Closed
                    || order.status == ccxt_core::types::OrderStatus::Partial,
                "Market order should be filled immediately, got status: {:?}",
                order.status
            );
        }
        Err(e) => {
            let error_msg = e.to_string();
            // 测试网流动性不足是已知问题，不视为测试失败
            if error_msg.contains("could not immediately match") {
                println!(
                    "Market sell skipped (insufficient testnet liquidity): {}",
                    error_msg
                );
            } else {
                panic!("Market sell order failed: {}", error_msg);
            }
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_spot_limit_sell() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("PURR/USDC")
        .await
        .expect("Failed to get ticker");
    let price = ticker.last.expect("Ticker price is missing");

    // 现货卖单 - 使用限价单模拟市价单
    // 注意: HyperLiquid 测试网对 price="0" 的市价单支持有限，使用当前价格更可靠
    // 实际生产环境中，可以使用 price="0" 实现真正的市价单效果
    let request = OrderRequest::builder()
        .symbol("PURR/USDC")
        .side(OrderSide::Sell)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(5)))
        .price(price) // 使用当前价格
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;

    // 验证订单创建成功
    assert!(
        order.is_ok(),
        "Failed to create spot sell order: {:?}",
        order.err()
    );

    let order = order.unwrap();
    println!("Created spot sell order: {}", order.id);
}

// ============================================================================
// 现货限价单测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_spot_limit_order() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("PURR/USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    // 设置比市场价低 10% 的限价（在 80% 限制内，避免立即成交）
    let limit_price = market_price * dec!(0.9);

    let request = OrderRequest::builder()
        .symbol("PURR/USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(5)))
        .price(Price::new(limit_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;
    assert!(
        order.is_ok(),
        "Failed to create spot limit order: {:?}",
        order.err()
    );

    let order = order.unwrap();
    println!(
        "Created spot limit order: {} (status: {:?})",
        order.id, order.status
    );

    // 清理: 尝试取消订单（如果订单已成交或已取消，忽略错误）
    if order.status == ccxt_core::types::OrderStatus::Open {
        let cancel_result = exchange.cancel_order(&order.id, "PURR/USDC").await;
        match cancel_result {
            Ok(_) => println!("Order canceled successfully"),
            Err(e) => {
                // 订单可能已经成交或被其他进程取消，这是可以接受的
                println!("Cancel order failed (may be already filled): {:?}", e);
            }
        }
    } else {
        println!("Order status is {:?}, no need to cancel", order.status);
    }
}

#[tokio::test]
#[ignore]
async fn test_spot_postonly_order() {
    let exchange = create_authenticated_exchange().await;

    // PostOnly 订单 (Add Liquidity Only)
    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("PURR/USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    // PostOnly 卖单：设置比市场价高 10% 的价格，确保不会立即成交
    let limit_price = market_price * dec!(1.1);

    let request = OrderRequest::builder()
        .symbol("PURR/USDC")
        .side(OrderSide::Sell) // 使用卖单
        .order_type(OrderType::LimitMaker)
        .amount(Amount::new(dec!(5)))
        .price(Price::new(limit_price))
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;
    assert!(
        order.is_ok(),
        "Failed to create spot PostOnly order: {:?}",
        order.err()
    );

    let order = order.unwrap();
    println!("Created spot PostOnly order: {}", order.id);

    // 清理
    if order.status == ccxt_core::types::OrderStatus::Open {
        let _ = exchange.cancel_order(&order.id, "PURR/USDC").await;
    }
}

// ============================================================================
// 现货订单生命周期测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_spot_order_lifecycle() {
    let exchange = create_authenticated_exchange().await;

    // 1. 创建订单
    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("PURR/USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    // 设置比市场价低 10% 的限价（在 80% 限制内，避免立即成交）
    let limit_price = market_price * dec!(0.9);
    let request = OrderRequest::builder()
        .symbol("PURR/USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(5)))
        .price(Price::new(limit_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("Valid order request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");
    println!("Step 1: Created order {}", order.id);

    // 2. 查询订单
    let fetched_order = exchange.fetch_order(&order.id, "PURR/USDC").await;
    assert!(
        fetched_order.is_ok(),
        "Failed to fetch order: {:?}",
        fetched_order.err()
    );
    let fetched_order = fetched_order.unwrap();
    println!("Step 2: Fetched order {}", fetched_order.id);

    // 3. 取消订单
    if fetched_order.status == ccxt_core::types::OrderStatus::Open {
        let cancel_result = exchange.cancel_order(&order.id, "PURR/USDC").await;
        assert!(
            cancel_result.is_ok(),
            "Failed to cancel order: {:?}",
            cancel_result.err()
        );
        println!("Step 3: Cancelled order {}", order.id);
    }

    // 4. 验证订单状态
    let final_order = exchange.fetch_order(&order.id, "PURR/USDC").await;
    if let Ok(order) = final_order {
        println!("Step 4: Final order status: {:?}", order.status);
    }
}

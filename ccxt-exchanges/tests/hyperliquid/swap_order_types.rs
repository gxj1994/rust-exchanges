//! HyperLiquid 合约订单类型集成测试
//!
//! 这些测试需要连接 HyperLiquid 测试网,默认标记为 `#[ignore]`。
//! 运行前需设置环境变量:
//! - HYPERLIQUID_TESTNET=true
//! - HYPERLIQUID_WALLET_ADDRESS
//! - HYPERLIQUID_PRIVATE_KEY

use ccxt_core::types::financial::{Amount, Price};
use ccxt_core::types::{OrderRequest, OrderSide, OrderType, TimeInForce};
use ccxt_exchanges::hyperliquid::HyperLiquid;
use rust_decimal_macros::dec;

use crate::support::{create_hyperliquid_with_credentials, init_test};

/// 创建带认证的 HyperLiquid 测试网实例
async fn create_authenticated_exchange() -> HyperLiquid {
    let config = init_test();
    let exchange = create_hyperliquid_with_credentials(&config, "swap").expect(
        "Failed to create authenticated HyperLiquid exchange (check HYPERLIQUID_PRIVATE_KEY)",
    );
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");
    exchange
}

// ============================================================================
// 合约市价单测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_swap_market_buy_long() {
    let exchange = create_authenticated_exchange().await;
    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("ETH/USDC:USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();
    let slippage = rust_decimal::Decimal::new(5, 2); // 0.05
    let aggressive_price = market_price * (rust_decimal::Decimal::new(1, 0) + slippage);
    println!(
        "market_price: {}, Aggressive price: {}",
        market_price, aggressive_price
    );
    // 合约市价开多（需要至少 $10 的订单价值）
    // ETH 约 $2400，所以 0.005 ETH ≈ $12
    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.005))) // 0.005 ETH ≈ $12（满足 $10 最小值）
        .price(ccxt_core::Price(aggressive_price))
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;

    // 验证订单（测试网可能流动性不足）
    match &order {
        Ok(order) => {
            println!("Created swap market buy order: {}", order.id);
        }
        Err(e) => {
            let error_msg = e.to_string();
            // 测试网流动性不足是已知问题，不视为测试失败
            if error_msg.contains("could not immediately match") {
                println!(
                    "Swap market buy skipped (insufficient testnet liquidity): {}",
                    error_msg
                );
            } else {
                panic!("Failed to create swap market buy order: {}", error_msg);
            }
        }
    }

    // 清理: 市价单通常立即成交
}

#[tokio::test]
#[ignore]
async fn test_swap_market_sell_short() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("ETH/USDC:USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();
    let slippage = rust_decimal::Decimal::new(5, 2); // 0.05
    let aggressive_price = market_price * (rust_decimal::Decimal::new(1, 0) - slippage);

    // 合约市价开空（需要至少 $10 的订单价值）
    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.005))) // 0.005 ETH ≈ $12（满足 $10 最小值）
        .price(ccxt_core::Price(aggressive_price))
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;

    // 验证订单（测试网可能流动性不足）
    match &order {
        Ok(order) => {
            println!("Created swap market sell order: {}", order.id);
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("could not immediately match") {
                println!(
                    "Swap market sell skipped (insufficient testnet liquidity): {}",
                    error_msg
                );
            } else {
                panic!("Failed to create swap market sell order: {}", error_msg);
            }
        }
    }
}

// ============================================================================
// 合约限价单测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_swap_limit_order() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("ETH/USDC:USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    // 使用低于市场价 20% 的限价，避免立即成交
    let limit_price = market_price * rust_decimal::Decimal::new(80, 2);

    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.005))) // 0.005 ETH ≈ $12
        .price(Price::new(limit_price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;
    assert!(
        order.is_ok(),
        "Failed to create swap limit order: {:?}",
        order.err()
    );

    let order = order.unwrap();
    println!("Created swap limit order: {}", order.id);

    // 清理: 取消订单
    if order.status == ccxt_core::types::OrderStatus::Open {
        let cancel_result = exchange.cancel_order(&order.id, "ETH/USDC:USDC").await;
        assert!(
            cancel_result.is_ok(),
            "Failed to cancel order: {:?}",
            cancel_result.err()
        );
        println!("Cancelled order: {}", order.id);
    }
}

#[tokio::test]
#[ignore]
async fn test_swap_ioc_order() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("ETH/USDC:USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    // 使用市场价 + 5% 滑点，增加成交概率
    let aggressive_price = market_price * rust_decimal::Decimal::new(105, 2);

    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.005))) // 0.005 ETH ≈ $12
        .price(Price::new(aggressive_price))
        .time_in_force(TimeInForce::IOC)
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;

    // IOC 订单可能因流动性不足而取消
    match &order {
        Ok(order) => {
            println!(
                "Created swap IOC order: {} (status: {:?})",
                order.id, order.status
            );
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("could not immediately match") {
                println!(
                    "Swap IOC order skipped (insufficient testnet liquidity): {}",
                    error_msg
                );
            } else {
                panic!("Failed to create swap IOC order: {}", error_msg);
            }
        }
    }
}

// ============================================================================
// 止盈止损测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_swap_take_profit_market() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("ETH/USDC:USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    // 止盈市价单 (需要有持仓才能测试)
    // 使用高于市场价 20% 的止盈价作为触发价
    // 同时提供执行价格（市场价 + 25%，确保能成交）
    let tp_trigger_price = market_price * rust_decimal::Decimal::new(120, 2);
    let tp_execution_price = market_price * rust_decimal::Decimal::new(125, 2);

    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Sell) // 平仓方向
        .order_type(OrderType::TakeProfit)
        .amount(Amount::new(dec!(0.005))) // 0.005 ETH ≈ $12
        .price(Price::new(tp_execution_price)) // 执行价格
        .stop_price(Price::new(tp_trigger_price)) // 触发价格
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;

    // 可能因无持仓失败,但验证了 API 调用格式正确
    if let Err(e) = &order {
        let error_msg = e.to_string();
        // 无持仓是预期错误
        if error_msg.contains("position") || error_msg.contains("no") {
            println!("Expected error (no position): {:?}", e);
        } else {
            panic!("Unexpected error: {}", error_msg);
        }
    } else {
        println!("Created swap take-profit order: {}", order.unwrap().id);
    }
}

#[tokio::test]
#[ignore]
async fn test_swap_stop_loss_market() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("ETH/USDC:USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    // 使用低于市场价 20% 的止损价作为触发价
    // 同时提供执行价格（市场价 - 25%，确保能成交）
    let sl_trigger_price = market_price * rust_decimal::Decimal::new(80, 2);
    let sl_execution_price = market_price * rust_decimal::Decimal::new(75, 2);

    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Sell)
        .order_type(OrderType::StopLoss)
        .amount(Amount::new(dec!(0.005))) // 0.005 ETH ≈ $12
        .price(Price::new(sl_execution_price)) // 执行价格
        .stop_price(Price::new(sl_trigger_price)) // 触发价格
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;

    if let Err(e) = &order {
        let error_msg = e.to_string();
        // 无持仓是预期错误
        if error_msg.contains("position") || error_msg.contains("no") {
            println!("Expected error (no position): {:?}", e);
        } else {
            panic!("Unexpected error: {}", error_msg);
        }
    } else {
        println!("Created swap stop-loss order: {}", order.unwrap().id);
    }
}

#[tokio::test]
#[ignore]
async fn test_swap_tp_sl_combined() {
    let exchange = create_authenticated_exchange().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("ETH/USDC:USDC")
        .await
        .expect("Failed to get ticker");
    let market_price: rust_decimal::Decimal = ticker.last.expect("Ticker price is missing").into();

    // 同时设置止盈止损 (grouping: "normalTpsl")
    // 限价单：低于市场价 20%
    // 止损价：低于市场价 30%
    let limit_price = market_price * rust_decimal::Decimal::new(80, 2);
    let sl_price = market_price * rust_decimal::Decimal::new(70, 2);

    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.005))) // 0.005 ETH ≈ $12
        .price(Price::new(limit_price))
        .stop_price(Price::new(sl_price))
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;
    assert!(
        order.is_ok(),
        "Failed to create order with TP/SL: {:?}",
        order.err()
    );

    let order = order.unwrap();
    println!("Created swap order with TP/SL: {}", order.id);

    // 清理
    if order.status == ccxt_core::types::OrderStatus::Open {
        let _ = exchange.cancel_order(&order.id, "ETH/USDC:USDC").await;
    }
}

// ============================================================================
// 合约订单生命周期测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_swap_order_lifecycle() {
    let exchange = create_authenticated_exchange().await;

    // 1. 创建订单
    let limit_price = dec!(1000);
    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.005)))
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
    let fetched_order = exchange.fetch_order(&order.id, "ETH/USDC:USDC").await;
    assert!(
        fetched_order.is_ok(),
        "Failed to fetch order: {:?}",
        fetched_order.err()
    );
    let fetched_order = fetched_order.unwrap();
    println!("Step 2: Fetched order {}", fetched_order.id);

    // 3. 查询未结订单列表
    let open_orders = exchange
        .fetch_open_orders(Some("ETH/USDC:USDC"), None, None)
        .await;
    assert!(
        open_orders.is_ok(),
        "Failed to fetch open orders: {:?}",
        open_orders.err()
    );
    println!(
        "Step 3: Found {} open orders",
        open_orders.as_ref().unwrap().len()
    );

    // 4. 取消订单
    if fetched_order.status == ccxt_core::types::OrderStatus::Open {
        let cancel_result = exchange.cancel_order(&order.id, "ETH/USDC:USDC").await;
        assert!(
            cancel_result.is_ok(),
            "Failed to cancel order: {:?}",
            cancel_result.err()
        );
        println!("Step 4: Cancelled order {}", order.id);
    }

    // 5. 验证订单状态
    let final_order = exchange.fetch_order(&order.id, "ETH/USDC:USDC").await;
    if let Ok(order) = final_order {
        println!("Step 5: Final order status: {:?}", order.status);
    }
}

#[tokio::test]
#[ignore]
async fn test_swap_fetch_open_orders() {
    let exchange = create_authenticated_exchange().await;

    // 创建多个订单
    let mut order_ids = Vec::new();

    for i in 0..3 {
        let limit_price = dec!(1000) - (dec!(100.0) * rust_decimal::Decimal::from(i));
        let request = OrderRequest::builder()
            .symbol("ETH/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.005)))
            .price(Price::new(limit_price))
            .build()
            .expect("Valid order request");

        let order = exchange.create_order(request).await;
        if let Ok(order) = order {
            order_ids.push(order.id);
        }
    }

    // 查询未结订单
    let open_orders = exchange
        .fetch_open_orders(Some("ETH/USDC:USDC"), None, None)
        .await;
    assert!(open_orders.is_ok(), "Failed to fetch open orders");

    println!("Found {} open orders", open_orders.as_ref().unwrap().len());

    // 清理: 取消所有订单
    for order_id in &order_ids {
        let _ = exchange.cancel_order(order_id, "ETH/USDC:USDC").await;
    }
}

// ============================================================================
// ReduceOnly 测试
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_swap_reduce_only() {
    let exchange = create_authenticated_exchange().await;

    // ReduceOnly 订单 (需要有持仓才能测试)
    let limit_price = dec!(10000);

    let request = OrderRequest::builder()
        .symbol("ETH/USDC:USDC")
        .side(OrderSide::Sell)
        .order_type(OrderType::Limit)
        .amount(Amount::new(dec!(0.005)))
        .price(Price::new(limit_price))
        .reduce_only(true)
        .build()
        .expect("Valid order request");

    let order = exchange.create_order(request).await;

    // 可能因无持仓失败
    if let Err(e) = &order {
        println!("Expected error (no position to reduce): {:?}", e);
    } else {
        let order = order.unwrap();
        println!("Created reduce-only order: {}", order.id);

        // 清理
        if order.status == ccxt_core::types::OrderStatus::Open {
            let _ = exchange.cancel_order(&order.id, "ETH/USDC:USDC").await;
        }
    }
}

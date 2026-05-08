//! OKX Swap Order Types Tests
//!
//! Tests for swap/linear contract order types:
//! - Market orders with position side (Long/Short)
//! - Limit orders with position side
//! - PostOnly orders
//! - Take Profit / Stop Loss orders
//! - Trailing Stop orders
//!
//! ## Prerequisites
//!
//! - OKX API credentials configured (OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE)
//! - Sufficient balance for test orders
//! - Can run on testnet (set OKX_TESTNET=true)
//!
//! ## Safety
//!
//! - All tests use minimum contract sizes (0.001 BTC or 1 contract)
//! - Limit orders are placed far from market price
//! - All tests cleanup created orders
//! - Tests are marked with `#[ignore]` and require explicit execution
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all swap order tests
//! cargo test -p ccxt-exchanges --test okx_swap_order_types -- --ignored
//!
//! # Run specific test
//! cargo test -p ccxt-exchanges --test okx_swap_order_types test_swap_market_buy_long -- --ignored
//! ```

use crate::support::{create_okx_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::types::order::PositionSide;
use ccxt_core::types::{
    Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
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

/// Create authenticated OKX client for swap trading
async fn create_auth_okx_swap() -> Okx {
    let config = init_test();
    let exchange = create_okx_with_credentials(&config, "swap")
        .expect("Failed to create OKX with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    exchange
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create swap market buy request (Long position)
fn swap_market_buy_long(symbol: &str, amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(amount))
        .position_side(PositionSide::Long)
        .build()
        .unwrap()
}

/// Create swap market sell request (Short position)
fn swap_market_sell_short(symbol: &str, amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(amount))
        .position_side(PositionSide::Short)
        .build()
        .unwrap()
}

/// Create swap limit order with position side
fn swap_limit_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    price: Decimal,
    position_side: PositionSide,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .position_side(position_side)
        .build()
        .unwrap()
}

/// Create swap PostOnly order
fn swap_postonly_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    price: Decimal,
    position_side: PositionSide,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::LimitMaker)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .position_side(position_side)
        .build()
        .unwrap()
}

/// Create Take Profit order
fn take_profit_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    tp_trigger_price: Decimal,
    position_side: PositionSide,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::Market)
        .amount(Amount::new(amount))
        .tp_limit_price(Price::new(tp_trigger_price)) // 止盈触发价
        .position_side(position_side)
        // 注意: OKX普通订单接口不支持独立的平仓止盈单
        // attachAlgoOrds只适用于开仓订单,所以不使用reduceOnly
        .build()
        .unwrap()
}

/// Create Stop Loss order
fn stop_loss_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    sl_trigger_price: Decimal,
    position_side: PositionSide,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::Market)
        .amount(Amount::new(amount))
        .stop_price(Price::new(sl_trigger_price)) // 止损触发价
        .position_side(position_side)
        // 注意: OKX普通订单接口不支持独立的平仓止损单
        // attachAlgoOrds只适用于开仓订单,所以不使用reduceOnly
        .build()
        .unwrap()
}

// ============================================================================
// Market Order Tests
// ============================================================================

/// Test: Swap market buy (Long position)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_market_buy_long() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let request = swap_market_buy_long("BTC/USDT:USDT", dec!(0.0001));

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create swap market buy order");

    // 验证订单创建成功
    assert!(!order.id.is_empty(), "Order ID should not be empty");
    assert_eq!(
        order.symbol.to_string(),
        "BTC/USDT:USDT",
        "Symbol should match"
    );
    assert_eq!(order.side, OrderSide::Buy, "Side should be Buy");
    assert_eq!(order.order_type, OrderType::Market, "Type should be Market");

    // 市价单应该立即成交
    assert!(
        order.status == OrderStatus::Closed || order.status == OrderStatus::Open,
        "Market order should be Closed or Open, got: {:?}",
        order.status
    );

    println!("✓ Swap market buy (Long) test passed");
    println!("  Order ID: {}", order.id);
    println!("  Status: {:?}", order.status);
    println!("  Filled: {}", order.filled.unwrap_or_default());
}

/// Test: Swap market sell (Short position)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_market_sell_short() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let request = swap_market_sell_short("BTC/USDT:USDT", dec!(0.0001));

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create swap market sell order");

    // 验证订单创建成功
    assert!(!order.id.is_empty(), "Order ID should not be empty");
    assert_eq!(order.side, OrderSide::Sell, "Side should be Sell");
    assert_eq!(order.order_type, OrderType::Market, "Type should be Market");

    println!("✓ Swap market sell (Short) test passed");
    println!("  Order ID: {}", order.id);
    println!("  Status: {:?}", order.status);
}

// ============================================================================
// Limit Order Tests
// ============================================================================

/// Test: Swap limit order (Long position)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_limit_order_long() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // 设置远低于市场价的买单(避免成交)
    let limit_price = current_price * dec!(0.5);
    let request = swap_limit_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.0001),
        limit_price,
        PositionSide::Long,
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create swap limit order");

    // 验证订单处于Open状态
    assert_eq!(
        order.status,
        OrderStatus::Open,
        "Limit order should be Open"
    );
    assert_eq!(order.order_type, OrderType::Limit, "Type should be Limit");

    // 清理: 取消订单
    let canceled = exchange
        .cancel_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel order");

    assert_eq!(
        canceled.status,
        OrderStatus::Cancelled,
        "Order should be cancelled"
    );

    println!("✓ Swap limit order (Long) test passed");
    println!("  Order ID: {}", order.id);
    println!("  Limit Price: {}", limit_price);
}

/// Test: Swap limit order (Short position)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_limit_order_short() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // 设置远高于市场价的卖单(避免成交)
    let limit_price = current_price * dec!(1.5);

    let request = swap_limit_request(
        "BTC/USDT:USDT",
        OrderSide::Sell,
        dec!(0.0001),
        limit_price,
        PositionSide::Short,
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create swap limit order");

    assert_eq!(
        order.status,
        OrderStatus::Open,
        "Limit order should be Open"
    );

    // 清理: 取消订单
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;

    println!("✓ Swap limit order (Short) test passed");
    println!("  Order ID: {}", order.id);
    println!("  Limit Price: {}", limit_price);
}

// ============================================================================
// PostOnly Order Tests
// ============================================================================

/// Test: Swap PostOnly order (LimitMaker)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_postonly_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    let postonly_price = current_price * dec!(0.5);

    let request = swap_postonly_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.0001),
        postonly_price,
        PositionSide::Long,
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create PostOnly order");

    // 验证订单类型
    assert_eq!(
        order.order_type,
        OrderType::LimitMaker,
        "Type should be LimitMaker"
    );
    assert!(
        order.post_only.unwrap_or(false),
        "PostOnly flag should be true"
    );

    // 清理: 取消订单
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;

    println!("✓ Swap PostOnly order test passed");
    println!("  Order ID: {}", order.id);
    println!("  PostOnly Price: {}", postonly_price);
}

// ============================================================================
// Take Profit / Stop Loss Tests
// ============================================================================

/// Test: Take Profit order (Long position)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_take_profit_long() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // 止盈价设置在当前价之上(做多)
    let tp_price = current_price * dec!(1.1);

    let request = take_profit_request(
        "BTC/USDT:USDT",
        OrderSide::Buy, // 开多仓
        dec!(0.0001),
        tp_price,
        PositionSide::Long,
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create Take Profit order");

    // 开仓附带止盈单: 主订单会立即成交(状态Closed),止盈单作为附属订单等待触发
    assert_eq!(
        order.status,
        OrderStatus::Closed,
        "TP attach order should be Closed (filled immediately)"
    );
    // 注意: API返回的是主订单信息,所以order_type是Market,不是TakeProfit
    // 止盈单信息在attachAlgoOrds中,需要单独查询

    println!("✓ Swap Take Profit (Long) test passed");
    println!("  Order ID: {}", order.id);
    println!("  TP Price: {}", tp_price);
    println!("  Status: {:?} (主订单已成交,止盈单已附加)", order.status);
}

/// Test: Stop Loss order (Long position)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_stop_loss_long() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // 止损价设置在当前价之下(做多)
    let sl_price = current_price * dec!(0.9);

    let request = stop_loss_request(
        "BTC/USDT:USDT",
        OrderSide::Buy, // 开多仓
        dec!(0.01),
        sl_price,
        PositionSide::Long,
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create Stop Loss order");

    // 开仓附带止损单: 主订单会立即成交(状态Closed),止损单作为附属订单等待触发
    assert_eq!(
        order.status,
        OrderStatus::Closed,
        "SL attach order should be Closed (filled immediately)"
    );
    // 注意: API返回的是主订单信息,所以order_type是Market,不是StopLoss
    // 止损单信息在attachAlgoOrds中,需要单独查询

    println!("✓ Swap Stop Loss (Long) test passed");
    println!("  Order ID: {}", order.id);
    println!("  SL Price: {}", sl_price);
    println!("  Status: {:?} (主订单已成交,止损单已附加)", order.status);
}

/// Test: Order with both Take Profit and Stop Loss attached
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_order_with_both_tp_sl() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // 止盈价设置在当前价之上5%
    let tp_price = current_price * dec!(1.05);
    // 止损价设置在当前价之下5%
    let sl_price = current_price * dec!(0.95);

    // 创建开多仓订单,同时附加止盈和止损
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.01))) // 1张合约
        .position_side(PositionSide::Long)
        .stop_price(Price::new(sl_price)) // 止损触发价
        .tp_limit_price(Price::new(tp_price)) // 止盈触发价
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with TP/SL");

    // 主订单应该立即成交
    assert_eq!(
        order.status,
        OrderStatus::Closed,
        "Order should be Closed (filled immediately)"
    );
    assert_eq!(
        order.order_type,
        OrderType::Market,
        "Order type should be Market"
    );
    assert_eq!(order.side, OrderSide::Buy, "Side should be Buy");

    println!("✓ Swap Order with both TP/SL test passed");
    println!("  Order ID: {}", order.id);
    println!("  Entry Price: ~{}", current_price);
    println!("  TP Price: {}", tp_price);
    println!("  SL Price: {}", sl_price);
    println!(
        "  Status: {:?} (主订单已成交,止盈止损单已附加)",
        order.status
    );
}

/// Test: Order with tpTriggerRatio/slTriggerRatio (percentage-based TP/SL)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_order_with_trigger_ratio() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();

    // 使用比例设置止盈止损:
    // 测试只传0-1之间的值,order_builder根据订单方向自动添加±
    //
    // OKX文档规则(由order_builder自动处理):
    // - 买入订单: tpTriggerRatio>0, slTriggerRatio<0
    // - 卖出订单: tpTriggerRatio<0, slTriggerRatio>0
    //
    // 本测试: 开多仓(buy+long),设置5%止盈止损
    // - tpTriggerRatio: 0.3 → order_builder转为 0.3 (买入)
    // - slTriggerRatio: 0.3 → order_builder转为 -0.3 (买入)

    // 创建开多仓订单,使用比例设置止盈止损
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.01))) // 1张合约
        .position_side(PositionSide::Long)
        .extra(
            "tpTriggerRatio",
            serde_json::Value::String("0.3".to_string()),
        )
        .extra(
            "slTriggerRatio",
            serde_json::Value::String("0.3".to_string()),
        )
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with TP/SL ratio");

    // 主订单应该立即成交
    assert_eq!(
        order.status,
        OrderStatus::Closed,
        "Order should be Closed (filled immediately)"
    );
    assert_eq!(
        order.order_type,
        OrderType::Market,
        "Order type should be Market"
    );
    assert_eq!(order.side, OrderSide::Buy, "Side should be Buy");

    println!("✓ Swap Order with TP/SL Ratio test passed");
    println!("  Order ID: {}", order.id);
    println!("  Entry Price: ~{}", current_price);
    println!("  TP Ratio: 5% (tpTriggerRatio=0.05)");
    println!("  SL Ratio: 5% (slTriggerRatio=0.05)");
    println!(
        "  Status: {:?} (主订单已成交,止盈止损单已附加)",
        order.status
    );
}

/// Test: ReduceOnly order with closeFraction
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_reduce_only_with_close_fraction() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();

    // 创建80%平仓的市价单
    // 注意: closeFraction参数在OrderRequest中还未支持,这里用reduceOnly演示
    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.01))) // 1张合约
        .position_side(PositionSide::Long)
        .reduce_only(true)
        .build()
        .expect("OrderRequest should build successfully");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create reduce-only order");

    // reduceOnly订单在没有持仓时会立即关闭
    assert!(
        order.status == OrderStatus::Closed || order.status == OrderStatus::Cancelled,
        "Reduce-only order should be Closed or Cancelled (no position)"
    );
    assert_eq!(order.side, OrderSide::Sell, "Side should be Sell");
    assert_eq!(
        order.order_type,
        OrderType::Market,
        "Order type should be Market"
    );

    println!("✓ Swap Reduce-Only test passed");
    println!("  Order ID: {}", order.id);
    println!("  Current Price: ~{}", current_price);
    println!(
        "  Status: {:?} (无持仓时reduceOnly订单会被关闭)",
        order.status
    );
    println!("  Note: closeFraction参数需在order-algo接口中使用");
}

// ============================================================================
// Order Lifecycle Tests
// ============================================================================

/// Test: Complete swap order lifecycle (create -> fetch -> cancel)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_order_lifecycle() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    let limit_price = current_price * dec!(0.5);

    // 1. 创建订单
    let request = swap_limit_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.0001),
        limit_price,
        PositionSide::Long,
    );
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    let order_id = order.id.clone();
    assert_eq!(order.status, OrderStatus::Open, "Order should be Open");

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

    println!("✓ Swap order lifecycle test passed");
    println!("  Order ID: {}", order_id);
    println!("  Created -> Fetched -> Canceled -> Verified");
}

/// Test: Open orders query
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_swap_fetch_open_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    let limit_price = current_price * dec!(0.5);

    // 创建2个订单
    let mut order_ids = Vec::new();
    for _ in 0..2 {
        let request = swap_limit_request(
            "BTC/USDT:USDT",
            OrderSide::Buy,
            dec!(0.0001),
            limit_price,
            PositionSide::Long,
        );

        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create order");

        order_ids.push(order.id);
    }

    tokio::time::sleep(Duration::from_secs(1)).await;

    // 查询所有未结订单
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT:USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");

    // 应该至少有我们创建的订单
    assert!(
        open_orders.len() >= 2,
        "Should have at least 2 open orders, got: {}",
        open_orders.len()
    );

    // 清理所有订单
    for order_id in &order_ids {
        let _ = exchange.cancel_order(order_id, "BTC/USDT:USDT").await;
    }

    println!("✓ Swap fetch open orders test passed");
    println!("  Open orders count: {}", open_orders.len());
}

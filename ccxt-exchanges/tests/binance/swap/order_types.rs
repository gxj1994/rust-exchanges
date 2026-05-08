//! Binance U本位合约 (USDT-Margined Futures) 订单测试
//!
//! 测试覆盖:
//! - 市价单 (Market orders) - 开多/开空
//! - 限价单 (Limit orders) - 开多/开空
//! - PostOnly 限价单 (LimitMaker)
//! - IOC/FOK 订单
//! - 订单查询与取消
//!
//! ⚠️ 注意: 条件单 (STOP/TAKE_PROFIT/STOP_MARKET/TAKE_PROFIT_MARKET)
//! 必须使用 Algo Order API (POST /fapi/v1/algoOrder),而不是普通订单 API (POST /fapi/v1/order)
//! 虽然 /fapi/v1/order 文档列出了这些订单类型,但实际调用会返回:
//! "-4120: Order type not supported for this endpoint. Please use the Algo Order API endpoints instead."
//! 因此条件单测试需要等待实现 Algo Order API 后才能启用
//!
//! ## 前置条件
//!
//! - 配置 Binance API 凭证 (BINANCE_API_KEY, BINANCE_API_SECRET)
//! - 账户有足够的 USDT 余额
//! - 可在测试网运行 (设置 BINANCE_TESTNET=true)
//!
//! ## 安全说明
//!
//! - 所有测试使用最小合约数量 (0.001 BTC)
//! - 限价单设置在远离市场价格的位置
//! - 所有测试会清理创建的订单
//! - 测试标记为 `#[ignore]`,需要显式执行
//!
//! ## 运行测试
//!
//! ```bash
//! # 运行所有合约订单测试
//! cargo test --package ccxt-exchanges --test mod -- binance::swap::order_types --include-ignored
//!
//! # 运行单个测试
//! cargo test --package ccxt-exchanges --test mod -- binance::swap::order_types::test_swap_market_buy_long --include-ignored
//! ```

use crate::support::{create_binance_swap_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::types::common::ticker_params::TickerParams;
use ccxt_core::types::{
    Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
};
use ccxt_exchanges::binance::Binance;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

/// Skip test if no credentials available
macro_rules! skip_if_no_credentials {
    () => {
        if should_skip_private_tests("binance") {
            println!("SKIPPED: No Binance credentials configured");
            return;
        }
    };
}

/// Create authenticated Binance client for swap trading
async fn create_auth_binance_swap() -> Binance {
    let config = init_test();
    let exchange = create_binance_swap_with_credentials(&config)
        .expect("Failed to create Binance swap with credentials");

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
        .extra("positionSide", serde_json::json!("BOTH"))
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
        .extra("positionSide", serde_json::json!("BOTH"))
        .build()
        .unwrap()
}

/// Create swap limit order with position side
fn swap_limit_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    price: Decimal,
    position_side: &str,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .extra("positionSide", serde_json::json!(position_side))
        .build()
        .unwrap()
}

/// Create swap PostOnly order (LimitMaker not supported in futures, use LIMIT + GTX)
fn swap_postonly_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    price: Decimal,
    position_side: &str,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::PO) // PostOnly maps to GTX
        .extra("positionSide", serde_json::json!(position_side))
        .build()
        .unwrap()
}

/// Create swap IOC order
fn swap_ioc_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    price: Decimal,
    position_side: &str,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::IOC)
        .extra("positionSide", serde_json::json!(position_side))
        .build()
        .unwrap()
}

/// Create swap FOK order
fn swap_fok_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    price: Decimal,
    position_side: &str,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::FOK)
        .extra("positionSide", serde_json::json!(position_side))
        .build()
        .unwrap()
}

/// Create Stop Loss order (STOP type - 限价条件单)
fn stop_loss_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    stop_price: Decimal,
    order_price: Decimal,
    position_side: &str,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::StopLoss)
        .amount(Amount::new(amount))
        .price(Price::new(order_price))
        .stop_price(Price::new(stop_price))
        .extra("positionSide", serde_json::json!(position_side))
        .build()
        .unwrap()
}

/// Create Take Profit order (TAKE_PROFIT type - 限价条件单)
fn take_profit_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    tp_price: Decimal,
    order_price: Decimal,
    position_side: &str,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::TakeProfit)
        .amount(Amount::new(amount))
        .price(Price::new(order_price))
        .stop_price(Price::new(tp_price))
        .extra("positionSide", serde_json::json!(position_side))
        .build()
        .unwrap()
}

/// Create STOP_MARKET order
fn stop_market_request(
    symbol: &str,
    side: OrderSide,
    amount: Decimal,
    stop_price: Decimal,
    position_side: &str,
) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(side)
        .order_type(OrderType::StopMarket)
        .amount(Amount::new(amount))
        .stop_price(Price::new(stop_price))
        .extra("positionSide", serde_json::json!(position_side))
        .build()
        .unwrap()
}

// ============================================================================
// Market Order Tests
// ============================================================================

/// Test: Swap market buy (Long position)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_market_buy_long() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    let request = swap_market_buy_long("BTC/USDT:USDT", dec!(0.001));

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
    assert!(
        matches!(order.status, OrderStatus::Closed | OrderStatus::Open),
        "Status should be Closed or Open"
    );

    println!("✅ Market buy long order created: id={}", order.id);
}

/// Test: Swap market sell (Short position)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_market_sell_short() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    let request = swap_market_sell_short("BTC/USDT:USDT", dec!(0.001));

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create swap market sell order");

    assert!(!order.id.is_empty(), "Order ID should not be empty");
    assert_eq!(order.side, OrderSide::Sell, "Side should be Sell");
    assert_eq!(order.order_type, OrderType::Market, "Type should be Market");

    println!("✅ Market sell short order created: id={}", order.id);
}

// ============================================================================
// Limit Order Tests
// ============================================================================

/// Test: Swap limit buy order
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_limit_buy() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    // Fetch current price
    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // Place limit order far below market (10% below)
    let raw_price = last_price * dec!(0.90);

    // Adjust price to tick size precision
    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_price)
        .await
        .expect("Failed to adjust price precision");

    let request = swap_limit_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.001),
        limit_price,
        "BOTH",
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create swap limit buy order");

    assert!(!order.id.is_empty(), "Order ID should not be empty");
    assert_eq!(order.order_type, OrderType::Limit, "Type should be Limit");
    assert_eq!(order.status, OrderStatus::Open, "Status should be Open");

    // Cleanup: cancel the order and fetch to get final status
    exchange
        .cancel_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel order");

    // Wait for cancellation to process
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Fetch order to verify it's cancelled
    let cancelled = exchange
        .fetch_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order");

    assert_eq!(
        cancelled.status,
        OrderStatus::Cancelled,
        "Order should be cancelled"
    );

    println!("✅ Limit buy order created and cancelled: id={}", order.id);
}

/// Test: Swap limit sell order
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_limit_sell() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // Place limit order far above market (10% above)
    let raw_price = last_price * dec!(1.10);
    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_price)
        .await
        .expect("Failed to adjust price precision");

    let request = swap_limit_request(
        "BTC/USDT:USDT",
        OrderSide::Sell,
        dec!(0.001),
        limit_price,
        "BOTH",
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create swap limit sell order");

    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.status, OrderStatus::Open);

    // Cleanup
    exchange
        .cancel_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel order");

    println!("✅ Limit sell order created and cancelled: id={}", order.id);
}

// ============================================================================
// PostOnly / IOC / FOK Tests
// ============================================================================

/// Test: Swap PostOnly order (GTX)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_postonly_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // Place PostOnly order far below market (15% below, should stay open)
    let raw_price = last_price * dec!(0.85);
    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_price)
        .await
        .expect("Failed to adjust price precision");

    let request = swap_postonly_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.001),
        limit_price,
        "BOTH",
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create PostOnly order");

    assert_eq!(order.order_type, OrderType::Limit);
    // PostOnly orders should be Open or Expired (if immediately executable)
    assert!(
        matches!(
            order.status,
            OrderStatus::Open | OrderStatus::Expired | OrderStatus::Cancelled
        ),
        "PostOnly order should be Open, Expired, or Cancelled"
    );

    // Cleanup if still open
    if order.status == OrderStatus::Open {
        exchange
            .cancel_order(&order.id, "BTC/USDT:USDT")
            .await
            .expect("Failed to cancel order");
    }

    println!("✅ PostOnly order test completed: id={}", order.id);
}

/// Test: Swap IOC order
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_ioc_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // IOC order at aggressive price (5% above market, should fill immediately or cancel)
    let raw_price = last_price * dec!(1.05);
    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_price)
        .await
        .expect("Failed to adjust price precision");

    let request = swap_ioc_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.001),
        limit_price,
        "BOTH",
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create IOC order");

    // IOC orders should be Closed (filled), Cancelled (not filled), or Open (partially filled)
    assert!(
        matches!(
            order.status,
            OrderStatus::Closed | OrderStatus::Cancelled | OrderStatus::Open
        ),
        "IOC order should be Closed, Cancelled, or Open (partial fill)"
    );

    println!(
        "✅ IOC order test completed: id={}, status={:?}",
        order.id, order.status
    );
}

/// Test: Swap FOK order
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_fok_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // FOK order (Fill or Kill)
    let raw_price = last_price * dec!(1.05);
    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_price)
        .await
        .expect("Failed to adjust price precision");

    let request = swap_fok_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.001),
        limit_price,
        "BOTH",
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create FOK order");

    // FOK orders should be Closed (fully filled), Cancelled (not filled), or Open (partial)
    assert!(
        matches!(
            order.status,
            OrderStatus::Closed | OrderStatus::Cancelled | OrderStatus::Open
        ),
        "FOK order should be Closed, Cancelled, or Open"
    );

    println!(
        "✅ FOK order test completed: id={}, status={:?}",
        order.id, order.status
    );
}

// ============================================================================
// Stop Loss / Take Profit Tests (Algo Order API)
// ============================================================================

/// Test: Swap Stop Loss order (STOP type)
/// 使用 Algo Order API (POST /fapi/v1/algoOrder)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_stop_loss_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    // Fetch current price
    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // Place stop loss far below market (10% below)
    let raw_stop_price = last_price * dec!(0.90);
    let raw_order_price = last_price * dec!(0.89); // 委托价略低于触发价

    // Adjust price to tick size precision
    let stop_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_stop_price)
        .await
        .expect("Failed to adjust stop price precision");

    let order_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_order_price)
        .await
        .expect("Failed to adjust order price precision");

    let request = stop_loss_request(
        "BTC/USDT:USDT",
        OrderSide::Sell,
        dec!(0.002), // 增加数量以满足最小名义价值要求 (>= 100 USDT)
        stop_price,
        order_price,
        "BOTH",
    );

    println!("📤 Creating stop loss order at {}", stop_price);
    let order = exchange
        .create_futures_algo_order(request)
        .await
        .expect("Failed to create stop loss order");

    println!("✅ Stop loss order created:");
    println!("   ID: {}", order.id);
    println!("   Status: {:?}", order.status);
    println!("   Type: {:?}", order.order_type);
    println!("   Stop Price: {:?}", order.stop_price);

    // Verify order properties
    assert_eq!(order.symbol.to_string(), "BTC/USDT:USDT");
    assert_eq!(order.side, OrderSide::Sell);
    assert_eq!(order.order_type, OrderType::StopLoss);
    assert!(order.stop_price.is_some());

    // Note: Algo orders require a different cancel endpoint (/fapi/v1/algoOrder)
    // Cancel test will be added after implementing cancel_algo_order method
    println!("⚠️  Note: Algo order cancel requires separate API endpoint");
}

/// Test: Swap Take Profit order (TAKE_PROFIT type)
/// 使用 Algo Order API (POST /fapi/v1/algoOrder)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_take_profit_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    // Fetch current price
    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // Place take profit far above market (10% above)
    let raw_tp_price = last_price * dec!(1.10);
    let raw_order_price = last_price * dec!(1.11); // 委托价略高于触发价

    // Adjust price to tick size precision
    let tp_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_tp_price)
        .await
        .expect("Failed to adjust take profit price precision");

    let order_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_order_price)
        .await
        .expect("Failed to adjust order price precision");

    let request = take_profit_request(
        "BTC/USDT:USDT",
        OrderSide::Sell,
        dec!(0.002), // 增加数量以满足最小名义价值要求 (>= 100 USDT)
        tp_price,
        order_price,
        "BOTH",
    );

    println!("📤 Creating take profit order at {}", tp_price);
    let order = exchange
        .create_futures_algo_order(request)
        .await
        .expect("Failed to create take profit order");

    println!("✅ Take profit order created:");
    println!("   ID: {}", order.id);
    println!("   Status: {:?}", order.status);
    println!("   Type: {:?}", order.order_type);
    println!("   Stop Price: {:?}", order.stop_price);

    // Verify order properties
    assert_eq!(order.symbol.to_string(), "BTC/USDT:USDT");
    assert_eq!(order.side, OrderSide::Sell);
    assert_eq!(order.order_type, OrderType::TakeProfit);
    assert!(order.stop_price.is_some());

    // Note: Algo orders require a different cancel endpoint (/fapi/v1/algoOrder)
    println!("⚠️  Note: Algo order cancel requires separate API endpoint");
}

/// Test: Swap STOP_MARKET order
/// 使用 Algo Order API (POST /fapi/v1/algoOrder)
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_stop_market_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    // Fetch current price
    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // Place stop market far below market (10% below)
    let raw_stop_price = last_price * dec!(0.90);

    // Adjust price to tick size precision
    let stop_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_stop_price)
        .await
        .expect("Failed to adjust stop price precision");

    let request = stop_market_request(
        "BTC/USDT:USDT",
        OrderSide::Sell,
        dec!(0.002), // 增加数量以满足最小名义价值要求 (>= 100 USDT)
        stop_price,
        "BOTH",
    );

    println!("📤 Creating STOP_MARKET order at {}", stop_price);
    let order = exchange
        .create_futures_algo_order(request)
        .await
        .expect("Failed to create STOP_MARKET order");

    println!("✅ STOP_MARKET order created:");
    println!("   ID: {}", order.id);
    println!("   Status: {:?}", order.status);
    println!("   Type: {:?}", order.order_type);
    println!("   Stop Price: {:?}", order.stop_price);

    // Verify order properties
    assert_eq!(order.symbol.to_string(), "BTC/USDT:USDT");
    assert_eq!(order.side, OrderSide::Sell);
    assert_eq!(order.order_type, OrderType::StopMarket);
    assert!(order.stop_price.is_some());

    // Note: Algo orders require a different cancel endpoint (/fapi/v1/algoOrder)
    println!("⚠️  Note: Algo order cancel requires separate API endpoint");
}

// ============================================================================
// Order Query Tests
// ============================================================================

/// Test: Fetch order by ID
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_fetch_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    // Create market order
    let request = swap_market_buy_long("BTC/USDT:USDT", dec!(0.001));
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order");

    // Wait for order to be processed
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Fetch order
    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order");

    assert_eq!(fetched.id, order.id, "Order ID should match");
    assert_eq!(fetched.symbol.to_string(), "BTC/USDT:USDT");

    println!("✅ Fetch order test passed: id={}", fetched.id);
}

/// Test: Fetch open orders
#[tokio::test]
#[ignore = "需要配置 BINANCE_API_KEY, BINANCE_API_SECRET 环境变量（可在测试网运行）"]
async fn test_swap_fetch_open_orders() {
    skip_if_no_credentials!();
    let exchange = create_auth_binance_swap().await;

    // Create a limit order that will stay open
    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
        .expect("Failed to fetch ticker");

    let last_price: Decimal = ticker.last.expect("Ticker should have last price").into();

    // Place order 20% below market (should stay open)
    let raw_price = last_price * dec!(0.80);
    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", raw_price)
        .await
        .expect("Failed to adjust price precision");

    let request = swap_limit_request(
        "BTC/USDT:USDT",
        OrderSide::Buy,
        dec!(0.001),
        limit_price,
        "BOTH",
    );

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create limit order");

    // Fetch open orders
    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT:USDT"))
        .await
        .expect("Failed to fetch open orders");

    assert!(open_orders.len() > 0, "Should have at least one open order");

    // Verify our order is in the list
    let found = open_orders.iter().any(|o| o.id == order.id);
    assert!(found, "Created order should be in open orders list");

    // Cleanup
    exchange
        .cancel_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel order");

    println!(
        "✅ Fetch open orders test passed: count={}",
        open_orders.len()
    );
}

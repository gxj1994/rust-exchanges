//! OKX Spot Order Types Tests
//!
//! Tests for spot order types:
//! - Market buy (quote currency amount)
//! - Market sell (base currency amount)
//! - Limit orders
//! - PostOnly orders (LimitMaker)
//! - IOC/FOK orders
//!
//! ## Prerequisites
//!
//! - OKX API credentials configured (OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE)
//! - Sufficient USDT balance for test orders
//! - Can run on testnet (set OKX_TESTNET=true)
//!
//! ## Safety
//!
//! - All tests use minimum order amounts
//! - Limit orders are placed far from market price to avoid execution
//! - All tests cleanup created orders
//! - Tests are marked with `#[ignore]` and require explicit execution
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all spot order tests
//! cargo test -p ccxt-exchanges --test okx_spot_order_types -- --ignored
//!
//! # Run specific test
//! cargo test -p ccxt-exchanges --test okx_spot_order_types test_spot_market_buy -- --ignored
//! ```

use crate::support::{create_okx_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::types::{
    Amount, AmountSpec, OrderRequest, OrderSide, OrderStatus, OrderType, Price, TimeInForce,
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
async fn create_auth_okx_spot() -> Okx {
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Create spot market buy request (quote currency amount - USDT)
fn spot_market_buy_request(symbol: &str, quote_amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(AmountSpec::quote(Amount::new(quote_amount)))
        .build()
        .unwrap()
}

/// Create spot market sell request (base currency amount - BTC)
fn spot_market_sell_request(symbol: &str, base_amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Sell)
        .order_type(OrderType::Market)
        .amount(Amount::new(base_amount))
        .build()
        .unwrap()
}

/// Create spot limit buy request
fn spot_limit_buy_request(symbol: &str, amount: Decimal, price: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::GTC)
        .build()
        .unwrap()
}

/// Create spot PostOnly limit order (LimitMaker)
fn spot_postonly_request(symbol: &str, amount: Decimal, price: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::LimitMaker)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .build()
        .unwrap()
}

/// Create spot IOC order
fn spot_ioc_request(symbol: &str, amount: Decimal, price: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::IOC)
        .build()
        .unwrap()
}

/// Create spot FOK order
fn spot_fok_request(symbol: &str, amount: Decimal, price: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(amount))
        .price(Price::new(price))
        .time_in_force(TimeInForce::FOK)
        .build()
        .unwrap()
}

// ============================================================================
// Market Order Tests
// ============================================================================

/// Test: Spot market buy with quote currency amount (USDT)
///
/// OKX特殊规则: 现货市价买单必须使用quote金额(USDT),而非base数量(BTC)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_market_buy_with_quote_amount() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_spot().await;

    // 使用极小的quote金额 (1 USDT)
    let request = spot_market_buy_request("BTC/USDT", dec!(1.0));

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create spot market buy order");

    // 验证订单创建成功
    assert!(!order.id.is_empty(), "Order ID should not be empty");
    assert_eq!(order.symbol.to_string(), "BTC/USDT", "Symbol should match");
    assert_eq!(order.side, OrderSide::Buy, "Side should be Buy");
    assert_eq!(order.order_type, OrderType::Market, "Type should be Market");

    // 市价单应该立即成交或处于Open状态
    assert!(
        order.status == OrderStatus::Closed || order.status == OrderStatus::Open,
        "Market order should be Closed or Open, got: {:?}",
        order.status
    );

    println!("✓ Spot market buy (quote amount) test passed");
    println!("  Order ID: {}", order.id);
    println!("  Status: {:?}", order.status);
    println!("  Filled: {}", order.filled.unwrap_or_default());
}

/// Test: Spot market sell with base currency amount (BTC)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_market_sell_with_base_amount() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_spot().await;

    // 使用极小的base数量 (0.0001 BTC)
    let request = spot_market_sell_request("BTC/USDT", dec!(0.0001));

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create spot market sell order");

    // 验证订单创建成功
    assert!(!order.id.is_empty(), "Order ID should not be empty");
    assert_eq!(order.side, OrderSide::Sell, "Side should be Sell");
    assert_eq!(order.order_type, OrderType::Market, "Type should be Market");

    println!("✓ Spot market sell (base amount) test passed");
    println!("  Order ID: {}", order.id);
    println!("  Status: {:?}", order.status);
}

// ============================================================================
// Limit Order Tests
// ============================================================================

/// Test: Spot limit order (placed far from market to avoid execution)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_limit_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_spot().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // 设置远低于市场价的买单(避免成交)

    let limit_price = exchange
        .base()
        .price_to_precision("BTC/USDT", current_price * dec!(0.5))
        .await
        .expect("Failed to adjust price precision");
    let request = spot_limit_buy_request("BTC/USDT", dec!(0.0001), limit_price);

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create spot limit order");

    // 验证订单处于Open状态
    assert_eq!(
        order.status,
        OrderStatus::Open,
        "Limit order should be Open"
    );
    assert_eq!(order.order_type, OrderType::Limit, "Type should be Limit");
    assert_eq!(
        order.price.unwrap_or_default(),
        limit_price,
        "Price should match"
    );

    // 清理: 取消订单
    let canceled = exchange
        .cancel_order(&order.id, "BTC/USDT")
        .await
        .expect("Failed to cancel order");

    assert_eq!(
        canceled.status,
        OrderStatus::Cancelled,
        "Order should be cancelled"
    );

    println!("✓ Spot limit order test passed");
    println!("  Order ID: {}", order.id);
    println!("  Limit Price: {}", limit_price);
}

// ============================================================================
// PostOnly Order Tests
// ============================================================================

/// Test: Spot PostOnly order (LimitMaker)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_postonly_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_spot().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // PostOnly订单应该不会立即成交
    let postonly_price = current_price * dec!(0.5);

    let request = spot_postonly_request("BTC/USDT", dec!(0.0001), postonly_price);

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
    let _ = exchange.cancel_order(&order.id, "BTC/USDT").await;

    println!("✓ Spot PostOnly order test passed");
    println!("  Order ID: {}", order.id);
    println!("  PostOnly Price: {}", postonly_price);
}

// ============================================================================
// TimeInForce Tests
// ============================================================================

/// Test: Spot IOC order (Immediate Or Cancel)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_ioc_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_spot().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // IOC订单如果无法立即成交会被取消
    let ioc_price = current_price * dec!(0.5);

    let request = spot_ioc_request("BTC/USDT", dec!(0.0001), ioc_price);

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create IOC order");

    // IOC订单应该立即成交或被取消
    assert!(
        order.status == OrderStatus::Closed
            || order.status == OrderStatus::Cancelled
            || order.status == OrderStatus::Open,
        "IOC order should be Closed, Cancelled, or partially Open"
    );

    println!("✓ Spot IOC order test passed");
    println!("  Order ID: {}", order.id);
    println!("  Status: {:?}", order.status);
}

/// Test: Spot FOK order (Fill Or Kill)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_fok_order() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_spot().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    // FOK订单如果无法全部成交会被取消
    let fok_price = current_price * dec!(0.5);

    let request = spot_fok_request("BTC/USDT", dec!(0.0001), fok_price);

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create FOK order");

    // FOK订单应该全部成交或被取消
    assert!(
        order.status == OrderStatus::Closed || order.status == OrderStatus::Cancelled,
        "FOK order should be Closed or Cancelled"
    );

    println!("✓ Spot FOK order test passed");
    println!("  Order ID: {}", order.id);
    println!("  Status: {:?}", order.status);
}

// ============================================================================
// Order Lifecycle Tests
// ============================================================================

/// Test: Complete order lifecycle (create -> fetch -> cancel)
#[tokio::test]
#[ignore = "需要配置 OKX_API_KEY, OKX_API_SECRET, OKX_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_spot_order_lifecycle() {
    skip_if_no_credentials!();
    let exchange = create_auth_okx_spot().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    let current_price: Decimal = ticker.last.unwrap_or(Price::new(dec!(50000))).into();
    let limit_price = current_price * dec!(0.5);

    // 1. 创建订单
    let request = spot_limit_buy_request("BTC/USDT", dec!(0.0001), limit_price);
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
        .fetch_order(&order_id, "BTC/USDT")
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
        .cancel_order(&order_id, "BTC/USDT")
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
        .fetch_order(&order_id, "BTC/USDT")
        .await
        .expect("Failed to fetch order after cancel");

    assert_eq!(
        final_order.status,
        OrderStatus::Cancelled,
        "Order should remain cancelled"
    );

    println!("✓ Spot order lifecycle test passed");
    println!("  Order ID: {}", order_id);
    println!("  Created -> Fetched -> Canceled -> Verified");
}

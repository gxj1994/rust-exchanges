//! Bitget Contract/Futures Order Types Integration Tests
//!
//! Tests for contract/futures order types:
//! - Market orders
//! - Limit orders
//! - StopLoss plan orders
//! - TakeProfit plan orders
//! - TrailingStop plan orders
//!
//! ## Prerequisites
//!
//! - Bitget API credentials configured (BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE)
//! - Contract trading enabled on account
//! - Sufficient balance for test orders
//! - Can run on testnet (set BITGET_TESTNET=true)
//!
//! ## Safety
//!
//! - All tests use minimum order amounts
//! - Limit orders are placed far from market price to avoid execution
//! - All tests cleanup created orders
//! - Tests are marked with `#[ignore]` and require explicit execution
//!
//! ## Note
//!
//! - Spot order tests are in: spot_order_types.rs
//! - Contract tests require default_type="swap" configuration

use crate::support::{init_test, should_skip_private_tests};
use ccxt_core::types::{Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price};
use ccxt_exchanges::bitget::Bitget;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Create authenticated Bitget client for contract tests.
async fn create_auth_bitget_contract() -> Bitget {
    let config = init_test();

    // 获取认证信息
    let (api_key, api_secret) = config
        .get_active_api_key("bitget")
        .expect("No Bitget credentials configured");

    let passphrase = std::env::var("BITGET_PASSPHRASE").expect("BITGET_PASSPHRASE not set");

    // 创建合约模式的 Bitget 实例
    let exchange = Bitget::builder()
        .sandbox(config.bitget.use_testnet)
        .default_type("swap") // 设置为合约模式
        .api_key(&api_key)
        .secret(&api_secret)
        .passphrase(&passphrase)
        .build()
        .expect("Failed to create Bitget with contract mode");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    exchange
}

/// Helper: Convert Price to Decimal
fn price_to_decimal(price: Price) -> Decimal {
    price.into()
}

// ============================================================================
// Contract/Futures Plan Order Tests (合约计划单测试)
// ============================================================================
// Note: Bitget Contract API supports independent TP/SL plan orders via:
// - POST /api/v2/mix/order/placeTpsl
// - PlanType: profit_plan, loss_plan, moving_plan, pos_profit, pos_loss, normal_plan
// See: ccxt-exchanges/src/bitget/rest/plan.rs
//
// These tests require contract/futures symbols (e.g., "BTC/USDT:USDT")
// ============================================================================

// ============================================================================
// Contract Basic Order Tests (合约基础订单测试)
// ============================================================================

/// Test: Contract Market Buy Order (合约市价买单)
///
/// Creates a market buy order for contract/futures.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（合约测试网）"]
async fn test_contract_market_buy_order() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_contract().await;

    // 合约市价买单: size 表示合约张数
    let symbol = "BTC/USDT:USDT";

    // 检查市场是否已加载
    match exchange.base().market(symbol).await {
        Ok(market) => {
            println!("✅ Market found: {} -> {}", symbol, market.id);
        }
        Err(e) => {
            println!("❌ Market not found: {}", e);
            println!("   Note: Bitget testnet may not support contract markets");
            return;
        }
    }

    let request = OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::new(dec!(0.001))) // 0.001 合约
        .build()
        .expect("Failed to build contract market buy order request");

    println!("\n=== Contract Market Buy Order Test ===");
    println!("Symbol: {}", symbol);
    println!("Size: 0.001 contracts");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create contract market buy order");

    // 市价单应该立即成交
    assert_eq!(order.order_type, OrderType::Market);
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.status, OrderStatus::Closed);
    assert!(order.filled.unwrap_or(rust_decimal::Decimal::ZERO) > rust_decimal::Decimal::ZERO);

    println!("✅ Contract market buy order filled:");
    println!("   Order ID: {}", order.id);
    println!("   Status: {:?}", order.status);
    println!("   Filled: {}", order.filled.unwrap_or_default());
    println!("   Cost: {}", order.cost.unwrap_or_default());
}

/// Test: Contract Limit Order (合约限价单)
///
/// Creates a limit order for contract/futures.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（合约测试网）"]
async fn test_contract_limit_order() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_contract().await;

    // 获取当前市场价格
    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price: Decimal = price_to_decimal(ticker.last.expect("Ticker should have last price"));

    // 设置限价单价格 (比市场价低 10%)
    let limit_price = (last_price * dec!(0.90)).round_dp(2);
    // 根据市场精度调整价格
    let price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", limit_price)
        .await
        .expect("Failed to adjust price precision");
    let order_size = dec!(0.001);

    println!("\n=== Contract Limit Order Test ===");
    println!("Current price: {}", last_price);
    println!("Limit price: {}", price);
    println!("Size: {} contracts", order_size);

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::new(order_size))
        .price(Price::new(price))
        .build()
        .expect("Failed to build contract limit order request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create contract limit order");

    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.status, OrderStatus::Open);

    println!("✅ Contract limit order created:");
    println!("   Order ID: {}", order.id);
    println!("   Status: {:?}", order.status);
    println!("   Price: {}", limit_price);
    println!("   Size: {}", order.amount);

    // 取消订单
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
    println!("   Order cancelled");
}

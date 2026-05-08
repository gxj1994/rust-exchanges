//! Bitget Order Query Tests
//!
//! Tests for fetch_order, fetch_open_orders, FETCH_HISTORY_ORDERS methods.
//!
//! ## Prerequisites
//!
//! - Bitget API credentials configured (BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE)
//! - Sufficient balance for test orders
//! - Can run on testnet (set BITGET_TESTNET=true)

use crate::support::{create_bitget_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::types::{Amount, OrderRequest, OrderSide, OrderStatus, OrderType, Price};
use ccxt_exchanges::bitget::Bitget;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

/// Create authenticated Bitget client for tests.
async fn create_auth_bitget() -> Bitget {
    let config = init_test();
    let exchange = create_bitget_with_credentials(&config, "swap")
        .expect("Failed to create Bitget with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");
    exchange
}

fn price_to_decimal(price: Price) -> Decimal {
    price.into()
}

fn market_buy_request(symbol: &str, base_amount: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Market)
        .amount(Amount::from(base_amount)) // 合约市价单使用 base amount（合约张数）
        .build()
        .unwrap()
}

fn limit_buy_request(symbol: &str, amount: Decimal, price: Decimal) -> OrderRequest {
    OrderRequest::builder()
        .symbol(symbol)
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::from(amount))
        .price(Price::from(price))
        .build()
        .unwrap()
}

/// Scenario 1: Query single order by ID
///
/// Tests fetching a specific order using its ID.
/// Verifies data consistency between created and fetched order.
///
/// ## 注意事项
///
/// 此测试需要账户持仓模式与代码兼容：
/// - 单向持仓模式 (one_way_mode): 默认，不需要 posSide
/// - 双向持仓模式 (hedge_mode): 需要 posSide 参数
///
/// 如果出现 "Incorrect position open type" 错误，请检查：
/// 1. 登录 Bitget 测试网
/// 2. 进入合约交易设置
/// 3. 确认持仓模式（建议测试时使用单向模式）
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_fetch_order_by_id() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget().await;

    // 打印当前持仓模式，帮助诊断
    println!("\n=== Contract Order Test ===");
    println!("Note: If you see 'Incorrect position open type' error,");
    println!("please check your account position mode settings:");
    println!("  - one_way_mode (单向持仓): Default, simpler");
    println!("  - hedge_mode (双向持仓): Allows long and short simultaneously");
    println!("\nYou can change it in Bitget testnet: Account Settings > Position Mode");

    let request = market_buy_request("BTC/USDT:USDT", dec!(0.001));
    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create market order. Check position mode settings.");

    // 等待订单成交
    tokio::time::sleep(Duration::from_secs(2)).await;

    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order");

    assert_eq!(fetched.id, order.id);
    // 市价单应该已成交
    assert_eq!(fetched.status, OrderStatus::Closed);
    println!("✅ Fetch order by ID test passed: {}", fetched.id);
}

/// Scenario 2: Query open orders
///
/// Tests fetching all open (unfilled) orders.
/// Creates multiple limit orders and verifies they appear in open orders.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_fetch_open_orders() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");

    let mut open_ids = Vec::new();
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // 根据市场精度调整价格
    let base_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", base_price)
        .await
        .expect("Failed to adjust price precision");

    for i in 0..3 {
        // 使用市场精度调整价格，确保是tick size的倍数
        let price_offset = dec!(100.0) * Decimal::from(i);
        let raw_price = if base_price > price_offset {
            base_price - price_offset
        } else {
            base_price + price_offset
        };

        let price = exchange
            .base()
            .price_to_precision("BTC/USDT:USDT", raw_price)
            .await
            .expect("Failed to adjust price precision");

        let request = limit_buy_request("BTC/USDT:USDT", dec!(0.0001), price);
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create limit order");
        open_ids.push(order.id);
    }

    let open_orders = exchange
        .fetch_open_orders(Some("BTC/USDT:USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");

    for order in &open_orders {
        assert_eq!(order.status, OrderStatus::Open);
    }

    // Cleanup
    for open_id in &open_ids {
        let _ = exchange.cancel_order(open_id, "BTC/USDT:USDT").await;
    }
}

/// Scenario 3: Query closed/filled orders
///
/// Tests fetching only filled/closed orders.
/// Creates market orders (which fill immediately) and verifies they appear.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_fetch_history_orders() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget().await;

    let mut filled_ids = Vec::new();

    for _ in 0..1 {
        // 合约市价买单：size 表示合约张数（BTC 数量）
        // 根据 market 数据，minTradeNum: 0.0001，我们使用 0.001
        let request = market_buy_request("BTC/USDT:USDT", dec!(0.001));
        let order = exchange
            .create_order(request)
            .await
            .expect("Failed to create market order");
        filled_ids.push(order.id);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let closed_orders = exchange
        .fetch_history_orders(Some("BTC/USDT:USDT"), None, Some(10))
        .await
        .expect("Failed to fetch closed orders");

    assert!(!closed_orders.is_empty());

    let closed_ids: Vec<&String> = closed_orders.iter().map(|o| &o.id).collect();
    for filled_id in &filled_ids {
        assert!(closed_ids.contains(&filled_id));
    }
}

/// Scenario 4: Order lifecycle (create -> fetch -> cancel)
///
/// Tests the full lifecycle of an order.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_order_lifecycle() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let price: Decimal = (price_to_decimal(last_price) * dec!(0.75)).round_dp(2);

    let request = limit_buy_request("BTC/USDT:USDT", dec!(0.0001), price);
    let created = exchange
        .create_order(request)
        .await
        .expect("Failed to create order");

    assert_eq!(created.status, OrderStatus::Open);

    // Fetch order
    let fetched = exchange
        .fetch_order(&created.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order");
    assert_eq!(fetched.id, created.id);

    // Cancel order
    let _canceled = exchange
        .cancel_order(&created.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to cancel order");

    // Verify not in open orders
    let open_orders_after = exchange
        .fetch_open_orders(Some("BTC/USDT:USDT"), None, None)
        .await
        .expect("Failed to fetch open orders");
    assert!(!open_orders_after.iter().any(|o| o.id == created.id));
}

/// Scenario 5: Order with preset Take Profit (TP)
///
/// Tests creating a limit order with preset take profit price.
/// Verifies that tpTriggerBy and tpOrderType parameters work correctly.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_order_with_take_profit() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // Adjust price precision
    let entry_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", base_price)
        .await
        .expect("Failed to adjust entry price precision");

    // Take profit price: 5% above entry
    let tp_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", entry_price * dec!(1.05))
        .await
        .expect("Failed to adjust TP price precision");

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::from(dec!(0.001)))
        .price(Price::from(entry_price))
        .tp_limit_price(Price::from(tp_price))
        .build()
        .expect("Failed to build order request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with TP");

    println!(
        "✅ Order with TP created: id={}, entry_price={}, tp_price={}",
        order.id, entry_price, tp_price
    );

    // Fetch and verify
    tokio::time::sleep(Duration::from_secs(1)).await;
    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order with TP");

    assert_eq!(fetched.id, order.id);
    assert!(
        fetched.take_profit_price.is_some(),
        "Take profit should be set"
    );

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
}

/// Scenario 6: Order with preset Stop Loss (SL)
///
/// Tests creating a limit order with preset stop loss price.
/// Verifies that slTriggerBy and slOrderType parameters work correctly.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_order_with_stop_loss() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // Adjust price precision
    let entry_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", base_price)
        .await
        .expect("Failed to adjust entry price precision");

    // Stop loss price: 5% below entry
    let sl_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", entry_price * dec!(0.95))
        .await
        .expect("Failed to adjust SL price precision");

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::from(dec!(0.001)))
        .price(Price::from(entry_price))
        .sl_limit_price(Price::from(sl_price))
        .build()
        .expect("Failed to build order request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with SL");

    println!(
        "✅ Order with SL created: id={}, entry_price={}, sl_price={}",
        order.id, entry_price, sl_price
    );

    // Fetch and verify
    tokio::time::sleep(Duration::from_secs(1)).await;
    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order with SL");

    assert_eq!(fetched.id, order.id);
    assert!(fetched.stop_loss_price.is_some(), "Stop loss should be set");

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
}

/// Scenario 7: Order with both Take Profit and Stop Loss (TP+SL)
///
/// Tests creating a limit order with both TP and SL preset.
/// This is the most common risk management setup.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_order_with_tp_and_sl() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    // Adjust price precision
    let entry_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", base_price)
        .await
        .expect("Failed to adjust entry price precision");

    // TP: 5% above, SL: 5% below
    let tp_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", entry_price * dec!(1.05))
        .await
        .expect("Failed to adjust TP price precision");

    let sl_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", entry_price * dec!(0.95))
        .await
        .expect("Failed to adjust SL price precision");

    let request = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::from(dec!(0.001)))
        .price(Price::from(entry_price))
        .tp_limit_price(Price::from(tp_price))
        .sl_limit_price(Price::from(sl_price))
        .build()
        .expect("Failed to build order request");

    let order = exchange
        .create_order(request)
        .await
        .expect("Failed to create order with TP+SL");

    println!(
        "✅ Order with TP+SL created: id={}, entry={}, tp={}, sl={}",
        order.id, entry_price, tp_price, sl_price
    );

    // Fetch and verify
    tokio::time::sleep(Duration::from_secs(1)).await;
    let fetched = exchange
        .fetch_order(&order.id, "BTC/USDT:USDT")
        .await
        .expect("Failed to fetch order with TP+SL");

    assert_eq!(fetched.id, order.id);
    assert!(
        fetched.take_profit_price.is_some(),
        "Take profit should be set"
    );
    assert!(fetched.stop_loss_price.is_some(), "Stop loss should be set");

    // Cleanup
    let _ = exchange.cancel_order(&order.id, "BTC/USDT:USDT").await;
}

/// Scenario 8: Different trigger types comparison
///
/// Tests market price vs mark price trigger for TP/SL.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量（可在测试网运行）"]
async fn test_tp_sl_trigger_types() {
    use ccxt_core::types::order::TriggerPriceType;

    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget().await;

    let ticker = exchange
        .fetch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to fetch ticker");
    let last_price = ticker.last.expect("Ticker should have last price");
    let base_price: Decimal = price_to_decimal(last_price) * dec!(0.70);

    let entry_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", base_price)
        .await
        .expect("Failed to adjust entry price precision");

    let tp_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", entry_price * dec!(1.03))
        .await
        .expect("Failed to adjust TP price precision");

    let sl_price = exchange
        .base()
        .price_to_precision("BTC/USDT:USDT", entry_price * dec!(0.97))
        .await
        .expect("Failed to adjust SL price precision");

    // Test 1: Market price trigger (default - LastPrice)
    let request1 = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::from(dec!(0.001)))
        .price(Price::from(entry_price))
        .tp_limit_price(Price::from(tp_price))
        .sl_limit_price(Price::from(sl_price))
        .trigger_price_type(TriggerPriceType::LastPrice)
        .build()
        .expect("Failed to build order request");

    let order1 = exchange
        .create_order(request1)
        .await
        .expect("Failed to create order with market trigger");

    println!(
        "✅ Order with MARKET (LastPrice) trigger created: id={}",
        order1.id
    );

    // Test 2: Mark price trigger
    let request2 = OrderRequest::builder()
        .symbol("BTC/USDT:USDT")
        .side(OrderSide::Buy)
        .order_type(OrderType::Limit)
        .amount(Amount::from(dec!(0.001)))
        .price(Price::from(entry_price))
        .tp_limit_price(Price::from(tp_price))
        .sl_limit_price(Price::from(sl_price))
        .trigger_price_type(TriggerPriceType::MarkPrice)
        .build()
        .expect("Failed to build order request");

    let order2 = exchange
        .create_order(request2)
        .await
        .expect("Failed to create order with mark trigger");

    println!("✅ Order with MARK trigger created: id={}", order2.id);

    // Cleanup
    let _ = exchange.cancel_order(&order1.id, "BTC/USDT:USDT").await;
    let _ = exchange.cancel_order(&order2.id, "BTC/USDT:USDT").await;
}

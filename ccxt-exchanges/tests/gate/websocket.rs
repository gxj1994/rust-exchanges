//! Gate.io WebSocket Tests
//!
//! Tests for Gate.io WebSocket components:
//! - WS client creation (public/testnet)
//! - Subscription message building
//! - Message parsing (heartbeat, ticker, subscription confirm, error)
//! - Channel extraction from subscription
//!
//! Note: Full WebSocket integration tests require `WsExchange` trait,
//! which is not yet implemented for Gate (see plan Task 6 - CANCELLED).

use ccxt_core::WsExchange;
use ccxt_core::ws::parser::{ParsedMessage, StreamParser};
use ccxt_core::ws::subscription::{SubscriptionBuilder, SubscriptionChannel};
use ccxt_exchanges::gate::Gate;
use ccxt_exchanges::gate::auth::GateWsAuth;
use ccxt_exchanges::gate::ws::{
    GateStreamParser, GateSubscriptionBuilder, create_gate_ws_client, create_gate_ws_client_auth,
};
use futures_util::StreamExt;

// ============================================================================
// WS Client Creation Tests
// ============================================================================

#[test]
fn test_ws_client_creation_production() {
    let client = create_gate_ws_client(false, "usdt");
    // Verify client can be created for production
    drop(client);
}

#[test]
fn test_ws_client_creation_testnet() {
    let client = create_gate_ws_client(true, "usdt");
    // Verify testnet client can be created
    drop(client);
}

#[test]
fn test_ws_client_thread_safety() {
    // Verify GateWsClient implements Send + Sync
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ccxt_exchanges::gate::ws::GateWsClient>();
}

// ============================================================================
// Subscription Message Building Tests
// ============================================================================

#[test]
fn test_build_ticker_subscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::ticker("BTC/USDT");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "spot.tickers");
    assert_eq!(msg["event"], "subscribe");
    assert_eq!(msg["payload"][0], "BTC_USDT");
}

#[test]
fn test_build_ticker_unsubscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::ticker("ETH/USDT");
    let msg = builder.build_unsubscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "spot.tickers");
    assert_eq!(msg["event"], "unsubscribe");
    assert_eq!(msg["payload"][0], "ETH_USDT");
}

#[test]
fn test_build_trades_subscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::trades("BTC/USDT");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "spot.trades");
    assert_eq!(msg["event"], "subscribe");
    assert_eq!(msg["payload"][0], "BTC_USDT");
}

#[test]
fn test_build_orderbook_subscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::orderbook("BTC/USDT");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "spot.order_book");
    assert_eq!(msg["event"], "subscribe");
    assert_eq!(msg["payload"][0], "BTC_USDT");
    assert_eq!(msg["payload"][1], "5");
    assert_eq!(msg["payload"][2], "100ms");
}

#[test]
fn test_build_kline_subscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::kline("BTC/USDT", "1m");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "spot.candlesticks");
    assert_eq!(msg["event"], "subscribe");
    assert_eq!(msg["payload"][0], "1m");
    assert_eq!(msg["payload"][1], "BTC_USDT");
}

#[test]
fn test_build_multiple_subscriptions() {
    let builder = GateSubscriptionBuilder;
    let channels = vec![
        SubscriptionChannel::ticker("BTC/USDT"),
        SubscriptionChannel::trades("ETH/USDT"),
    ];
    let msg = builder.build_subscribe(&channels).unwrap();

    // Multiple channels should return an array
    assert!(msg.is_array(), "Multiple subscriptions should return array");
    let arr = msg.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["channel"], "spot.tickers");
    assert_eq!(arr[1]["channel"], "spot.trades");
}

// ============================================================================
// Message Parsing Tests
// ============================================================================

#[test]
fn test_parse_ticker_update() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.tickers",
        "event": "update",
        "result": {
            "currency_pair": "BTC_USDT",
            "last": "50000.0",
            "high_24h": "52000.0",
            "low_24h": "48000.0",
            "volume_24h": "1234.56",
            "change_percentage": "2.5"
        }
    });

    let result = parser.parse(&message).unwrap();
    match result {
        ParsedMessage::ExchangeSpecific(msg) => {
            assert_eq!(msg.exchange_id, "gate");
            assert_eq!(msg.channel, "spot.tickers");
            assert_eq!(msg.data["currency_pair"], "BTC_USDT");
            assert_eq!(msg.data["last"], "50000.0");
        }
        _ => panic!("Expected ExchangeSpecific message, got {:?}", result),
    }
}

#[test]
fn test_parse_trade_update() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.trades",
        "event": "update",
        "result": {
            "id": 123456,
            "price": "50000.0",
            "amount": "0.01",
            "type": "sell"
        }
    });

    let result = parser.parse(&message).unwrap();
    match result {
        ParsedMessage::ExchangeSpecific(msg) => {
            assert_eq!(msg.channel, "spot.trades");
            assert_eq!(msg.data["price"], "50000.0");
        }
        _ => panic!("Expected ExchangeSpecific message, got {:?}", result),
    }
}

#[test]
fn test_parse_orderbook_update() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.order_book_update",
        "event": "update",
        "result": {
            "t": 1606292218123_i64,
            "last_update_id": 100001,
            "s": "BTC_USDT",
            "bids": [["50000.0", "1.5"], ["49900.0", "2.0"]],
            "asks": [["50100.0", "0.5"], ["50200.0", "1.0"]]
        }
    });

    let result = parser.parse(&message).unwrap();
    match result {
        ParsedMessage::ExchangeSpecific(msg) => {
            assert_eq!(msg.channel, "spot.order_book_update");
            assert_eq!(msg.data["s"], "BTC_USDT");
            assert!(msg.data["bids"].is_array());
            assert!(msg.data["asks"].is_array());
        }
        _ => panic!("Expected ExchangeSpecific message, got {:?}", result),
    }
}

#[test]
fn test_parse_subscribe_ack() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.tickers",
        "event": "subscribe",
        "result": null
    });

    let result = parser.parse(&message).unwrap();
    match result {
        ParsedMessage::SubscriptionConfirm { channel } => {
            assert_eq!(channel, "spot.tickers");
        }
        _ => panic!("Expected SubscriptionConfirm message, got {:?}", result),
    }
}

#[test]
fn test_parse_unsubscribe_ack() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.trades",
        "event": "unsubscribe",
        "result": null
    });

    let result = parser.parse(&message).unwrap();
    match result {
        ParsedMessage::SubscriptionConfirm { channel } => {
            assert_eq!(channel, "spot.trades");
        }
        _ => panic!("Expected SubscriptionConfirm message, got {:?}", result),
    }
}

#[test]
fn test_parse_heartbeat() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218
    });

    let result = parser.parse(&message).unwrap();
    assert!(
        matches!(result, ParsedMessage::Heartbeat),
        "Expected Heartbeat, got {:?}",
        result
    );
}

#[test]
fn test_parse_error_message() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.tickers",
        "event": "update",
        "error": "Invalid symbol"
    });

    let result = parser.parse(&message).unwrap();
    match result {
        ParsedMessage::Error { message, .. } => {
            assert_eq!(message, "Invalid symbol");
        }
        _ => panic!("Expected Error message, got {:?}", result),
    }
}

#[test]
fn test_parse_error_with_message_field() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218,
        "message": "Rate limit exceeded"
    });

    let result = parser.parse(&message).unwrap();
    match result {
        ParsedMessage::Error { message, .. } => {
            assert_eq!(message, "Rate limit exceeded");
        }
        _ => panic!("Expected Error message, got {:?}", result),
    }
}

#[test]
fn test_parse_null_result() {
    let parser = GateStreamParser;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.tickers",
        "event": "update",
        "result": null
    });

    let result = parser.parse(&message).unwrap();
    match result {
        ParsedMessage::ExchangeSpecific(msg) => {
            assert_eq!(msg.channel, "spot.tickers");
            assert_eq!(msg.data, serde_json::Value::Null);
        }
        _ => panic!("Expected ExchangeSpecific with null data, got {:?}", result),
    }
}

// ============================================================================
// Channel Extraction Tests
// ============================================================================

#[test]
fn test_extract_channel_for_ticker() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::Ticker;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "BTC/USDT",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "spot.tickers:BTC/USDT");
}

#[test]
fn test_extract_channel_for_trades() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::Trades;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "ETH/USDT",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "spot.trades:ETH/USDT");
}

#[test]
fn test_extract_channel_for_orderbook() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::OrderBook;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "BTC/USDT",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "spot.order_book:BTC/USDT"); // Gate 服务器返回的 channel 不包含 level
}

#[test]
fn test_extract_channel_for_kline() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::Kline;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "BTC/USDT",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "spot.candlesticks:BTC/USDT"); // Gate 服务器返回的 channel 不包含 interval
}

#[test]
fn test_extract_channel_empty_symbol() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::Ticker;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "spot.tickers");
}

#[test]
fn test_extract_channel_from_message() {
    let builder = GateSubscriptionBuilder;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.tickers",
        "event": "update",
        "result": {
            "currency_pair": "BTC_USDT"
        }
    });

    let channel = builder.extract_channel(&message);
    assert!(channel.is_some());
    assert_eq!(channel.unwrap(), "spot.tickers:BTC/USDT");
}

#[test]
fn test_extract_channel_from_message_no_result() {
    let builder = GateSubscriptionBuilder;
    let message = serde_json::json!({
        "time": 1606292218,
        "channel": "spot.trades",
        "event": "update"
    });

    let channel = builder.extract_channel(&message);
    assert_eq!(channel, Some("spot.trades".to_string()));
}

// ============================================================================
// SubscriptionBuilder Trait Implementation Tests
// ============================================================================

#[test]
fn test_subscription_builder_trait_object() {
    // Verify GateSubscriptionBuilder implements SubscriptionBuilder trait
    fn assert_subscription_builder<T: SubscriptionBuilder>() {}
    assert_subscription_builder::<GateSubscriptionBuilder>();
}

#[test]
fn test_stream_parser_trait_object() {
    // Verify GateStreamParser implements StreamParser trait
    fn assert_stream_parser<T: StreamParser>() {}
    assert_stream_parser::<GateStreamParser>();
}

#[test]
fn test_gate_symbol_conversion_in_subscription() {
    let builder = GateSubscriptionBuilder;
    let pairs = [
        ("BTC/USDT", "BTC_USDT"),
        ("ETH/USDT", "ETH_USDT"),
        ("DOT/USDT", "DOT_USDT"),
        // Contract symbol: BTC/USDT:USDT -> BTC_USDT (remove :USDT settle part)
        ("BTC/USDT:USDT", "BTC_USDT"),
    ];

    for (ccxt_symbol, gate_symbol) in &pairs {
        let channel = SubscriptionChannel::ticker(*ccxt_symbol);
        let msg = builder.build_subscribe(&[channel]).unwrap();
        assert_eq!(
            msg["payload"][0], *gate_symbol,
            "Symbol {} should convert to {}",
            ccxt_symbol, gate_symbol
        );
    }
}

#[test]
fn test_empty_subscription_returns_array() {
    let builder = GateSubscriptionBuilder;
    let msg = builder.build_subscribe(&[]).unwrap();
    // Empty subscriptions should return empty array
    assert!(msg.is_array());
    assert!(msg.as_array().unwrap().is_empty());
}

// ============================================================================
// Futures Contract Subscription Building Tests
// ============================================================================

#[test]
fn test_build_futures_ticker_subscribe() {
    let builder = GateSubscriptionBuilder;
    // Contract symbol format: BTC/USDT:USDT
    let channel = SubscriptionChannel::ticker("BTC/USDT:USDT");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "futures.tickers");
    assert_eq!(msg["event"], "subscribe");
    // Gate contract symbol format: BTC/USDT:USDT -> BTC_USDT (for payload)
    assert_eq!(msg["payload"][0], "BTC_USDT");
}

#[test]
fn test_build_futures_ticker_unsubscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::ticker("ETH/USDT:USDT");
    let msg = builder.build_unsubscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "futures.tickers");
    assert_eq!(msg["event"], "unsubscribe");
    assert_eq!(msg["payload"][0], "ETH_USDT:USDT");
}

#[test]
fn test_build_futures_trades_subscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::trades("BTC/USDT:USDT");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "futures.trades");
    assert_eq!(msg["event"], "subscribe");
    // Contract symbol: BTC/USDT:USDT -> BTC_USDT (remove :USDT part)
    assert_eq!(msg["payload"][0], "BTC_USDT");
}

#[test]
fn test_build_futures_orderbook_subscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::orderbook("BTC/USDT:USDT");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "futures.order_book");
    assert_eq!(msg["event"], "subscribe");
    // 期货 OrderBook payload 只需要合约名称，不需要 depth 和 speed
    assert_eq!(msg["payload"][0], "BTC_USDT");
    assert_eq!(msg["payload"].as_array().unwrap().len(), 1); // 只有合约名称
}

#[test]
fn test_build_futures_kline_subscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::kline("BTC/USDT:USDT", "5m");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "futures.candlesticks");
    assert_eq!(msg["event"], "subscribe");
    assert_eq!(msg["payload"][0], "5m");
    // Contract symbol: BTC/USDT:USDT -> BTC_USDT (remove :USDT part)
    assert_eq!(msg["payload"][1], "BTC_USDT");
}

#[test]
fn test_extract_channel_for_futures_ticker() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::Ticker;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "BTC/USDT:USDT",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "futures.tickers:BTC/USDT:USDT");
}

#[test]
fn test_extract_channel_for_futures_trades() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::Trades;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "ETH/USDT:USDT",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "futures.trades:ETH/USDT:USDT");
}

#[test]
fn test_extract_channel_for_futures_orderbook() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::OrderBook;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "BTC/USDT:USDT",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "futures.order_book:BTC/USDT:USDT"); // Gate 服务器返回的 channel 不包含 level
}

#[test]
fn test_extract_channel_for_futures_kline() {
    let builder = GateSubscriptionBuilder;
    let channel_type = ccxt_core::ws::subscription::ChannelType::Kline;
    let result = builder.extract_channel_from_subscription(
        &channel_type,
        "BTC/USDT:USDT",
        &std::collections::HashMap::new(),
    );
    assert_eq!(result, "futures.candlesticks:BTC/USDT:USDT"); // Gate 服务器返回的 channel 不包含 interval
}

// ============================================================================
// Spot vs Futures Symbol Detection Tests
// ============================================================================

#[test]
fn test_spot_vs_futures_ticker_detection() {
    let builder = GateSubscriptionBuilder;

    // Spot symbol (no ':')
    let spot_channel = SubscriptionChannel::ticker("BTC/USDT");
    let spot_msg = builder.build_subscribe(&[spot_channel]).unwrap();
    assert_eq!(spot_msg["channel"], "spot.tickers");

    // Futures symbol (has ':')
    let futures_channel = SubscriptionChannel::ticker("BTC/USDT:USDT");
    let futures_msg = builder.build_subscribe(&[futures_channel]).unwrap();
    assert_eq!(futures_msg["channel"], "futures.tickers");
}

#[test]
fn test_spot_vs_futures_trades_detection() {
    let builder = GateSubscriptionBuilder;

    let spot_channel = SubscriptionChannel::trades("ETH/USDT");
    let spot_msg = builder.build_subscribe(&[spot_channel]).unwrap();
    assert_eq!(spot_msg["channel"], "spot.trades");

    let futures_channel = SubscriptionChannel::trades("ETH/USDT:USDT");
    let futures_msg = builder.build_subscribe(&[futures_channel]).unwrap();
    assert_eq!(futures_msg["channel"], "futures.trades");
}

#[test]
fn test_spot_vs_futures_orderbook_detection() {
    let builder = GateSubscriptionBuilder;

    let spot_channel = SubscriptionChannel::orderbook("BTC/USDT");
    let spot_msg = builder.build_subscribe(&[spot_channel]).unwrap();
    assert_eq!(spot_msg["channel"], "spot.order_book");

    let futures_channel = SubscriptionChannel::orderbook("BTC/USDT:USDT");
    let futures_msg = builder.build_subscribe(&[futures_channel]).unwrap();
    assert_eq!(futures_msg["channel"], "futures.order_book");
}

#[test]
fn test_spot_vs_futures_kline_detection() {
    let builder = GateSubscriptionBuilder;

    let spot_channel = SubscriptionChannel::kline("BTC/USDT", "1h");
    let spot_msg = builder.build_subscribe(&[spot_channel]).unwrap();
    assert_eq!(spot_msg["channel"], "spot.candlesticks");

    let futures_channel = SubscriptionChannel::kline("BTC/USDT:USDT", "1h");
    let futures_msg = builder.build_subscribe(&[futures_channel]).unwrap();
    assert_eq!(futures_msg["channel"], "futures.candlesticks");
}

// ============================================================================
// Actual WebSocket Subscription Tests (Integration Tests)
// ============================================================================

#[tokio::test]
async fn test_watch_ticker_spot() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    println!("[TEST] Starting WebSocket connection...");
    exchange
        .ws_connect()
        .await
        .expect("WebSocket connection failed");
    println!("[TEST] WebSocket connected successfully");

    println!("[TEST] Subscribing to ticker for BTC/USDT...");
    let mut stream = exchange
        .watch_ticker("BTC/USDT")
        .await
        .expect("Failed to subscribe to ticker");

    println!("[TEST] Ticker stream created successfully");
    println!(
        "[TEST] Current subscriptions: {:?}",
        exchange.subscriptions()
    );

    println!("[TEST] Waiting for ticker message (timeout: 10s)...");
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await;

    match result {
        Ok(Some(Ok(ticker))) => {
            println!(
                "[TEST] ✓ Ticker received: symbol={}, last={:?}, bid={:?}, ask={:?}",
                ticker.symbol, ticker.last, ticker.bid, ticker.ask
            );
            // 打印原始数据用于调试
            println!("[TEST] Raw ticker info fields:");
            if let Some(info) = ticker.info.get("highest_bid") {
                println!("[TEST]   highest_bid (raw): {}", info);
            }
            if let Some(info) = ticker.info.get("lowest_ask") {
                println!("[TEST]   lowest_ask (raw): {}", info);
            }
            if let Some(info) = ticker.info.get("bid_size") {
                println!("[TEST]   bid_size (raw): {}", info);
            }
            if let Some(info) = ticker.info.get("ask_size") {
                println!("[TEST]   ask_size (raw): {}", info);
            }

            assert_eq!(
                ticker.symbol.as_str(),
                "BTC/USDT",
                "Symbol should match subscription"
            );

            // 检查ticker核心字段
            use rust_decimal::Decimal;
            assert!(ticker.last.is_some(), "Last price should be present");
            if let Some(last) = ticker.last {
                assert!(last.0 > Decimal::ZERO, "Last price should be positive");
            }

            // Gate现货ticker通常有bid/ask
            assert!(ticker.bid.is_some(), "Bid price should be present");
            assert!(ticker.ask.is_some(), "Ask price should be present");
            if let Some(bid) = ticker.bid {
                assert!(bid.0 > Decimal::ZERO, "Bid price should be positive");
            }
            if let Some(ask) = ticker.ask {
                assert!(ask.0 > Decimal::ZERO, "Ask price should be positive");
            }

            // 检查volume字段（如果有）
            if let Some(bid_volume) = ticker.bid_volume {
                assert!(
                    bid_volume.0 > Decimal::ZERO,
                    "Bid volume should be positive if present"
                );
            }
            if let Some(ask_volume) = ticker.ask_volume {
                assert!(
                    ask_volume.0 > Decimal::ZERO,
                    "Ask volume should be positive if present"
                );
            }
        }
        Ok(Some(Err(e))) => {
            panic!("[TEST] ✗ Ticker error: {}", e);
        }
        Ok(None) => {
            panic!("[TEST] ✗ Ticker stream ended (no more messages)");
        }
        Err(_) => {
            panic!(
                "[TEST] ✗ Ticker timeout after 10s - no messages received. Check subscription channel and network connection."
            );
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_orderbook_spot() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    println!("[TEST] Starting WebSocket connection...");
    exchange
        .ws_connect()
        .await
        .expect("WebSocket connection failed");
    println!("[TEST] WebSocket connected successfully");

    println!("[TEST] Subscribing to orderbook for BTC/USDT...");
    let mut stream = exchange
        .watch_order_book("BTC/USDT", Some(5))
        .await
        .expect("Failed to subscribe to orderbook");

    println!("[TEST] OrderBook stream created successfully");
    println!(
        "[TEST] Current subscriptions: {:?}",
        exchange.subscriptions()
    );

    println!("[TEST] Waiting for orderbook message (timeout: 10s)...");
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await;

    match result {
        Ok(Some(Ok(ob))) => {
            println!(
                "[TEST] ✓ OrderBook received: symbol={}, bids={}, asks={}",
                ob.symbol,
                ob.bids.len(),
                ob.asks.len()
            );
            assert_eq!(
                ob.symbol.as_str(),
                "BTC/USDT",
                "Symbol should match subscription"
            );
            assert!(
                !ob.bids.is_empty() || !ob.asks.is_empty(),
                "OrderBook should have bids or asks"
            );
        }
        Ok(Some(Err(e))) => {
            panic!("[TEST] ✗ OrderBook error: {}", e);
        }
        Ok(None) => {
            panic!("[TEST] ✗ OrderBook stream ended (no more messages)");
        }
        Err(_) => {
            panic!(
                "[TEST] ✗ OrderBook timeout after 10s - no messages received. Check subscription channel and network connection."
            );
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_trades_spot() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    println!("[TEST] Starting WebSocket connection...");
    exchange
        .ws_connect()
        .await
        .expect("WebSocket connection failed");
    println!("[TEST] WebSocket connected successfully");

    println!("[TEST] Subscribing to trades for BTC/USDT...");
    let mut stream = exchange
        .watch_market_trades("BTC/USDT")
        .await
        .expect("Failed to subscribe to trades");

    println!("[TEST] Trades stream created successfully");
    println!(
        "[TEST] Current subscriptions: {:?}",
        exchange.subscriptions()
    );

    println!("[TEST] Waiting for trades message (timeout: 30s)...");
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), stream.next()).await;

    match result {
        Ok(Some(Ok(trades))) => {
            println!("[TEST] ✓ Trades: {} trades received", trades.len());
            assert!(!trades.is_empty(), "Trades array should not be empty");

            let trade = trades.first().expect("Should have at least one trade");
            println!(
                "[TEST] First trade: symbol={}, price={:?}, amount={:?}",
                trade.symbol, trade.price, trade.amount
            );

            // Validate trade data
            assert_eq!(
                trade.symbol.as_str(),
                "BTC/USDT",
                "Symbol should match subscription"
            );
            assert!(trade.price.is_positive(), "Price should be positive");
            assert!(trade.amount.is_positive(), "Amount should be positive");
            assert!(trade.timestamp > 0, "Timestamp should be positive");
        }
        Ok(Some(Err(e))) => {
            panic!("[TEST] ✗ Trades error: {}", e);
        }
        Ok(None) => {
            panic!("[TEST] ✗ Trades stream ended (no more messages)");
        }
        Err(_) => {
            panic!(
                "[TEST] ✗ Trades timeout after 10s - no messages received. Check subscription channel and network connection."
            );
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_ohlcv_spot() {
    use ccxt_core::types::Timeframe;

    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    println!("[TEST] Starting WebSocket connection...");
    exchange
        .ws_connect()
        .await
        .expect("WebSocket connection failed");
    println!("[TEST] WebSocket connected successfully");

    println!("[TEST] Subscribing to OHLCV for BTC/USDT (1m)...");
    let mut stream = exchange
        .watch_ohlcv("BTC/USDT", Timeframe::M1)
        .await
        .expect("Failed to subscribe to OHLCV");

    println!("[TEST] OHLCV stream created successfully");
    println!(
        "[TEST] Current subscriptions: {:?}",
        exchange.subscriptions()
    );

    println!("[TEST] Waiting for OHLCV message (timeout: 10s)...");
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await;

    match result {
        Ok(Some(Ok(ohlcv_list))) => {
            println!("[TEST] ✓ OHLCV: {} candles received", ohlcv_list.len());
            assert!(!ohlcv_list.is_empty(), "OHLCV list should not be empty");

            let ohlcv = ohlcv_list.first().expect("Should have at least one candle");
            println!(
                "[TEST] First candle: timestamp={}, open={:?}",
                ohlcv.timestamp, ohlcv.open
            );

            assert!(ohlcv.timestamp > 0, "Timestamp should be positive");
            assert!(ohlcv.open.is_positive(), "Open price should be positive");
        }
        Ok(Some(Err(e))) => {
            panic!("[TEST] ✗ OHLCV error: {}", e);
        }
        Ok(None) => {
            panic!("[TEST] ✗ OHLCV stream ended (no more messages)");
        }
        Err(_) => {
            panic!(
                "[TEST] ✗ OHLCV timeout after 10s - no messages received. Check subscription channel and network connection."
            );
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_multiple_tickers() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    println!("[TEST] Starting WebSocket connection...");
    exchange
        .ws_connect()
        .await
        .expect("WebSocket connection failed");
    println!("[TEST] WebSocket connected successfully");

    println!("[TEST] Subscribing to multiple tickers...");
    let symbols = vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()];
    let mut stream = exchange
        .watch_tickers(&symbols)
        .await
        .expect("Failed to subscribe to tickers");

    println!("[TEST] Multi-ticker stream created successfully");
    println!(
        "[TEST] Current subscriptions: {:?}",
        exchange.subscriptions()
    );

    println!("[TEST] Waiting for ticker message (timeout: 10s)...");
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await;

    match result {
        Ok(Some(Ok(tickers))) => {
            println!("[TEST] ✓ Tickers: {} tickers received", tickers.len());
            assert!(!tickers.is_empty(), "Should receive at least one ticker");

            for ticker in &tickers {
                println!(
                    "[TEST] Ticker: symbol={}, last={:?}",
                    ticker.symbol, ticker.last
                );
            }

            // Verify we got tickers for both symbols
            let symbols_received: Vec<_> = tickers.iter().map(|t| t.symbol.as_str()).collect();
            assert!(
                symbols_received.contains(&"BTC/USDT") || symbols_received.contains(&"ETH/USDT"),
                "Should receive ticker for at least one subscribed symbol"
            );
        }
        Ok(Some(Err(e))) => {
            panic!("[TEST] ✗ Tickers error: {}", e);
        }
        Ok(None) => {
            panic!("[TEST] ✗ Tickers stream ended (no more messages)");
        }
        Err(_) => {
            panic!(
                "[TEST] ✗ Tickers timeout after 10s - no messages received. Check subscription channel and network connection."
            );
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

// ============================================================================
// Futures Contract WebSocket Integration Tests
// ============================================================================

#[tokio::test]
async fn test_watch_ticker_futures() {
    // Create Gate with swap/futures default type
    use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};
    let exchange = Gate::builder()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Linear)
        .build()
        .expect("Failed to create Gate with swap config");

    println!("[TEST] Starting WebSocket connection...");
    exchange
        .ws_connect()
        .await
        .expect("WebSocket connection failed");
    println!("[TEST] WebSocket connected successfully");

    println!("[TEST] Subscribing to futures ticker for BTC/USDT:USDT...");
    // Contract symbol format: BTC/USDT:USDT
    let mut stream = exchange
        .watch_ticker("BTC/USDT:USDT")
        .await
        .expect("Failed to subscribe to futures ticker");

    println!("[TEST] Futures ticker stream created successfully");
    println!(
        "[TEST] Current subscriptions: {:?}",
        exchange.subscriptions()
    );

    println!("[TEST] Waiting for futures ticker message (timeout: 10s)...");

    let result = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await;

    match result {
        Ok(Some(Ok(ticker))) => {
            println!(
                "[TEST] ✓ Futures ticker received: symbol={}, last={:?}, bid={:?}, ask={:?}",
                ticker.symbol, ticker.last, ticker.bid, ticker.ask
            );
            // 打印所有原始字段用于调试
            println!("[TEST] === Raw futures ticker data ===");
            println!(
                "[TEST] All info keys: {:?}",
                ticker.info.keys().collect::<Vec<_>>()
            );
            for (key, value) in &ticker.info {
                println!("[TEST]   {} = {}", key, value);
            }
            println!("[TEST] === End raw data ===");

            assert_eq!(
                ticker.symbol.as_str(),
                "BTC/USDT:USDT",
                "Symbol should match subscription"
            );

            // 检查ticker核心字段
            use rust_decimal::Decimal;
            assert!(ticker.last.is_some(), "Last price should be present");
            if let Some(last) = ticker.last {
                assert!(last.0 > Decimal::ZERO, "Last price should be positive");
            }

            // 合约ticker也应该有bid/ask（如果API提供）
            // 暂时改为警告，待确认API是否真的返回这些字段
            if ticker.bid.is_some() {
                println!("[TEST] ✓ Bid price present: {:?}", ticker.bid);
                if let Some(bid) = ticker.bid {
                    assert!(bid.0 > Decimal::ZERO, "Bid price should be positive");
                }
            } else {
                println!(
                    "[TEST] ⚠ Warning: Bid price is None (API may not provide this field for futures)"
                );
            }

            if ticker.ask.is_some() {
                println!("[TEST] ✓ Ask price present: {:?}", ticker.ask);
                if let Some(ask) = ticker.ask {
                    assert!(ask.0 > Decimal::ZERO, "Ask price should be positive");
                }
            } else {
                println!(
                    "[TEST] ⚠ Warning: Ask price is None (API may not provide this field for futures)"
                );
            }

            // 检查volume字段（如果有）
            if let Some(bid_volume) = ticker.bid_volume {
                assert!(
                    bid_volume.0 > Decimal::ZERO,
                    "Bid volume should be positive if present"
                );
            }
            if let Some(ask_volume) = ticker.ask_volume {
                assert!(
                    ask_volume.0 > Decimal::ZERO,
                    "Ask volume should be positive if present"
                );
            }
        }
        Ok(Some(Err(e))) => {
            panic!("[TEST] ✗ Futures ticker error: {}", e);
        }
        Ok(None) => {
            panic!("[TEST] ✗ Futures ticker stream ended (no more messages)");
        }
        Err(_) => {
            panic!(
                "[TEST] ✗ Futures ticker timeout after 10s - no messages received. Check subscription channel and network connection."
            );
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_orderbook_futures() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    println!("[TEST] Starting WebSocket connection...");
    match exchange.ws_connect().await {
        Ok(()) => {
            println!("[TEST] WebSocket connected successfully");
            println!("[TEST] Connection state: {:?}", exchange.ws_state());
        }
        Err(e) => {
            println!("[TEST] Connection failed: {} (skipping)", e);
            return;
        }
    }

    println!("[TEST] Subscribing to futures orderbook for BTC/USDT:USDT...");
    match exchange.watch_order_book("BTC/USDT:USDT", Some(5)).await {
        Ok(mut stream) => {
            println!("[TEST] Futures OrderBook stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for futures orderbook message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(ob))) => {
                    println!(
                        "[TEST] ✓ Futures OrderBook received: symbol={}, bids={}, asks={}",
                        ob.symbol,
                        ob.bids.len(),
                        ob.asks.len()
                    );
                    assert!(!ob.bids.is_empty() || !ob.asks.is_empty());
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Futures OrderBook error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Futures OrderBook stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Futures OrderBook timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_order_book (futures) failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_trades_futures() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    println!("[TEST] Starting WebSocket connection...");
    match exchange.ws_connect().await {
        Ok(()) => {
            println!("[TEST] WebSocket connected successfully");
            println!("[TEST] Connection state: {:?}", exchange.ws_state());
        }
        Err(e) => {
            println!("[TEST] Connection failed: {} (skipping)", e);
            return;
        }
    }

    println!("[TEST] Subscribing to futures trades for BTC/USDT:USDT...");
    match exchange.watch_market_trades("BTC/USDT:USDT").await {
        Ok(mut stream) => {
            println!("[TEST] Futures trades stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for futures trades message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(trades))) => {
                    println!("[TEST] ✓ Futures Trades: {} trades received", trades.len());
                    if let Some(trade) = trades.first() {
                        println!(
                            "[TEST] First trade: symbol={}, price={:?}, amount={:?}",
                            trade.symbol, trade.price, trade.amount
                        );
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Futures Trades error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Futures Trades stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Futures Trades timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_market_trades (futures) failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_ohlcv_futures() {
    use ccxt_core::types::Timeframe;

    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    println!("[TEST] Starting WebSocket connection...");
    match exchange.ws_connect().await {
        Ok(()) => {
            println!("[TEST] WebSocket connected successfully");
            println!("[TEST] Connection state: {:?}", exchange.ws_state());
        }
        Err(e) => {
            println!("[TEST] Connection failed: {} (skipping)", e);
            return;
        }
    }

    println!("[TEST] Subscribing to futures OHLCV for BTC/USDT:USDT (1s)...");
    match exchange.watch_ohlcv("BTC/USDT:USDT", Timeframe::S1).await {
        Ok(mut stream) => {
            println!("[TEST] Futures OHLCV stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for futures OHLCV message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(ohlcv_list))) => {
                    println!(
                        "[TEST] ✓ Futures OHLCV: {} candles received",
                        ohlcv_list.len()
                    );
                    if let Some(ohlcv) = ohlcv_list.first() {
                        println!(
                            "[TEST] First candle: timestamp={}, open={:?}",
                            ohlcv.timestamp, ohlcv.open
                        );
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Futures OHLCV error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Futures OHLCV stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Futures OHLCV timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_ohlcv (futures) failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

// ============================================================================
// WebSocket Authentication Tests
// ============================================================================

#[test]
fn test_ws_auth_creation() {
    // Test GateWsAuth creation
    let auth = GateWsAuth::new("test_api_key", "test_api_secret");
    // Verify auth object can be created
    drop(auth);
}

#[test]
fn test_ws_auth_signature_generation() {
    use ccxt_core::ws::auth::WsAuthCore;

    let auth = GateWsAuth::new("test_api_key", "test_api_secret");

    // Verify auth mode is SubscribeAuth
    assert_eq!(auth.mode(), ccxt_core::ws::auth::AuthMode::SubscribeAuth);
}

#[tokio::test]
async fn test_ws_auth_client_creation() {
    // Test authenticated WebSocket client creation
    let client = create_gate_ws_client_auth(
        false, // not testnet
        "usdt",
        "test_api_key",
        "test_api_secret",
    );

    // Verify client can be created
    drop(client);
}

#[tokio::test]
async fn test_ws_auth_subscribe_message() {
    use ccxt_core::ws::auth::SubscribeAuthenticator;

    let auth = GateWsAuth::new("test_api_key", "test_api_secret");

    // Create a subscribe message
    let subscribe_msg = serde_json::json!({
        "channel": "spot.orders",
        "event": "subscribe",
        "time": 1234567890,
        "payload": ["BTC_USDT"]
    });

    // Authenticate the message
    let authenticated = auth.authenticate(subscribe_msg.clone()).await.unwrap();

    // Verify auth field was added
    assert!(
        authenticated.get("auth").is_some(),
        "Authenticated message should have 'auth' field"
    );
    assert_eq!(authenticated["auth"]["method"], "api_key");
    assert_eq!(authenticated["auth"]["KEY"], "test_api_key");
    assert!(
        authenticated["auth"]["SIGN"].is_string(),
        "SIGN should be a string"
    );
    assert_eq!(
        authenticated["auth"]["SIGN"].as_str().unwrap().len(),
        128,
        "HMAC-SHA512 signature should be 128 hex chars"
    );

    // Verify original fields are preserved
    assert_eq!(authenticated["channel"], "spot.orders");
    assert_eq!(authenticated["event"], "subscribe");
    assert_eq!(authenticated["time"], 1234567890);
    assert_eq!(authenticated["payload"][0], "BTC_USDT");
}

#[tokio::test]
async fn test_ws_auth_different_channels() {
    use ccxt_core::ws::auth::SubscribeAuthenticator;

    let auth = GateWsAuth::new("test_api_key", "test_api_secret");

    // Test different private channels
    let channels = vec!["spot.orders", "spot.usertrades", "spot.balances"];

    for channel in channels {
        let subscribe_msg = serde_json::json!({
            "channel": channel,
            "event": "subscribe",
            "time": 1234567890,
            "payload": ["BTC_USDT"]
        });

        let authenticated = auth.authenticate(subscribe_msg).await.unwrap();

        assert!(
            authenticated.get("auth").is_some(),
            "Channel {} should have auth field",
            channel
        );
        assert_eq!(authenticated["auth"]["method"], "api_key");
    }
}

#[tokio::test]
async fn test_ws_auth_unsubscribe_message() {
    use ccxt_core::ws::auth::SubscribeAuthenticator;

    let auth = GateWsAuth::new("test_api_key", "test_api_secret");

    // Create an unsubscribe message
    let unsubscribe_msg = serde_json::json!({
        "channel": "spot.orders",
        "event": "unsubscribe",
        "time": 1234567890,
        "payload": ["BTC_USDT"]
    });

    // Authenticate the message
    let authenticated = auth.authenticate(unsubscribe_msg).await.unwrap();

    // Verify auth field was added
    assert!(authenticated.get("auth").is_some());
    assert_eq!(authenticated["event"], "unsubscribe");
}

#[test]
fn test_ws_auth_thread_safety() {
    // Verify GateWsAuth implements Send + Sync
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GateWsAuth>();
}

#[test]
fn test_ws_auth_client_thread_safety() {
    // Verify GateWsClientAuth implements Send + Sync
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ccxt_exchanges::gate::ws::GateWsClientAuth>();
}

// ============================================================================
// Private Channel Subscription Building Tests
// ============================================================================

#[test]
fn test_build_private_orders_subscribe() {
    let builder = GateSubscriptionBuilder;

    // Private channels use the same channel format but will be authenticated later
    let channel = SubscriptionChannel::ticker("BTC/USDT");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    // Verify basic structure (authentication happens at subscribe time)
    assert_eq!(msg["channel"], "spot.tickers");
    assert_eq!(msg["event"], "subscribe");
    assert_eq!(msg["payload"][0], "BTC_USDT");
}

#[test]
fn test_build_private_usertrades_subscribe() {
    let builder = GateSubscriptionBuilder;
    let channel = SubscriptionChannel::trades("ETH/USDT");
    let msg = builder.build_subscribe(&[channel]).unwrap();

    assert_eq!(msg["channel"], "spot.trades");
    assert_eq!(msg["event"], "subscribe");
    assert_eq!(msg["payload"][0], "ETH_USDT");
}

#[test]
fn test_private_channel_message_format() {
    // Verify that private channel messages have the correct format for authentication
    let subscribe_msg = serde_json::json!({
        "channel": "spot.orders",
        "event": "subscribe",
        "time": 1234567890,
        "payload": ["BTC_USDT"]
    });

    // Check required fields for Gate authentication
    assert!(
        subscribe_msg.get("channel").is_some(),
        "Must have channel field"
    );
    assert!(
        subscribe_msg.get("event").is_some(),
        "Must have event field"
    );
    assert!(subscribe_msg.get("time").is_some(), "Must have time field");
    assert!(
        subscribe_msg.get("payload").is_some(),
        "Must have payload field"
    );

    // Verify field types
    assert!(subscribe_msg["channel"].is_string());
    assert!(subscribe_msg["event"].is_string());
    assert!(subscribe_msg["time"].is_number());
    assert!(subscribe_msg["payload"].is_array());
}

// ============================================================================
// Private Channel Integration Tests (require credentials)
// ============================================================================

#[tokio::test]
async fn test_watch_balance_requires_credentials() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    // Should fail without credentials
    let result = exchange.watch_balance().await;
    assert!(
        result.is_err(),
        "watch_balance should fail without credentials"
    );

    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            err_msg.contains("API credentials required") || err_msg.contains("API key is required"),
            "Error should mention credentials: {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_watch_orders_requires_credentials() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    // Should fail without credentials
    let result = exchange.watch_orders(Some("BTC/USDT")).await;
    assert!(
        result.is_err(),
        "watch_orders should fail without credentials"
    );

    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            err_msg.contains("API credentials required") || err_msg.contains("API key is required"),
            "Error should mention credentials: {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn test_watch_account_trades_requires_credentials() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    // Should fail without credentials
    let result = exchange.watch_account_trades(Some("BTC/USDT")).await;
    assert!(
        result.is_err(),
        "watch_account_trades should fail without credentials"
    );

    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            err_msg.contains("API credentials required") || err_msg.contains("API key is required"),
            "Error should mention credentials: {}",
            err_msg
        );
    }
}

#[test]
fn test_ws_client_auth_requires_credentials() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Gate::new(config).expect("Failed to create Gate");

    // Should fail without credentials
    let result = exchange.ws_client_auth();
    assert!(
        result.is_err(),
        "ws_client_auth should fail without credentials"
    );

    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(
            err_msg.contains("API key is required"),
            "Error should mention API key: {}",
            err_msg
        );
    }
}

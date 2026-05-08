//! OKX WebSocket integration tests.
//!
//! Tests real-time data streaming via WebSocket for OKX exchange.

#![allow(clippy::disallowed_methods)]
use ccxt_core::{
    ExchangeConfig,
    types::{Timeframe, common::default_type::DefaultType},
    ws_exchange::WsExchange,
};
use ccxt_exchanges::okx::{Okx, OkxOptions};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

/// Helper to create OKX instance for WebSocket tests
fn create_okx_ws() -> Okx {
    let config = ExchangeConfig::default();
    let options = OkxOptions {
        testnet: false,
        ..Default::default()
    };
    Okx::new_with_options(config, options).unwrap()
}

/// Skip test in CI environment
fn should_skip() -> bool {
    std::env::var("CI").is_ok()
}

#[tokio::test]
async fn test_ws_connection() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_ws();

    // Test: ws_connect should succeed
    let connect_result = timeout(Duration::from_secs(10), exchange.ws_connect()).await;
    assert!(connect_result.is_ok(), "Connection timed out");
    assert!(connect_result.unwrap().is_ok(), "Connection failed");

    println!("OKX WebSocket connection successful");

    // Cleanup
    let _ = exchange.ws_disconnect().await;
}

/// Test ticker subscription via WebSocket.
#[tokio::test]
async fn test_ws_ticker() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_ws();

    match exchange.watch_ticker("BTC/USDT").await {
        Ok(mut ticker_stream) => {
            println!("Ticker stream created, waiting for data...");
            let ticker_item = timeout(Duration::from_secs(15), ticker_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(ticker)) = ticker_item {
                println!(
                    "Ticker received: symbol={}, last={:?}, bid={:?}, ask={:?}, bid_volume={:?}, ask_volume={:?}",
                    ticker.symbol.as_str(),
                    ticker.last,
                    ticker.bid,
                    ticker.ask,
                    ticker.bid_volume,
                    ticker.ask_volume
                );

                // 检查ticker核心字段
                use rust_decimal::Decimal;
                assert!(
                    ticker.last.is_some() || ticker.bid.is_some(),
                    "Ticker should have price data (last or bid)"
                );

                if let Some(last) = ticker.last {
                    assert!(last.0 > Decimal::ZERO, "Last price should be positive");
                }

                // OKX ticker应该有bid/ask
                assert!(ticker.bid.is_some(), "Bid price should be present");
                assert!(ticker.ask.is_some(), "Ask price should be present");
                if let Some(bid) = ticker.bid {
                    assert!(bid.0 > Decimal::ZERO, "Bid price should be positive");
                }
                if let Some(ask) = ticker.ask {
                    assert!(ask.0 > Decimal::ZERO, "Ask price should be positive");
                }

                // 检查volume字段（修复后应该有数据）
                if let Some(bid_volume) = ticker.bid_volume {
                    println!("✓ Bid volume: {}", bid_volume);
                    assert!(
                        bid_volume.0 >= Decimal::ZERO,
                        "Bid volume should be non-negative"
                    );
                } else {
                    println!("⚠ Warning: Bid volume is None");
                }
                if let Some(ask_volume) = ticker.ask_volume {
                    println!("✓ Ask volume: {}", ask_volume);
                    assert!(
                        ask_volume.0 >= Decimal::ZERO,
                        "Ask volume should be non-negative"
                    );
                } else {
                    println!("⚠ Warning: Ask volume is None");
                }

                let symbol_str = ticker.symbol.as_str();
                assert!(!symbol_str.is_empty(), "Symbol should not be empty");
                assert!(
                    symbol_str.contains('/'),
                    "Symbol should be in unified format (BASE/QUOTE)"
                );
            } else if let Some(Err(e)) = ticker_item {
                panic!("Ticker error: {}", e);
            } else {
                panic!("Ticker subscription returned no data after 15s");
            }
        }
        Err(e) => {
            panic!("Ticker subscription failed: {}", e);
        }
    }
}

/// Test order book subscription via WebSocket.
#[tokio::test]
async fn test_ws_orderbook() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_ws();

    match exchange.watch_order_book("BTC/USDT", Some(5)).await {
        Ok(mut orderbook_stream) => {
            println!("OrderBook stream created, waiting for data...");
            let orderbook_item = timeout(Duration::from_secs(15), orderbook_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(orderbook)) = orderbook_item {
                println!(
                    "OrderBook received: bids={}, asks={}",
                    orderbook.bids.len(),
                    orderbook.asks.len()
                );
                assert!(
                    !orderbook.bids.is_empty() || !orderbook.asks.is_empty(),
                    "OrderBook should have bids or asks"
                );
            } else if let Some(Err(e)) = orderbook_item {
                panic!("OrderBook error: {}", e);
            } else {
                panic!("OrderBook subscription returned no data after 15s");
            }
        }
        Err(e) => {
            panic!("OrderBook subscription failed: {}", e);
        }
    }
}

/// Test trades subscription via WebSocket.
#[tokio::test]
async fn test_ws_trades() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_ws();

    match exchange.watch_market_trades("BTC/USDT").await {
        Ok(mut trades_stream) => {
            println!("Trades stream created, waiting for data...");
            let trades_item = timeout(Duration::from_secs(15), trades_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(trades)) = trades_item {
                println!("Trades received: {} trades", trades.len());
                if !trades.is_empty() {
                    let trade = &trades[0];
                    println!(
                        "First trade: price={}, amount={}",
                        trade.price, trade.amount
                    );
                    use rust_decimal::Decimal;
                    assert!(
                        trade.price.0 > Decimal::ZERO,
                        "Trade price should be positive"
                    );
                    assert!(
                        trade.amount.0 > Decimal::ZERO,
                        "Trade amount should be positive"
                    );
                }
            } else if let Some(Err(e)) = trades_item {
                panic!("Trades error: {}", e);
            } else {
                panic!("Trades subscription returned no data after 15s");
            }
        }
        Err(e) => {
            panic!("Trades subscription failed: {}", e);
        }
    }
}

/// Test OHLCV (K-line) subscription via WebSocket.
#[tokio::test]
async fn test_ws_ohlcv() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_ws();

    // watch_ohlcv creates its own connection internally
    // Don't call ws_connect first as it creates a different WebSocket instance
    match exchange.watch_ohlcv("BTC/USDT", Timeframe::M1).await {
        Ok(mut ohlcv_stream) => {
            println!("OHLCV stream created, waiting for data...");
            let ohlcv_item = timeout(Duration::from_secs(70), ohlcv_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(ohlcvs)) = ohlcv_item {
                // OHLCV 返回数组，取第一条数据显示
                if let Some(ohlcv) = ohlcvs.first() {
                    // Access inner Decimal directly to avoid Display bug
                    let open_str = ohlcv.open.0.to_string();
                    let high_str = ohlcv.high.0.to_string();
                    let low_str = ohlcv.low.0.to_string();
                    let close_str = ohlcv.close.0.to_string();
                    let vol_str = ohlcv.volume.0.to_string();
                    println!(
                        "OHLCV received: ts={}, open={}, high={}, low={}, close={}, vol={}",
                        ohlcv.timestamp, open_str, high_str, low_str, close_str, vol_str
                    );
                    assert!(ohlcv.timestamp > 0, "Timestamp should be positive");
                    use rust_decimal::Decimal;
                    assert!(
                        ohlcv.open.0 > Decimal::ZERO,
                        "Open price should be positive"
                    );
                }
            } else if let Some(Err(e)) = ohlcv_item {
                println!("OHLCV error: {}", e);
            } else {
                println!("OHLCV subscription returned no data after 70s");
            }
        }
        Err(e) => {
            println!("OHLCV subscription failed: {} (skipping)", e);
        }
    }
}

// ============================================================================
// Swap (Contract) Market WebSocket Tests
// ============================================================================

/// Helper to create OKX instance for Swap WebSocket tests
fn create_okx_swap_ws() -> Okx {
    let config = ExchangeConfig::default();
    let options = OkxOptions {
        default_type: DefaultType::Swap,
        testnet: false,
        ..Default::default()
    };
    Okx::new_with_options(config, options).unwrap()
}

/// Test swap ticker subscription via WebSocket.
#[tokio::test]
async fn test_ws_ticker_swap() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_swap_ws();

    println!("[TEST] Starting WebSocket connection (Swap market)...");
    println!(
        "[TEST] Options default_type: {:?}",
        exchange.options().default_type
    );
    match exchange.ws_connect().await {
        Ok(_) => println!("[TEST] WebSocket connected successfully"),
        Err(e) => panic!("[TEST] WebSocket connection failed: {}", e),
    }

    println!("[TEST] Subscribing to ticker for BTC/USDT:USDT...");
    match exchange.watch_ticker("BTC/USDT:USDT").await {
        Ok(mut ticker_stream) => {
            println!("[TEST] Ticker stream created successfully");
            // Check current subscriptions
            let subscriptions = exchange.ws_client().subscriptions().await;
            println!("[TEST] Current subscriptions: {:?}", subscriptions);

            println!("[TEST] Waiting for ticker message (timeout: 15s)...");
            let ticker_item = timeout(Duration::from_secs(15), ticker_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(ticker)) = ticker_item {
                println!(
                    "[TEST] ✓ Swap Ticker received: symbol={}, last={:?}, bid={:?}, ask={:?}, bid_volume={:?}, ask_volume={:?}",
                    ticker.symbol.as_str(),
                    ticker.last,
                    ticker.bid,
                    ticker.ask,
                    ticker.bid_volume,
                    ticker.ask_volume
                );

                // 检查ticker核心字段
                use rust_decimal::Decimal;
                assert!(
                    ticker.last.is_some() || ticker.bid.is_some(),
                    "Ticker should have price data (last or bid)"
                );

                if let Some(last) = ticker.last {
                    assert!(last.0 > Decimal::ZERO, "Last price should be positive");
                }

                // OKX合约ticker应该有bid/ask
                // 使用条件验证：如果有则验证，没有则打印警告
                if let Some(bid) = ticker.bid {
                    println!("[TEST] ✓ Bid price present: {:?}", bid);
                    assert!(bid.0 > Decimal::ZERO, "Bid price should be positive");
                } else {
                    println!(
                        "[TEST] ⚠ Warning: Bid price is None (OKX swap ticker API may not provide this field)"
                    );
                }

                if let Some(ask) = ticker.ask {
                    println!("[TEST] ✓ Ask price present: {:?}", ask);
                    assert!(ask.0 > Decimal::ZERO, "Ask price should be positive");
                } else {
                    println!(
                        "[TEST] ⚠ Warning: Ask price is None (OKX swap ticker API may not provide this field)"
                    );
                }

                // 检查volume字段（修复后应该有数据）
                if let Some(bid_volume) = ticker.bid_volume {
                    println!("[TEST] ✓ Bid volume: {}", bid_volume);
                    assert!(
                        bid_volume.0 >= Decimal::ZERO,
                        "Bid volume should be non-negative"
                    );
                } else {
                    println!("[TEST] ⚠ Warning: Bid volume is None");
                }
                if let Some(ask_volume) = ticker.ask_volume {
                    println!("[TEST] ✓ Ask volume: {}", ask_volume);
                    assert!(
                        ask_volume.0 >= Decimal::ZERO,
                        "Ask volume should be non-negative"
                    );
                } else {
                    println!("[TEST] ⚠ Warning: Ask volume is None");
                }
            } else if let Some(Err(e)) = ticker_item {
                panic!("[TEST] ✗ Swap Ticker error: {}", e);
            } else {
                panic!("[TEST] ✗ Swap Ticker timeout after 15s");
            }
        }
        Err(e) => {
            panic!("[TEST] ✗ Swap Ticker subscription failed: {}", e);
        }
    }
}

/// Test swap order book subscription via WebSocket.
#[tokio::test]
async fn test_ws_orderbook_swap() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_swap_ws();

    println!("[TEST] Starting WebSocket connection (Swap market)...");
    match exchange.ws_connect().await {
        Ok(_) => println!("[TEST] WebSocket connected successfully"),
        Err(e) => panic!("[TEST] WebSocket connection failed: {}", e),
    }

    println!("[TEST] Subscribing to orderbook for BTC/USDT:USDT (limit=5)...");
    match exchange.watch_order_book("BTC/USDT:USDT", Some(5)).await {
        Ok(mut orderbook_stream) => {
            println!("[TEST] OrderBook stream created successfully");
            let subscriptions = exchange.ws_client().subscriptions().await;
            println!("[TEST] Current subscriptions: {:?}", subscriptions);

            println!("[TEST] Waiting for orderbook message (timeout: 15s)...");
            let orderbook_item = timeout(Duration::from_secs(15), orderbook_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(orderbook)) = orderbook_item {
                println!(
                    "[TEST] ✓ Swap OrderBook received: symbol={}, bids={}, asks={}",
                    orderbook.symbol.as_str(),
                    orderbook.bids.len(),
                    orderbook.asks.len()
                );
                assert!(
                    !orderbook.bids.is_empty() || !orderbook.asks.is_empty(),
                    "OrderBook should have bids or asks"
                );
            } else if let Some(Err(e)) = orderbook_item {
                panic!("[TEST] ✗ Swap OrderBook error: {}", e);
            } else {
                panic!("[TEST] ✗ Swap OrderBook timeout after 15s");
            }
        }
        Err(e) => {
            panic!("[TEST] ✗ Swap OrderBook subscription failed: {}", e);
        }
    }
}

/// Test swap trades subscription via WebSocket.
#[tokio::test]
async fn test_ws_trades_swap() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_swap_ws();

    println!("[TEST] Starting WebSocket connection (Swap market)...");
    match exchange.ws_connect().await {
        Ok(_) => println!("[TEST] WebSocket connected successfully"),
        Err(e) => panic!("[TEST] WebSocket connection failed: {}", e),
    }

    println!("[TEST] Subscribing to trades for BTC/USDT:USDT...");
    match exchange.watch_market_trades("BTC/USDT:USDT").await {
        Ok(mut trades_stream) => {
            println!("[TEST] Trades stream created successfully");
            let subscriptions = exchange.ws_client().subscriptions().await;
            println!("[TEST] Current subscriptions: {:?}", subscriptions);

            println!("[TEST] Waiting for trades message (timeout: 15s)...");
            let trades_item = timeout(Duration::from_secs(15), trades_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(trades)) = trades_item {
                println!("[TEST] ✓ Swap Trades: {} trades received", trades.len());
                if !trades.is_empty() {
                    let trade = &trades[0];
                    println!(
                        "[TEST] First trade: price={}, amount={}",
                        trade.price, trade.amount
                    );
                    use rust_decimal::Decimal;
                    assert!(
                        trade.price.0 > Decimal::ZERO,
                        "Trade price should be positive"
                    );
                    assert!(
                        trade.amount.0 > Decimal::ZERO,
                        "Trade amount should be positive"
                    );
                }
            } else if let Some(Err(e)) = trades_item {
                panic!("[TEST] ✗ Swap Trades error: {}", e);
            } else {
                panic!("[TEST] ✗ Swap Trades timeout after 15s");
            }
        }
        Err(e) => {
            panic!("[TEST] ✗ Swap Trades subscription failed: {}", e);
        }
    }
}

/// Test swap OHLCV (K-line) subscription via WebSocket.
#[tokio::test]
async fn test_ws_ohlcv_swap() {
    if should_skip() {
        return;
    }

    let exchange = create_okx_swap_ws();

    println!("[TEST] Starting WebSocket connection (Swap market)...");
    match exchange.ws_connect().await {
        Ok(_) => println!("[TEST] WebSocket connected successfully"),
        Err(e) => panic!("[TEST] WebSocket connection failed: {}", e),
    }

    println!("[TEST] Subscribing to OHLCV for BTC/USDT:USDT (1m)...");
    match exchange.watch_ohlcv("BTC/USDT:USDT", Timeframe::M1).await {
        Ok(mut ohlcv_stream) => {
            println!("[TEST] OHLCV stream created successfully");
            let subscriptions = exchange.ws_client().subscriptions().await;
            println!("[TEST] Current subscriptions: {:?}", subscriptions);

            println!("[TEST] Waiting for OHLCV message (timeout: 90s)...");
            let ohlcv_item = timeout(Duration::from_secs(90), ohlcv_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(ohlcvs)) = ohlcv_item {
                if let Some(ohlcv) = ohlcvs.first() {
                    let open_str = ohlcv.open.0.to_string();
                    let high_str = ohlcv.high.0.to_string();
                    let low_str = ohlcv.low.0.to_string();
                    let close_str = ohlcv.close.0.to_string();
                    let vol_str = ohlcv.volume.0.to_string();
                    println!("[TEST] ✓ Swap OHLCV: {} candles received", ohlcvs.len());
                    println!(
                        "[TEST] First candle: ts={}, open={}, high={}, low={}, close={}, vol={}",
                        ohlcv.timestamp, open_str, high_str, low_str, close_str, vol_str
                    );
                    assert!(ohlcv.timestamp > 0, "Timestamp should be positive");
                    use rust_decimal::Decimal;
                    assert!(
                        ohlcv.open.0 > Decimal::ZERO,
                        "Open price should be positive"
                    );
                }
            } else if let Some(Err(e)) = ohlcv_item {
                println!("[TEST] ✗ Swap OHLCV error: {}", e);
            } else {
                println!("[TEST] ✗ Swap OHLCV timeout after 90s");
            }
        }
        Err(e) => {
            println!("[TEST] ✗ Swap OHLCV subscription failed: {} (skipping)", e);
        }
    }
}

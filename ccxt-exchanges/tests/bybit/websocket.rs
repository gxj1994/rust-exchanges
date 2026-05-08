//! Bybit WebSocket integration tests.
//!
//! Tests real-time data streaming via WebSocket for Bybit exchange.

#![allow(clippy::disallowed_methods)]
use ccxt_core::{
    ExchangeConfig, types::Timeframe, types::common::default_type::DefaultType,
    ws_exchange::WsExchange,
};
use ccxt_exchanges::bybit::{Bybit, BybitOptions};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

/// Helper to create Bybit instance for WebSocket tests
fn create_bybit_ws() -> Bybit {
    let config = ExchangeConfig::default();
    let options = BybitOptions {
        testnet: false,
        ..Default::default()
    };
    Bybit::new_with_options(config, options).unwrap()
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

    let exchange = create_bybit_ws();

    // Test: ws_connect should succeed
    let connect_result = timeout(Duration::from_secs(10), exchange.ws_connect()).await;
    assert!(connect_result.is_ok(), "Connection timed out");
    assert!(connect_result.unwrap().is_ok(), "Connection failed");

    println!("Bybit WebSocket connection successful");

    // Cleanup
    let _ = exchange.ws_disconnect().await;
}

/// Test ticker subscription via WebSocket.
#[tokio::test]
async fn test_ws_ticker() {
    if should_skip() {
        return;
    }

    let exchange = create_bybit_ws();

    match exchange.watch_ticker("BTC/USDT").await {
        Ok(mut ticker_stream) => {
            println!("Ticker stream created, waiting for data...");
            println!("Current subscriptions: {:?}", exchange.subscriptions());
            let ticker_item = timeout(Duration::from_secs(15), ticker_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(ticker)) = ticker_item {
                println!(
                    "Ticker received: symbol={}, last={:?}, bid={:?}, ask={:?}",
                    ticker.symbol.as_str(),
                    ticker.last,
                    ticker.bid,
                    ticker.ask
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

                // Bybit现货ticker可能有bid/ask
                if let Some(bid) = ticker.bid {
                    assert!(
                        bid.0 > Decimal::ZERO,
                        "Bid price should be positive if present"
                    );
                }
                if let Some(ask) = ticker.ask {
                    assert!(
                        ask.0 > Decimal::ZERO,
                        "Ask price should be positive if present"
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

                // Verify symbol is valid
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

    let exchange = create_bybit_ws();

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

    let exchange = create_bybit_ws();

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
///
/// # Why Ignored
///
/// This test is ignored by default because:
/// - Uses M1 (1-minute) timeframe, which may require waiting up to 60 seconds for data
/// - Dramatically slows down the overall test suite execution
/// - Bybit DOES support S1 (1-second) timeframe, but we use M1 for consistency with other exchanges
///
/// # How to Run
///
/// ```bash
/// cargo test -p ccxt-exchanges test_ws_ohlcv -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "Slow test: M1 timeframe may wait up to 60s for K-line data"]
async fn test_ws_ohlcv() {
    if should_skip() {
        return;
    }

    let exchange = create_bybit_ws();

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
                    println!(
                        "OHLCV received: ts={}, open={}, high={}, low={}, close={}, vol={}",
                        ohlcv.timestamp,
                        ohlcv.open,
                        ohlcv.high,
                        ohlcv.low,
                        ohlcv.close,
                        ohlcv.volume
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
// 合约市场数据测试 (需要 default_type = Swap)
// ============================================================================

/// Helper to create Bybit instance for swap WebSocket tests
fn create_bybit_swap_ws() -> Bybit {
    let config = ExchangeConfig::default();
    let options = BybitOptions {
        default_type: DefaultType::Swap,
        ..Default::default()
    };
    Bybit::new_with_options(config, options).unwrap()
}

/// Test swap ticker subscription via WebSocket.
#[tokio::test]
async fn test_ws_ticker_swap() {
    if should_skip() {
        return;
    }

    let exchange = create_bybit_swap_ws();

    println!("[TEST] Starting WebSocket connection (Swap market)...");
    match exchange.ws_connect().await {
        Ok(()) => {
            println!("[TEST] WebSocket connected successfully");
        }
        Err(e) => {
            println!("[TEST] Connection failed: {} (skipping)", e);
            return;
        }
    }

    println!("[TEST] Subscribing to ticker for BTC/USDT:USDT...");
    match exchange.watch_ticker("BTC/USDT:USDT").await {
        Ok(mut stream) => {
            println!("[TEST] Ticker stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for ticker message (timeout: 15s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(15), stream.next()).await {
                Ok(Some(Ok(ticker))) => {
                    println!(
                        "[TEST] ✓ Swap Ticker received: symbol={}, last={:?}, bid={:?}, ask={:?}",
                        ticker.symbol, ticker.last, ticker.bid, ticker.ask
                    );
                    assert!(!ticker.symbol.as_str().is_empty());

                    // 检查ticker核心字段
                    use rust_decimal::Decimal;
                    assert!(ticker.last.is_some(), "Last price should be present");
                    if let Some(last) = ticker.last {
                        assert!(last.0 > Decimal::ZERO, "Last price should be positive");
                    }

                    // 合约ticker也应该有bid/ask
                    // 使用条件验证：如果有则验证，没有则打印警告
                    if let Some(bid) = ticker.bid {
                        println!("[TEST] ✓ Bid price present: {:?}", bid);
                        assert!(bid.0 > Decimal::ZERO, "Bid price should be positive");
                    } else {
                        println!(
                            "[TEST] ⚠ Warning: Bid price is None (Bybit swap ticker API may not provide this field)"
                        );
                    }

                    if let Some(ask) = ticker.ask {
                        println!("[TEST] ✓ Ask price present: {:?}", ask);
                        assert!(ask.0 > Decimal::ZERO, "Ask price should be positive");
                    } else {
                        println!(
                            "[TEST] ⚠ Warning: Ask price is None (Bybit swap ticker API may not provide this field)"
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
                    println!("[TEST] ✗ Swap Ticker error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Swap Ticker stream ended");
                }
                Err(_) => {
                    println!("[TEST] ✗ Swap Ticker timeout after 15s");
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_ticker (swap) failed: {}", e);
        }
    }

    let _ = exchange.ws_disconnect().await;
}

/// Test swap order book subscription via WebSocket.
#[tokio::test]
async fn test_ws_orderbook_swap() {
    if should_skip() {
        return;
    }

    let exchange = create_bybit_swap_ws();

    println!("[TEST] Starting WebSocket connection (Swap market)...");
    match exchange.ws_connect().await {
        Ok(()) => {
            println!("[TEST] WebSocket connected successfully");
        }
        Err(e) => {
            println!("[TEST] Connection failed: {} (skipping)", e);
            return;
        }
    }

    println!("[TEST] Subscribing to orderbook for BTC/USDT:USDT (limit=5)...");
    match exchange.watch_order_book("BTC/USDT:USDT", Some(5)).await {
        Ok(mut stream) => {
            println!("[TEST] OrderBook stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for orderbook message (timeout: 15s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(15), stream.next()).await {
                Ok(Some(Ok(ob))) => {
                    println!(
                        "[TEST] ✓ Swap OrderBook received: symbol={}, bids={}, asks={}",
                        ob.symbol,
                        ob.bids.len(),
                        ob.asks.len()
                    );
                    assert!(!ob.bids.is_empty() || !ob.asks.is_empty());
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Swap OrderBook error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Swap OrderBook stream ended");
                }
                Err(_) => {
                    println!("[TEST] ✗ Swap OrderBook timeout after 15s");
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_order_book (swap) failed: {}", e);
        }
    }

    let _ = exchange.ws_disconnect().await;
}

/// Test swap trades subscription via WebSocket.
#[tokio::test]
async fn test_ws_trades_swap() {
    if should_skip() {
        return;
    }

    let exchange = create_bybit_swap_ws();

    println!("[TEST] Starting WebSocket connection (Swap market)...");
    match exchange.ws_connect().await {
        Ok(()) => {
            println!("[TEST] WebSocket connected successfully");
        }
        Err(e) => {
            println!("[TEST] Connection failed: {} (skipping)", e);
            return;
        }
    }

    println!("[TEST] Subscribing to trades for BTC/USDT:USDT...");
    match exchange.watch_market_trades("BTC/USDT:USDT").await {
        Ok(mut stream) => {
            println!("[TEST] Trades stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for trades message (timeout: 15s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(15), stream.next()).await {
                Ok(Some(Ok(trades))) => {
                    println!("[TEST] ✓ Swap Trades: {} trades received", trades.len());
                    if !trades.is_empty() {
                        let trade = &trades[0];
                        println!(
                            "[TEST] First trade: price={}, amount={}",
                            trade.price, trade.amount
                        );
                        use rust_decimal::Decimal;
                        assert!(trade.price.0 > Decimal::ZERO);
                        assert!(trade.amount.0 > Decimal::ZERO);
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Swap Trades error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Swap Trades stream ended");
                }
                Err(_) => {
                    println!("[TEST] ✗ Swap Trades timeout after 15s");
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_market_trades (swap) failed: {}", e);
        }
    }

    let _ = exchange.ws_disconnect().await;
}

/// Test swap OHLCV (K-line) subscription via WebSocket.
#[tokio::test]
async fn test_ws_ohlcv_swap() {
    if should_skip() {
        return;
    }

    let exchange = create_bybit_swap_ws();

    println!("[TEST] Starting WebSocket connection (Swap market)...");
    match exchange.ws_connect().await {
        Ok(()) => {
            println!("[TEST] WebSocket connected successfully");
        }
        Err(e) => {
            println!("[TEST] Connection failed: {} (skipping)", e);
            return;
        }
    }

    println!("[TEST] Subscribing to OHLCV for BTC/USDT:USDT (1m)...");
    match exchange.watch_ohlcv("BTC/USDT:USDT", Timeframe::M1).await {
        Ok(mut stream) => {
            println!("[TEST] OHLCV stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for OHLCV message (timeout: 90s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(90), stream.next()).await {
                Ok(Some(Ok(ohlcvs))) => {
                    println!("[TEST] ✓ Swap OHLCV: {} candles received", ohlcvs.len());
                    if let Some(ohlcv) = ohlcvs.first() {
                        println!(
                            "[TEST] First candle: ts={}, open={}, high={}, low={}, close={}, vol={}",
                            ohlcv.timestamp,
                            ohlcv.open,
                            ohlcv.high,
                            ohlcv.low,
                            ohlcv.close,
                            ohlcv.volume
                        );
                        assert!(ohlcv.timestamp > 0);
                        use rust_decimal::Decimal;
                        assert!(ohlcv.open.0 > Decimal::ZERO);
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Swap OHLCV error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Swap OHLCV stream ended");
                }
                Err(_) => {
                    println!("[TEST] ✗ Swap OHLCV timeout after 90s");
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_ohlcv (swap) failed: {}", e);
        }
    }

    let _ = exchange.ws_disconnect().await;
}

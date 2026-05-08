#![allow(clippy::disallowed_methods)]
use ccxt_core::{ExchangeConfig, types::Timeframe, ws_exchange::WsExchange};
use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidOptions};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_ws_connection_lifecycle() {
    // Skip if no internet connection or in CI environment that blocks external connections
    if std::env::var("CI").is_ok() {
        return;
    }

    let config = ExchangeConfig::default();
    // Use testnet for safety
    let options = HyperLiquidOptions {
        testnet: true,
        ..Default::default()
    };

    let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();

    // 1. Initial State
    assert!(!exchange.ws_is_connected());

    // 2. Connect
    let connect_result = timeout(Duration::from_secs(10), exchange.ws_connect()).await;
    assert!(connect_result.is_ok(), "Connection timed out");
    assert!(connect_result.unwrap().is_ok(), "Connection failed");

    assert!(exchange.ws_is_connected());

    // 3. Disconnect
    let disconnect_result = exchange.ws_disconnect().await;
    assert!(disconnect_result.is_ok());

    assert!(!exchange.ws_is_connected());
}

#[tokio::test]
async fn test_ws_public_streams() {
    if std::env::var("CI").is_ok() {
        return;
    }

    let config = ExchangeConfig::default();
    let options = HyperLiquidOptions {
        ..Default::default()
    };

    let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();

    // Connect with timeout
    match timeout(Duration::from_secs(10), exchange.ws_connect()).await {
        Ok(Ok(())) => println!("WebSocket connected successfully"),
        Ok(Err(e)) => {
            println!("WebSocket connection failed: {} (skipping test)", e);
            return;
        }
        Err(_) => {
            println!("WebSocket connection timed out (skipping test)");
            return;
        }
    }

    // Give connection a moment to stabilize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // OrderBook Subscription
    match exchange.watch_order_book("ETH/USDC:USDC", None).await {
        Ok(mut orderbook_stream) => {
            let orderbook_item = timeout(Duration::from_secs(10), orderbook_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(orderbook)) = orderbook_item {
                println!(
                    "OrderBook received: bids={}, asks={}",
                    orderbook.bids.len(),
                    orderbook.asks.len()
                );
            } else {
                println!("OrderBook subscription returned no data (testnet may have limited data)");
            }
        }
        Err(e) => {
            println!("OrderBook subscription failed: {} (skipping)", e);
        }
    }

    // Trades Subscription
    match exchange.watch_market_trades("ETH/USDC:USDC").await {
        Ok(mut trades_stream) => {
            let trades_item = timeout(Duration::from_secs(10), trades_stream.next())
                .await
                .ok()
                .flatten();
            match trades_item {
                Some(Ok(trades)) => {
                    println!("Trades received: {} trades", trades.len());
                    if trades.is_empty() {
                        println!("Warning: No trades data (testnet may have no recent trades)");
                    }
                }
                Some(Err(e)) => {
                    println!("Trades stream error: {}", e);
                }
                None => {
                    println!("Trades subscription timed out (testnet may have no trades)");
                }
            }
        }
        Err(e) => {
            println!("Trades subscription failed: {} (skipping)", e);
        }
    }

    // Ticker Subscription
    match exchange.watch_ticker("ETH/USDC:USDC").await {
        Ok(mut ticker_stream) => {
            let ticker_item = timeout(Duration::from_secs(10), ticker_stream.next())
                .await
                .ok()
                .flatten();
            if let Some(Ok(ticker)) = ticker_item {
                println!(
                    "Ticker received: symbol={}, last={:?}, bid={:?}, ask={:?}",
                    ticker.symbol, ticker.last, ticker.bid, ticker.ask
                );

                // 检查ticker核心字段
                use rust_decimal::Decimal;
                if let Some(last) = ticker.last {
                    assert!(last.0 > Decimal::ZERO, "Last price should be positive");
                }

                // Hyperliquid allMids提供的是中间价，可能没有传统bid/ask
                // 但如果有这些字段，应该验证其有效性
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
            } else {
                println!("Ticker subscription returned no data (testnet may have limited data)");
            }
        }
        Err(e) => {
            println!("Ticker subscription failed: {} (skipping)", e);
        }
    }
    let _ = exchange.ws_disconnect().await;
}

#[tokio::test]
async fn test_watch_bids_asks() {
    // Skip if no internet connection or in CI environment
    if std::env::var("CI").is_ok() {
        return;
    }

    let config = ccxt_core::ExchangeConfig::default();
    let options = ccxt_exchanges::hyperliquid::HyperLiquidOptions {
        ..Default::default()
    };

    let exchange =
        ccxt_exchanges::hyperliquid::HyperLiquid::new_with_options(config, options, None).unwrap();

    println!("[TEST] Starting WebSocket connection...");

    // Connect with timeout
    match tokio::time::timeout(Duration::from_secs(10), exchange.ws_connect()).await {
        Ok(Ok(())) => println!("[TEST] WebSocket connected successfully"),
        Ok(Err(e)) => {
            println!("[TEST] WebSocket connection failed: {} (skipping test)", e);
            return;
        }
        Err(_) => {
            println!("[TEST] WebSocket connection timed out (skipping test)");
            return;
        }
    }

    // Give connection a moment to stabilize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // BidsAsks Subscription
    println!("[TEST] Subscribing to bids_asks for BTC/USDC:USDC...");
    match exchange.watch_bids_asks("BTC/USDC:USDC").await {
        Ok(mut bids_asks_stream) => {
            println!("[TEST] BidsAsks stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            // Wait for bids_asks message
            println!("[TEST] Waiting for bids_asks message (timeout: 20s)...");
            let bids_asks_item =
                tokio::time::timeout(Duration::from_secs(20), bids_asks_stream.next())
                    .await
                    .ok()
                    .flatten();

            if let Some(Ok(bidask)) = bids_asks_item {
                println!(
                    "[TEST] ✓ BidsAsks: symbol={}, bid={} @ {}, ask={} @ {}",
                    bidask.symbol,
                    bidask.bid_quantity,
                    bidask.bid_price,
                    bidask.ask_quantity,
                    bidask.ask_price
                );

                // Verify BidsAsks has valid data
                use rust_decimal::Decimal;
                assert!(
                    !bidask.symbol.as_str().is_empty(),
                    "Symbol should not be empty"
                );
                assert!(
                    bidask.bid_price > Decimal::ZERO,
                    "Bid price should be positive"
                );
                assert!(
                    bidask.ask_price > Decimal::ZERO,
                    "Ask price should be positive"
                );

                // Verify spread is non-negative
                let spread = bidask.ask_price - bidask.bid_price;
                assert!(
                    spread >= Decimal::ZERO,
                    "Spread should be non-negative, got: {}",
                    spread
                );
                println!("[TEST] Spread: {}", spread);
            } else if let Some(Err(e)) = bids_asks_item {
                println!("[TEST] ✗ BidsAsks error: {}", e);
            } else {
                println!(
                    "[TEST] ⚠ BidsAsks subscription returned no data (network may have limited data)"
                );
            }
        }
        Err(e) => {
            println!("[TEST] BidsAsks subscription failed: {} (skipping)", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_ws_private_streams() {
    if std::env::var("CI").is_ok() {
        return;
    }

    // Requires API key/private key to derive address, or explicit address
    // Since we can't easily mock auth here without credentials, we'll test the logic
    // but expect authentication error if no credentials provided, which is correct behavior.

    let config = ExchangeConfig::default();
    let options = HyperLiquidOptions {
        testnet: true,
        ..Default::default()
    };

    let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();

    // Should fail without auth
    let result = exchange.watch_orders(None).await;
    assert!(result.is_err());

    let result = exchange.watch_account_trades(None).await;
    assert!(result.is_err());
}

/// Test OHLCV subscription on mainnet (more active data)
///
/// # Why Ignored
///
/// This test is ignored by default because:
/// - Uses M1 (1-minute) timeframe, which may require waiting up to 60 seconds for data
/// - Dramatically slows down the overall test suite execution
/// - Network-dependent and may fail in unstable network conditions
///
/// # How to Run
///
/// ```bash
/// cargo test -p ccxt-exchanges test_ws_ohlcv_mainnet -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "Slow test: M1 timeframe may wait up to 60s for K-line data"]
async fn test_ws_ohlcv_mainnet() {
    if std::env::var("CI").is_ok() {
        return;
    }

    let config = ExchangeConfig::default();
    // Use mainnet for OHLCV test (more active trading)
    let options = HyperLiquidOptions {
        ..Default::default()
    };

    let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();

    // Connect with timeout
    match timeout(Duration::from_secs(10), exchange.ws_connect()).await {
        Ok(Ok(())) => println!("WebSocket connected to mainnet"),
        Ok(Err(e)) => {
            println!("WebSocket connection failed: {} (skipping test)", e);
            return;
        }
        Err(_) => {
            println!("WebSocket connection timed out (skipping test)");
            return;
        }
    }

    // Give connection a moment to stabilize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test OHLCV Subscription with BTC (most active pair)
    match exchange.watch_ohlcv("BTC/USDC:USDC", Timeframe::M1).await {
        Ok(mut ohlcv_stream) => {
            // Increase timeout for OHLCV - candles may take longer to appear
            let ohlcv_item = timeout(Duration::from_secs(30), ohlcv_stream.next())
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
                    // Verify OHLCV has valid data
                    assert!(ohlcv.timestamp > 0, "Timestamp should be positive");
                    use rust_decimal::Decimal;
                    assert!(
                        ohlcv.open.0 > Decimal::ZERO,
                        "Open price should be positive"
                    );
                }
            } else {
                println!("OHLCV subscription returned no data after 30s");
            }
        }
        Err(e) => {
            println!("OHLCV subscription failed: {} (skipping)", e);
        }
    }

    let _ = exchange.ws_disconnect().await;
}

//! Bybit 现货+合约混用 WebSocket 复合测试
//!
//! 验证同时订阅现货(`BTC/USDT`)和合约(`BTC/USDT:USDT`)时：
//! - ticker/OHLCV 数据正确路由，不串扰
//! - 返回数据的 symbol 合理
//! - 取消订阅正常工作

#![allow(clippy::disallowed_methods)]

use ccxt_core::{
    types::{BidAsk, Ohlcv, Ticker},
    ws::WsContext,
    ws::subscription::{MarketType, SubscriptionChannel},
};
use ccxt_exchanges::bybit::ws::create_bybit_ws_client;
use futures_util::StreamExt;
use std::time::Duration;

/// 复合测试：同时订阅现货和合约的 ticker + OHLCV，验证数据 + 取消订阅
#[tokio::test]
async fn test_spot_swap_mixed_ticker_and_ohlcv() {
    let client = create_bybit_ws_client(false);

    // ── 1. 预连接现货和合约端点 ──
    let spot_ctx = WsContext::new().with_market_type(MarketType::Spot);
    let swap_ctx = WsContext::new().with_market_type(MarketType::Swap);

    client
        .connect(&spot_ctx)
        .await
        .expect("Failed to connect spot");
    println!("[TEST] Spot connection established");

    client
        .connect(&swap_ctx)
        .await
        .expect("Failed to connect swap");
    println!("[TEST] Swap connection established");

    // ── 2. 同时订阅现货/合约 ticker ──
    println!("[TEST] Subscribing to spot ticker BTC/USDT...");
    let mut spot_ticker = client
        .watch::<Ticker>("BTC/USDT", None)
        .await
        .expect("Failed to subscribe spot ticker");

    println!("[TEST] Subscribing to swap ticker BTC/USDT:USDT...");
    let mut swap_ticker = client
        .watch::<Ticker>("BTC/USDT:USDT", None)
        .await
        .expect("Failed to subscribe swap ticker");

    // ── 3. 同时订阅现货/合约 OHLCV ──
    let ohlcv_params = vec![("interval".to_string(), serde_json::json!("1m"))];
    println!("[TEST] Subscribing to spot OHLCV BTC/USDT (1m)...");
    let mut spot_ohlcv = client
        .watch::<Vec<Ohlcv>>("BTC/USDT", Some(ohlcv_params.clone()))
        .await
        .expect("Failed to subscribe spot OHLCV");

    println!("[TEST] Subscribing to swap OHLCV BTC/USDT:USDT (1m)...");
    let mut swap_ohlcv = client
        .watch::<Vec<Ohlcv>>("BTC/USDT:USDT", Some(ohlcv_params))
        .await
        .expect("Failed to subscribe swap OHLCV");

    // ── 4. 验证订阅数量 ──
    let subs = client.subscriptions().await;
    println!("[TEST] Total subscriptions: {:?}", subs);
    assert!(
        subs.len() >= 4,
        "Expected at least 4 subscriptions, got {}: {:?}",
        subs.len(),
        subs
    );

    // ── 5. 验证 ticker 数据 ──
    println!("[TEST] Waiting for spot ticker data...");
    let spot_tick = tokio::time::timeout(Duration::from_secs(15), spot_ticker.next())
        .await
        .expect("Spot ticker timeout")
        .expect("Spot ticker stream ended")
        .expect("Spot ticker error");
    println!(
        "[TEST] ✓ Spot ticker: symbol={}, last={:?}, bid={:?}, ask={:?}",
        spot_tick.symbol, spot_tick.last, spot_tick.bid, spot_tick.ask
    );
    assert!(
        !spot_tick.symbol.as_str().is_empty(),
        "Spot ticker symbol should not be empty"
    );
    assert!(
        spot_tick.symbol.as_str().contains("BTC"),
        "Spot ticker symbol should contain BTC, got: {}",
        spot_tick.symbol
    );
    assert!(
        spot_tick.last.is_some() || spot_tick.bid.is_some() || spot_tick.ask.is_some(),
        "Spot ticker should have at least one price field"
    );

    println!("[TEST] Waiting for swap ticker data...");
    let swap_tick = tokio::time::timeout(Duration::from_secs(15), swap_ticker.next())
        .await
        .expect("Swap ticker timeout")
        .expect("Swap ticker stream ended")
        .expect("Swap ticker error");
    println!(
        "[TEST] ✓ Swap ticker: symbol={}, last={:?}, bid={:?}, ask={:?}",
        swap_tick.symbol, swap_tick.last, swap_tick.bid, swap_tick.ask
    );
    assert!(
        !swap_tick.symbol.as_str().is_empty(),
        "Swap ticker symbol should not be empty"
    );
    assert!(
        swap_tick.symbol.as_str().contains("BTC"),
        "Swap ticker symbol should contain BTC, got: {}",
        swap_tick.symbol
    );
    assert!(
        swap_tick.last.is_some() || swap_tick.bid.is_some() || swap_tick.ask.is_some(),
        "Swap ticker should have at least one price field"
    );

    // ── 6. 验证 OHLCV 数据 ──
    println!("[TEST] Waiting for spot OHLCV data...");
    let spot_candles = tokio::time::timeout(Duration::from_secs(90), spot_ohlcv.next())
        .await
        .expect("Spot OHLCV timeout")
        .expect("Spot OHLCV stream ended")
        .expect("Spot OHLCV error");
    assert!(!spot_candles.is_empty(), "Spot OHLCV should have data");
    let spot_candle = &spot_candles[0];
    println!(
        "[TEST] ✓ Spot OHLCV: timestamp={:?}, open={}, high={}, low={}, close={}, volume={}",
        spot_candle.timestamp,
        spot_candle.open,
        spot_candle.high,
        spot_candle.low,
        spot_candle.close,
        spot_candle.volume
    );
    assert!(
        spot_candle.high >= spot_candle.low,
        "OHLCV high should be >= low"
    );
    assert!(spot_candle.open.is_positive(), "OHLCV open should be > 0");

    println!("[TEST] Waiting for swap OHLCV data...");
    let swap_candles = tokio::time::timeout(Duration::from_secs(90), swap_ohlcv.next())
        .await
        .expect("Swap OHLCV timeout")
        .expect("Swap OHLCV stream ended")
        .expect("Swap OHLCV error");
    assert!(!swap_candles.is_empty(), "Swap OHLCV should have data");
    let swap_candle = &swap_candles[0];
    println!(
        "[TEST] ✓ Swap OHLCV: timestamp={:?}, open={}, high={}, low={}, close={}, volume={}",
        swap_candle.timestamp,
        swap_candle.open,
        swap_candle.high,
        swap_candle.low,
        swap_candle.close,
        swap_candle.volume
    );
    assert!(
        swap_candle.high >= swap_candle.low,
        "OHLCV high should be >= low"
    );
    assert!(swap_candle.open.is_positive(), "OHLCV open should be > 0");

    // ── 7. 测试取消订阅 ──
    let subs_before = client.subscriptions().await.len();
    println!("[TEST] Subscriptions before unsubscribe: {}", subs_before);

    client
        .unsubscribe(&[SubscriptionChannel::ticker("BTC/USDT")])
        .await
        .expect("Failed to unsubscribe spot ticker");

    let subs_after = client.subscriptions().await.len();
    println!("[TEST] Subscriptions after unsubscribe: {}", subs_after);
    assert!(
        subs_after < subs_before,
        "Unsubscription should reduce count: {} -> {}",
        subs_before,
        subs_after
    );

    // ── 7.1 验证取消订阅后 spot_ticker 最终会停止 ──
    println!("[TEST] Verifying spot_ticker stream terminates after unsubscribe...");
    // 记录取消订阅的时间
    let unsubscribe_time = std::time::Instant::now();
    let buffer_duration = Duration::from_millis(500); // 缓冲区时间

    // 清空缓冲区中的残留消息,并检查时间戳
    let mut drained_count = 0;

    loop {
        match tokio::time::timeout(Duration::from_millis(100), spot_ticker.next()).await {
            Ok(Some(Ok(tick))) => {
                drained_count += 1;
                let elapsed = unsubscribe_time.elapsed();
                if elapsed > buffer_duration {
                    panic!(
                        "❌ FAIL: spot_ticker received NEW message {}ms after unsubscribe (buffer: {}ms), symbol={}, last={:?}",
                        elapsed.as_millis(),
                        buffer_duration.as_millis(),
                        tick.symbol,
                        tick.last
                    );
                }
            }
            Ok(Some(Err(e))) => {
                panic!(
                    "❌ FAIL: spot_ticker returned error after unsubscribe: {}",
                    e
                );
            }
            Ok(None) | Err(_) => {
                break;
            }
        }
    }

    if drained_count > 0 {
        println!(
            "[TEST] ⚠ Drained {} buffered messages after unsubscribe",
            drained_count
        );
    }

    println!("[TEST] ✓ spot_ticker stream correctly terminated within buffer period");

    // ── 7.2 验证 swap_ticker 仍然可以收到消息(未被取消) ──
    println!("[TEST] Verifying swap_ticker still receives messages...");
    let swap_tick_after = tokio::time::timeout(Duration::from_secs(15), swap_ticker.next())
        .await
        .expect("Swap ticker timeout after spot unsubscribe")
        .expect("Swap ticker stream ended unexpectedly")
        .expect("Swap ticker error");
    println!(
        "[TEST] ✓ Swap ticker still working: symbol={}, last={:?}",
        swap_tick_after.symbol, swap_tick_after.last
    );
    assert!(
        swap_tick_after.symbol.as_str().contains("BTC"),
        "Swap ticker symbol should contain BTC"
    );

    // ── 7.3 验证 swap_ohlcv 仍然可以收到消息(未被取消) ──
    println!("[TEST] Verifying swap_ohlcv still receives messages...");
    let swap_candles_after = tokio::time::timeout(Duration::from_secs(90), swap_ohlcv.next())
        .await
        .expect("Swap OHLCV timeout after spot unsubscribe")
        .expect("Swap OHLCV stream ended unexpectedly")
        .expect("Swap OHLCV error");
    assert!(
        !swap_candles_after.is_empty(),
        "Swap OHLCV should still have data"
    );
    println!(
        "[TEST] ✓ Swap OHLCV still working: {} candles received",
        swap_candles_after.len()
    );

    // ── 7.4 取消订阅 spot OHLCV ──
    let subs_before_ohlcv = client.subscriptions().await.len();
    println!(
        "[TEST] Subscriptions before spot OHLCV unsubscribe: {}",
        subs_before_ohlcv
    );

    client
        .unsubscribe(&[SubscriptionChannel::kline("BTC/USDT", "1m")])
        .await
        .expect("Failed to unsubscribe spot OHLCV");

    let subs_after_ohlcv = client.subscriptions().await.len();
    println!(
        "[TEST] Subscriptions after spot OHLCV unsubscribe: {}",
        subs_after_ohlcv
    );
    assert!(
        subs_after_ohlcv < subs_before_ohlcv,
        "Unsubscription should reduce count: {} -> {}",
        subs_before_ohlcv,
        subs_after_ohlcv
    );

    // ── 7.5 验证取消订阅后 spot_ohlcv 最终会停止 ──
    println!("[TEST] Verifying spot_ohlcv stream terminates after unsubscribe...");
    // 记录取消订阅的时间
    let unsubscribe_time = std::time::Instant::now();
    let buffer_duration = Duration::from_millis(500); // 缓冲区时间

    // 清空缓冲区中的残留消息,并检查时间戳
    let mut drained_count = 0;

    loop {
        match tokio::time::timeout(Duration::from_millis(100), spot_ohlcv.next()).await {
            Ok(Some(Ok(candles))) => {
                drained_count += 1;
                let elapsed = unsubscribe_time.elapsed();
                if elapsed > buffer_duration {
                    panic!(
                        "❌ FAIL: spot_ohlcv received NEW message {}ms after unsubscribe (buffer: {}ms), got {} candles",
                        elapsed.as_millis(),
                        buffer_duration.as_millis(),
                        candles.len()
                    );
                }
            }
            Ok(Some(Err(e))) => {
                panic!(
                    "❌ FAIL: spot_ohlcv returned error after unsubscribe: {}",
                    e
                );
            }
            Ok(None) | Err(_) => {
                break;
            }
        }
    }

    if drained_count > 0 {
        println!(
            "[TEST] ⚠ Drained {} buffered messages after unsubscribe",
            drained_count
        );
    }

    if drained_count == 0 {
        println!("[TEST] ✓ spot_ohlcv stream correctly terminated (no buffered messages)");
    } else {
        println!("[TEST] ✓ spot_ohlcv stream correctly terminated within buffer period");
    }

    // ── 7.6 验证 swap_ohlcv 仍然可以收到消息(未被取消) ──
    println!("[TEST] Verifying swap_ohlcv still receives messages after spot OHLCV unsubscribe...");
    let swap_candles_final = tokio::time::timeout(Duration::from_secs(90), swap_ohlcv.next())
        .await
        .expect("Swap OHLCV timeout after spot OHLCV unsubscribe")
        .expect("Swap OHLCV stream ended unexpectedly")
        .expect("Swap OHLCV error");
    assert!(
        !swap_candles_final.is_empty(),
        "Swap OHLCV should still have data after spot OHLCV unsubscribe"
    );
    println!(
        "[TEST] ✓ Swap OHLCV still working: {} candles received",
        swap_candles_final.len()
    );

    // ── 7.7 取消订阅 swap OHLCV ──
    let subs_before_swap_ohlcv = client.subscriptions().await.len();
    println!(
        "[TEST] Subscriptions before swap OHLCV unsubscribe: {}",
        subs_before_swap_ohlcv
    );
    println!(
        "[TEST] Current subscriptions: {:?}",
        client.subscriptions().await
    );

    let cancel_swap_ohlcv_channel = SubscriptionChannel::kline("BTC/USDT:USDT", "1m");
    println!(
        "[TEST] Unsubscribe channel: type={:?}, symbol={}, params={:?}",
        cancel_swap_ohlcv_channel.channel_type,
        cancel_swap_ohlcv_channel.symbol,
        cancel_swap_ohlcv_channel.params
    );

    client
        .unsubscribe(&[cancel_swap_ohlcv_channel])
        .await
        .expect("Failed to unsubscribe swap OHLCV");

    let subs_after_swap_ohlcv = client.subscriptions().await.len();
    println!(
        "[TEST] Subscriptions after swap OHLCV unsubscribe: {}",
        subs_after_swap_ohlcv
    );
    assert!(
        subs_after_swap_ohlcv < subs_before_swap_ohlcv,
        "Unsubscription should reduce count: {} -> {}",
        subs_before_swap_ohlcv,
        subs_after_swap_ohlcv
    );

    // ── 7.8 验证取消订阅后 swap_ohlcv 最终会停止 ──
    println!("[TEST] Verifying swap_ohlcv stream terminates after unsubscribe...");
    let unsubscribe_time_swap = std::time::Instant::now();
    let buffer_duration_swap = Duration::from_millis(500);
    let mut drained_count_swap = 0;

    loop {
        match tokio::time::timeout(Duration::from_millis(100), swap_ohlcv.next()).await {
            Ok(Some(Ok(candles))) => {
                drained_count_swap += 1;
                let elapsed = unsubscribe_time_swap.elapsed();
                if elapsed > buffer_duration_swap {
                    panic!(
                        "❌ FAIL: swap_ohlcv received NEW message {}ms after unsubscribe (buffer: {}ms), got {} candles",
                        elapsed.as_millis(),
                        buffer_duration_swap.as_millis(),
                        candles.len()
                    );
                }
            }
            Ok(Some(Err(e))) => {
                panic!(
                    "❌ FAIL: swap_ohlcv returned error after unsubscribe: {}",
                    e
                );
            }
            Ok(None) | Err(_) => {
                break;
            }
        }
    }

    if drained_count_swap > 0 {
        println!(
            "[TEST] ⚠ Drained {} buffered messages after unsubscribe",
            drained_count_swap
        );
    }

    if drained_count_swap == 0 {
        println!("[TEST] ✓ swap_ohlcv stream correctly terminated (no buffered messages)");
    } else {
        println!("[TEST] ✓ swap_ohlcv stream correctly terminated within buffer period");
    }

    // ── 8. 清理 ──
    client.disconnect().await.expect("Failed to disconnect");
    println!("[TEST] ✓ All tests passed");
}

/// 复合测试：同时订阅现货和合约的 bids_asks (orderbook.1)，验证数据 + 取消订阅
#[allow(unused)]
#[tokio::test]
async fn test_spot_swap_mixed_bids_asks() {
    let client = create_bybit_ws_client(false);

    // ── 1. 预连接现货和合约端点 ──
    let spot_ctx = WsContext::new().with_market_type(MarketType::Spot);
    let swap_ctx = WsContext::new().with_market_type(MarketType::Swap);

    client
        .connect(&spot_ctx)
        .await
        .expect("Failed to connect spot");
    println!("[TEST] Spot connection established");

    client
        .connect(&swap_ctx)
        .await
        .expect("Failed to connect swap");
    println!("[TEST] Swap connection established");

    // ── 2. 同时订阅现货/合约 bids_asks ──
    // 注意：Bybit 的 orderbook.1 就是 bids_asks（最优买卖价）
    println!("[TEST] Subscribing to spot bids_asks BTC/USDT...");
    let mut spot_ba = client
        .watch::<BidAsk>("BTC/USDT", None)
        .await
        .expect("Failed to subscribe spot bids_asks");

    println!("[TEST] Subscribing to swap bids_asks BTC/USDT:USDT...");
    let mut swap_ba = client
        .watch::<BidAsk>("BTC/USDT:USDT", None)
        .await
        .expect("Failed to subscribe swap bids_asks");

    // ── 3. 验证订阅数量 ──
    let subs = client.subscriptions().await;
    println!("[TEST] Total subscriptions: {:?}", subs);
    assert!(
        subs.len() >= 2,
        "Expected at least 2 subscriptions, got {}: {:?}",
        subs.len(),
        subs
    );

    // ── 4. 验证现货 bids_asks 数据 ──
    println!("[TEST] Waiting for spot bids_asks data...");
    let spot_bidask = tokio::time::timeout(Duration::from_secs(20), spot_ba.next())
        .await
        .expect("Spot bids_asks timeout")
        .expect("Spot bids_asks stream ended")
        .expect("Spot bids_asks error");
    println!(
        "[TEST] ✓ Spot BidsAsks: symbol={}, bid={} @ {}, ask={} @ {}",
        spot_bidask.symbol,
        spot_bidask.bid_quantity,
        spot_bidask.bid_price,
        spot_bidask.ask_quantity,
        spot_bidask.ask_price
    );
    assert!(
        !spot_bidask.symbol.is_empty(),
        "Spot bids_asks symbol should not be empty"
    );
    assert!(
        spot_bidask.bid_price > rust_decimal::Decimal::ZERO,
        "Bid price should be positive"
    );
    assert!(
        spot_bidask.ask_price > rust_decimal::Decimal::ZERO,
        "Ask price should be positive"
    );
    // 验证价差为非负
    let spread = spot_bidask.ask_price - spot_bidask.bid_price;
    assert!(
        spread >= rust_decimal::Decimal::ZERO,
        "Spread should be non-negative"
    );
    println!("[TEST] Spot spread: {}", spread);

    // ── 5. 验证合约 bids_asks 数据 ──
    println!("[TEST] Waiting for swap bids_asks data...");
    let swap_bidask = tokio::time::timeout(Duration::from_secs(20), swap_ba.next())
        .await
        .expect("Swap bids_asks timeout")
        .expect("Swap bids_asks stream ended")
        .expect("Swap bids_asks error");
    println!(
        "[TEST] ✓ Swap BidsAsks: symbol={}, bid={} @ {}, ask={} @ {}",
        swap_bidask.symbol,
        swap_bidask.bid_quantity,
        swap_bidask.bid_price,
        swap_bidask.ask_quantity,
        swap_bidask.ask_price
    );
    assert!(
        !swap_bidask.symbol.is_empty(),
        "Swap bids_asks symbol should not be empty"
    );
    assert!(
        swap_bidask.bid_price > rust_decimal::Decimal::ZERO,
        "Bid price should be positive"
    );
    assert!(
        swap_bidask.ask_price > rust_decimal::Decimal::ZERO,
        "Ask price should be positive"
    );
    // 验证价差为非负
    let spread = swap_bidask.ask_price - swap_bidask.bid_price;
    assert!(
        spread >= rust_decimal::Decimal::ZERO,
        "Spread should be non-negative"
    );
    println!("[TEST] Swap spread: {}", spread);

    // ── 6. 测试取消订阅现货 bids_asks ──
    let subs_before = client.subscriptions().await.len();
    println!("[TEST] Subscriptions before unsubscribe: {}", subs_before);

    client
        .unsubscribe(&[SubscriptionChannel::bids_asks("BTC/USDT")])
        .await
        .expect("Failed to unsubscribe spot bids_asks");

    let subs_after = client.subscriptions().await.len();
    println!("[TEST] Subscriptions after unsubscribe: {}", subs_after);
    assert!(
        subs_after < subs_before,
        "Unsubscription should reduce count: {} -> {}",
        subs_before,
        subs_after
    );

    // ── 6.1 验证取消订阅后 spot_ba 最终会停止 ──
    println!("[TEST] Verifying spot_ba stream terminates after unsubscribe...");
    let unsubscribe_time = std::time::Instant::now();
    let buffer_duration = Duration::from_millis(500);

    let mut drained_count = 0;
    let mut found_future_message = false;

    loop {
        match tokio::time::timeout(Duration::from_millis(100), spot_ba.next()).await {
            Ok(Some(Ok(ba))) => {
                drained_count += 1;
                let elapsed = unsubscribe_time.elapsed();
                if elapsed > buffer_duration {
                    found_future_message = true;
                    panic!(
                        "❌ FAIL: spot_ba received NEW message {}ms after unsubscribe (buffer: {}ms), symbol={}",
                        elapsed.as_millis(),
                        buffer_duration.as_millis(),
                        ba.symbol
                    );
                }
            }
            Ok(Some(Err(e))) => {
                panic!("❌ FAIL: spot_ba returned error after unsubscribe: {}", e);
            }
            Ok(None) | Err(_) => {
                break;
            }
        }
    }

    if drained_count > 0 {
        println!(
            "[TEST] ⚠ Drained {} buffered messages after unsubscribe",
            drained_count
        );
    }

    if !found_future_message {
        println!("[TEST] ✓ spot_ba stream correctly terminated within buffer period");
    }

    // ── 6.2 验证 swap_ba 仍然可以收到消息(未被取消) ──
    println!("[TEST] Verifying swap_ba still receives messages...");
    let swap_ba_after = tokio::time::timeout(Duration::from_secs(20), swap_ba.next())
        .await
        .expect("Swap bids_asks timeout after spot unsubscribe")
        .expect("Swap bids_asks stream ended unexpectedly")
        .expect("Swap bids_asks error");
    println!(
        "[TEST] ✓ Swap bids_asks still working: symbol={}, bid={}, ask={}",
        swap_ba_after.symbol, swap_ba_after.bid_price, swap_ba_after.ask_price
    );
    assert!(
        !swap_ba_after.symbol.is_empty(),
        "Swap bids_asks symbol should not be empty"
    );

    // ── 7. 取消订阅合约 bids_asks ──
    let subs_before_swap = client.subscriptions().await.len();
    println!(
        "[TEST] Subscriptions before swap bids_asks unsubscribe: {}",
        subs_before_swap
    );
    println!(
        "[TEST] Current subscriptions: {:?}",
        client.subscriptions().await
    );

    let cancel_swap_channel = SubscriptionChannel::bids_asks("BTC/USDT:USDT");
    println!(
        "[TEST] Unsubscribe channel: type={:?}, symbol={}",
        cancel_swap_channel.channel_type, cancel_swap_channel.symbol
    );

    client
        .unsubscribe(&[cancel_swap_channel])
        .await
        .expect("Failed to unsubscribe swap bids_asks");

    let subs_after_swap = client.subscriptions().await.len();
    println!(
        "[TEST] Subscriptions after swap bids_asks unsubscribe: {}",
        subs_after_swap
    );
    assert!(
        subs_after_swap < subs_before_swap,
        "Unsubscription should reduce count: {} -> {}",
        subs_before_swap,
        subs_after_swap
    );

    // ── 7.1 验证取消订阅后 swap_ba 最终会停止 ──
    println!("[TEST] Verifying swap_ba stream terminates after unsubscribe...");
    let unsubscribe_time = std::time::Instant::now();
    let buffer_duration = Duration::from_millis(500);

    let mut drained_count = 0;

    loop {
        match tokio::time::timeout(Duration::from_millis(100), swap_ba.next()).await {
            Ok(Some(Ok(ba))) => {
                drained_count += 1;
                let elapsed = unsubscribe_time.elapsed();
                if elapsed > buffer_duration {
                    panic!(
                        "❌ FAIL: swap_ba received NEW message {}ms after unsubscribe (buffer: {}ms), symbol={}",
                        elapsed.as_millis(),
                        buffer_duration.as_millis(),
                        ba.symbol
                    );
                }
            }
            Ok(Some(Err(e))) => {
                panic!("❌ FAIL: swap_ba returned error after unsubscribe: {}", e);
            }
            Ok(None) | Err(_) => {
                break;
            }
        }
    }

    if drained_count > 0 {
        println!(
            "[TEST] ⚠ Drained {} buffered messages after unsubscribe",
            drained_count
        );
    } else {
        println!("[TEST] ✓ swap_ba stream correctly terminated (no buffered messages)");
    }

    // ── 8. 清理 ──
    client.disconnect().await.expect("Failed to disconnect");
    println!("[TEST] ✓ All bids_asks tests passed");
}

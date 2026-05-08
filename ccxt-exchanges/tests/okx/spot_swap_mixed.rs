//! OKX 现货+合约混用 WebSocket 复合测试
//!
//! 验证同时订阅现货(`BTC/USDT`)和合约(`BTC/USDT:USDT`)时：
//! - ticker/OHLCV 数据正确路由，不串扰
//! - 返回数据的 symbol 合理
//! - 取消订阅正常工作

#![allow(clippy::disallowed_methods)]

use ccxt_core::{
    types::{Ohlcv, Ticker},
    ws::WsContext,
    ws::subscription::{MarketType, SubscriptionChannel},
};
use ccxt_exchanges::okx::ws::create_okx_ws_client;
use futures_util::StreamExt;
use std::time::Duration;

/// 复合测试：同时订阅现货和合约的 ticker + OHLCV，验证数据 + 取消订阅
#[tokio::test]
async fn test_spot_swap_mixed_ticker_and_ohlcv() {
    let client = create_okx_ws_client(false);

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
    let unsubscribe_time = std::time::Instant::now();
    let buffer_duration = Duration::from_millis(500);
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

    println!("[TEST] ✓ spot_ohlcv stream correctly terminated within buffer period");

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

    let cancel_swap_ohlcv_channel = SubscriptionChannel::kline("BTC/USDT:USDT", "1m");
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

    println!("[TEST] ✓ swap_ohlcv stream correctly terminated within buffer period");

    // ── 8. 清理 ──
    client.disconnect().await.expect("Failed to disconnect");
    println!("[TEST] ✓ All tests passed");
}

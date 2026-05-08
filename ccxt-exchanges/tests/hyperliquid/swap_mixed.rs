//! HyperLiquid 合约 WebSocket 混用测试
//!
//! 验证同时订阅合约的 bids_asks 和 OHLCV 时：
//! - 数据正确路由，不串扰
//! - 返回数据的 symbol 合理
//! - 取消订阅正常工作
//!
//! 注意：HyperLiquid 只支持合约（Swap），不支持现货

#![allow(clippy::disallowed_methods)]

use ccxt_core::{types::Timeframe, ws::subscription::SubscriptionChannel, ws_exchange::WsExchange};
use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidOptions};
use futures_util::StreamExt;
use std::time::Duration;

/// 复合测试：同时订阅合约的 bids_asks + OHLCV，验证数据 + 取消订阅
#[tokio::test]
async fn test_swap_mixed_bids_asks_and_ohlcv() {
    // Skip if in CI environment
    if std::env::var("CI").is_ok() {
        return;
    }

    let config = ccxt_core::ExchangeConfig::default();
    let options = HyperLiquidOptions {
        ..Default::default()
    };

    let exchange = HyperLiquid::new_with_options(config, options, None).unwrap();

    // ── 1. 连接 WebSocket ──
    println!("[TEST] Connecting to HyperLiquid WebSocket...");
    match tokio::time::timeout(Duration::from_secs(10), exchange.ws_connect()).await {
        Ok(Ok(())) => println!("[TEST] WebSocket connection established"),
        Ok(Err(e)) => {
            println!("[TEST] Connection failed: {} (skipping)", e);
            return;
        }
        Err(_) => {
            println!("[TEST] Connection timed out (skipping)");
            return;
        }
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // ── 2. 同时订阅 bids_asks 和 OHLCV ──
    println!("[TEST] Subscribing to bids_asks BTC/USDC:USDC...");
    let mut ba_stream = exchange
        .watch_bids_asks("BTC/USDC:USDC")
        .await
        .expect("Failed to subscribe bids_asks");

    println!("[TEST] Subscribing to OHLCV BTC/USDC:USDC (1m)...");
    let mut ohlcv_stream = exchange
        .watch_ohlcv("BTC/USDC:USDC", Timeframe::M1)
        .await
        .expect("Failed to subscribe OHLCV");

    // ── 3. 验证 bids_asks 数据 ──
    println!("[TEST] Waiting for bids_asks data...");
    let ba_result = tokio::time::timeout(Duration::from_secs(20), ba_stream.next())
        .await
        .expect("BidsAsks timeout")
        .expect("BidsAsks stream ended");

    match ba_result {
        Ok(ba) => {
            println!(
                "[TEST] ✓ BidsAsks: symbol={}, bid={} @ {}, ask={} @ {}",
                ba.symbol, ba.bid_quantity, ba.bid_price, ba.ask_quantity, ba.ask_price
            );
            assert!(!ba.symbol.as_str().is_empty(), "Symbol should not be empty");
            assert!(
                ba.bid_price > rust_decimal::Decimal::ZERO,
                "Bid price should be positive"
            );
            assert!(
                ba.ask_price > rust_decimal::Decimal::ZERO,
                "Ask price should be positive"
            );

            let spread = ba.ask_price - ba.bid_price;
            assert!(
                spread >= rust_decimal::Decimal::ZERO,
                "Spread should be non-negative"
            );
            println!("[TEST] BidsAsks spread: {}", spread);
        }
        Err(e) => {
            panic!("BidsAsks error: {}", e);
        }
    }

    // ── 4. 验证 OHLCV 数据 ──
    println!("[TEST] Waiting for OHLCV data...");
    let ohlcv_result = tokio::time::timeout(Duration::from_secs(30), ohlcv_stream.next())
        .await
        .expect("OHLCV timeout")
        .expect("OHLCV stream ended");

    match ohlcv_result {
        Ok(ohlcvs) => {
            if let Some(ohlcv) = ohlcvs.first() {
                println!(
                    "[TEST] ✓ OHLCV: ts={}, open={}, high={}, low={}, close={}, vol={}",
                    ohlcv.timestamp, ohlcv.open, ohlcv.high, ohlcv.low, ohlcv.close, ohlcv.volume
                );
                assert!(ohlcv.timestamp > 0, "Timestamp should be positive");
                assert!(
                    ohlcv.open.0 > rust_decimal::Decimal::ZERO,
                    "Open price should be positive"
                );
            } else {
                println!("[TEST] ⚠ OHLCV array is empty (network may have limited data)");
            }
        }
        Err(e) => {
            panic!("OHLCV error: {}", e);
        }
    }

    // ── 5. 测试取消订阅 bids_asks ──
    let subs_before = exchange.subscriptions().len();
    println!("[TEST] Subscriptions before unsubscribe: {}", subs_before);

    let cancel_ba_channel = SubscriptionChannel::bids_asks("BTC/USDC:USDC");
    println!(
        "[TEST] Unsubscribe channel: type={:?}, symbol={}",
        cancel_ba_channel.channel_type, cancel_ba_channel.symbol
    );

    exchange
        .ws_client()
        .unsubscribe(&[cancel_ba_channel])
        .await
        .expect("Failed to unsubscribe bids_asks");

    let subs_after = exchange.subscriptions().len();
    println!("[TEST] Subscriptions after unsubscribe: {}", subs_after);
    assert!(
        subs_after < subs_before,
        "Unsubscription should reduce count: {} -> {}",
        subs_before,
        subs_after
    );

    // ── 5.1 验证取消订阅后 ba_stream 最终会停止 ──
    println!("[TEST] Verifying ba_stream terminates after unsubscribe...");
    let unsubscribe_time = std::time::Instant::now();
    let buffer_duration = Duration::from_millis(500);

    let mut drained_count = 0;
    loop {
        match tokio::time::timeout(Duration::from_millis(100), ba_stream.next()).await {
            Ok(Some(Ok(ba))) => {
                drained_count += 1;
                let elapsed = unsubscribe_time.elapsed();
                if elapsed > buffer_duration {
                    panic!(
                        "❌ FAIL: ba_stream received NEW message {}ms after unsubscribe (buffer: {}ms), symbol={}",
                        elapsed.as_millis(),
                        buffer_duration.as_millis(),
                        ba.symbol
                    );
                }
            }
            Ok(Some(Err(e))) => {
                panic!("❌ FAIL: ba_stream returned error after unsubscribe: {}", e);
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
        println!("[TEST] ✓ ba_stream stream correctly terminated (no buffered messages)");
    }

    // ── 5.2 验证 ohlcv_stream 仍然可以收到消息(未被取消) ──
    println!("[TEST] Verifying ohlcv_stream still receives messages...");
    let ohlcv_after = tokio::time::timeout(Duration::from_secs(30), ohlcv_stream.next())
        .await
        .expect("OHLCV timeout after bids_asks unsubscribe")
        .expect("OHLCV stream ended unexpectedly");

    match ohlcv_after {
        Ok(ohlcvs) => {
            if let Some(ohlcv) = ohlcvs.first() {
                println!(
                    "[TEST] ✓ OHLCV still working: ts={}, close={}",
                    ohlcv.timestamp, ohlcv.close
                );
            } else {
                println!("[TEST] ⚠ OHLCV returned empty array (network limitation)");
            }
        }
        Err(e) => {
            panic!("❌ FAIL: OHLCV error after bids_asks unsubscribe: {}", e);
        }
    }

    // ── 6. 取消订阅 OHLCV ──
    let subs_before_ohlcv = exchange.subscriptions().len();
    println!(
        "[TEST] Subscriptions before OHLCV unsubscribe: {}",
        subs_before_ohlcv
    );

    let cancel_ohlcv_channel = SubscriptionChannel::kline("BTC/USDC:USDC", "1m");
    println!(
        "[TEST] Unsubscribe channel: type={:?}, symbol={}",
        cancel_ohlcv_channel.channel_type, cancel_ohlcv_channel.symbol
    );

    exchange
        .ws_client()
        .unsubscribe(&[cancel_ohlcv_channel])
        .await
        .expect("Failed to unsubscribe OHLCV");

    let subs_after_ohlcv = exchange.subscriptions().len();
    println!(
        "[TEST] Subscriptions after OHLCV unsubscribe: {}",
        subs_after_ohlcv
    );
    assert!(
        subs_after_ohlcv < subs_before_ohlcv,
        "Unsubscription should reduce count: {} -> {}",
        subs_before_ohlcv,
        subs_after_ohlcv
    );

    // ── 6.1 验证取消订阅后 ohlcv_stream 最终会停止 ──
    println!("[TEST] Verifying ohlcv_stream terminates after unsubscribe...");
    let unsubscribe_time = std::time::Instant::now();
    let buffer_duration = Duration::from_millis(500);

    let mut drained_count = 0;
    loop {
        match tokio::time::timeout(Duration::from_millis(100), ohlcv_stream.next()).await {
            Ok(Some(Ok(ohlcvs))) => {
                drained_count += 1;
                let elapsed = unsubscribe_time.elapsed();
                if elapsed > buffer_duration {
                    panic!(
                        "❌ FAIL: ohlcv_stream received NEW message {}ms after unsubscribe (buffer: {}ms), got {} candles",
                        elapsed.as_millis(),
                        buffer_duration.as_millis(),
                        ohlcvs.len()
                    );
                }
            }
            Ok(Some(Err(e))) => {
                panic!(
                    "❌ FAIL: ohlcv_stream returned error after unsubscribe: {}",
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
    } else {
        println!("[TEST] ✓ ohlcv_stream correctly terminated (no buffered messages)");
    }

    // ── 7. 断开连接 ──
    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;

    println!("[TEST] ✓ All mixed tests passed");
}

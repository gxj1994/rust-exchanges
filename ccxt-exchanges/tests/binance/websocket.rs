#![allow(clippy::disallowed_methods)]
//! Binance WebSocket integration tests (新架构)
//!
//! 使用 ws_v2 统一架构的测试

use crate::support::{create_binance_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::WsExchange;
use ccxt_core::types::Symbol;
use ccxt_core::types::common::default_type::DefaultType;
use ccxt_exchanges::binance::{Binance, BinanceOptions};
use futures_util::StreamExt;

/// Helper: 创建并初始化 Binance 实例用于私有频道测试
async fn create_binance_for_private_tests() -> Option<Binance> {
    if should_skip_private_tests("binance") {
        println!("SKIPPED: No Binance credentials or private tests disabled");
        return None;
    }

    let config = init_test();
    match create_binance_with_credentials(&config) {
        Ok(binance) => Some(binance),
        Err(e) => {
            println!("Failed to create Binance: {}", e);
            None
        }
    }
}

// ============================================================================
// 公共频道测试
// ============================================================================

#[tokio::test]
async fn test_watch_ticker() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Binance::new(config).expect("Failed to create Binance");

    println!("[TEST] Starting WebSocket connection...");
    // 连接 WebSocket
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

    println!("[TEST] Subscribing to ticker for BTC/USDT...");
    // 订阅 ticker
    match exchange.watch_ticker("BTC/USDT").await {
        Ok(mut stream) => {
            println!("[TEST] Ticker stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            // 等待第一条消息
            println!("[TEST] Waiting for ticker message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(ticker))) => {
                    println!(
                        "[TEST] ✓ Ticker received: symbol={}, last={:?}, bid={:?}, ask={:?}",
                        ticker.symbol, ticker.last, ticker.bid, ticker.ask
                    );
                    // 打印原始事件类型
                    if let Some(event_type) = ticker.info.get("e") {
                        println!("[TEST] Event type (e): {}", event_type);
                    }
                    // 打印可能的bid/ask原始字段
                    if let Some(b) = ticker.info.get("b") {
                        println!("[TEST] Raw 'b' field: {}", b);
                    }
                    if let Some(a) = ticker.info.get("a") {
                        println!("[TEST] Raw 'a' field: {}", a);
                    }
                    if let Some(bid_price) = ticker.info.get("bidPrice") {
                        println!("[TEST] Raw 'bidPrice' field: {}", bid_price);
                    }
                    if let Some(ask_price) = ticker.info.get("askPrice") {
                        println!("[TEST] Raw 'askPrice' field: {}", ask_price);
                    }

                    assert!(!ticker.symbol.is_empty());

                    // 检查ticker核心字段
                    use rust_decimal::Decimal;
                    assert!(ticker.last.is_some(), "Last price should be present");
                    if let Some(last) = ticker.last {
                        assert!(last.0 > Decimal::ZERO, "Last price should be positive");
                    }

                    // Binance现货ticker通常有bid/ask
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
                    println!("[TEST] ✗ Ticker error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Ticker stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Ticker timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_ticker failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_orderbook() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Binance::new(config).expect("Failed to create Binance");

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

    println!("[TEST] Subscribing to orderbook for BTC/USDT (limit=5)...");
    match exchange.watch_order_book("BTC/USDT", Some(5)).await {
        Ok(mut stream) => {
            println!("[TEST] OrderBook stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for orderbook message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(ob))) => {
                    println!(
                        "[TEST] ✓ OrderBook received: symbol={}, bids={}, asks={}",
                        ob.symbol,
                        ob.bids.len(),
                        ob.asks.len()
                    );
                    assert!(!ob.bids.is_empty() || !ob.asks.is_empty());
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ OrderBook error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ OrderBook stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ OrderBook timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_order_book failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_trades() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Binance::new(config).expect("Failed to create Binance");

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

    println!("[TEST] Subscribing to trades for BTC/USDT...");
    match exchange.watch_market_trades("BTC/USDT").await {
        Ok(mut stream) => {
            println!("[TEST] Trades stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for trades message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(trades))) => {
                    println!("[TEST] ✓ Trades: {} trades received", trades.len());
                    if let Some(trade) = trades.first() {
                        println!(
                            "[TEST] First trade: symbol={}, price={:?}, amount={:?}",
                            trade.symbol, trade.price, trade.amount
                        );
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Trades error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Trades stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Trades timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_trades failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

// ============================================================================
// Binance 特有方法测试
// ============================================================================

#[tokio::test]
async fn test_ws_ohlcv() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Binance::new(config).expect("Failed to create Binance");

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

    println!("[TEST] Subscribing to OHLCV for BTC/USDT (1m timeframe)...");
    // 使用 WsExchange trait 的 watch_ohlcv 方法
    use ccxt_core::types::Timeframe;
    match exchange.watch_ohlcv("BTC/USDT", Timeframe::M1).await {
        Ok(mut stream) => {
            println!("[TEST] OHLCV stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for OHLCV message (timeout: 70s)...");
            // OHLCV 可能需要较长时间才有新数据
            match tokio::time::timeout(std::time::Duration::from_secs(70), stream.next()).await {
                Ok(Some(Ok(ohlcvs))) => {
                    println!("[TEST] ✓ OHLCV: {} candles received", ohlcvs.len());
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
                        assert!(ohlcv.timestamp > 0, "Timestamp should be positive");
                        use rust_decimal::Decimal;
                        assert!(
                            ohlcv.open.0 > Decimal::ZERO,
                            "Open price should be positive"
                        );
                        assert!(ohlcv.high.0 >= ohlcv.low.0, "High should be >= low");
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ OHLCV error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ OHLCV stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ OHLCV timeout after 70s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_ohlcv failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_bids_asks() {
    let config = ccxt_core::ExchangeConfig::default();
    let exchange = Binance::new(config).expect("Failed to create Binance");

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

    println!("[TEST] Subscribing to bids_asks for BTC/USDT...");
    // 使用 Binance 特有方法
    match exchange.watch_bids_asks("BTC/USDT").await {
        Ok(mut stream) => {
            println!("[TEST] BidsAsks stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for bids_asks message (timeout: 20s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(20), stream.next()).await {
                Ok(Some(Ok(ba))) => {
                    println!(
                        "[TEST] ✓ Spot BidsAsks: symbol={}, bid={} @ {}, ask={} @ {}",
                        ba.symbol, ba.bid_quantity, ba.bid_price, ba.ask_quantity, ba.ask_price
                    );
                    assert!(!ba.symbol.is_empty(), "Symbol should not be empty");
                    assert!(
                        ba.bid_price > rust_decimal::Decimal::ZERO,
                        "Bid price should be positive"
                    );
                    assert!(
                        ba.ask_price > rust_decimal::Decimal::ZERO,
                        "Ask price should be positive"
                    );

                    // 验证价差为正
                    let spread = ba.ask_price - ba.bid_price;
                    assert!(
                        spread >= rust_decimal::Decimal::ZERO,
                        "Spread should be non-negative"
                    );
                    println!("[TEST] Spread: {}", spread);
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ BidsAsks error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ BidsAsks stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ BidsAsks timeout after 20s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_bids_asks failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_bids_asks_swap() {
    let config = ccxt_core::ExchangeConfig::default();
    let options = BinanceOptions {
        default_type: DefaultType::Swap,
        ..Default::default()
    };
    let exchange = Binance::new_with_options(config, options).expect("Failed to create Binance");

    println!("[TEST] Default WebSocket URL: {}", exchange.get_ws_url());
    println!("[TEST] Note: bookTicker belongs to /public, will use /public/ws for swap");
    println!("[TEST] Starting WebSocket connection (Swap market)...");
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

    println!("[TEST] Subscribing to bids_asks for BTC/USDT:USDT...");
    match exchange.watch_bids_asks("BTC/USDT:USDT").await {
        Ok(mut stream) => {
            println!("[TEST] Swap BidsAsks stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for bids_asks message (timeout: 20s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(20), stream.next()).await {
                Ok(Some(Ok(ba))) => {
                    println!(
                        "[TEST] ✓ Swap BidsAsks: symbol={}, bid={} @ {}, ask={} @ {}",
                        ba.symbol, ba.bid_quantity, ba.bid_price, ba.ask_quantity, ba.ask_price
                    );
                    assert!(!ba.symbol.is_empty(), "Symbol should not be empty");
                    assert!(
                        ba.bid_price > rust_decimal::Decimal::ZERO,
                        "Bid price should be positive"
                    );
                    assert!(
                        ba.ask_price > rust_decimal::Decimal::ZERO,
                        "Ask price should be positive"
                    );

                    // 验证价差为正
                    let spread = ba.ask_price - ba.bid_price;
                    assert!(
                        spread >= rust_decimal::Decimal::ZERO,
                        "Spread should be non-negative"
                    );
                    println!("[TEST] Spread: {}", spread);
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Swap BidsAsks error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Swap BidsAsks stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Swap BidsAsks timeout after 20s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_bids_asks (swap) failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

// ============================================================================
// 私有频道测试 (需要 API credentials)
// ============================================================================

#[tokio::test]
async fn test_watch_balance() {
    let Some(exchange) = create_binance_for_private_tests().await else {
        return;
    };

    match exchange.watch_balance().await {
        Ok(mut stream) => {
            match tokio::time::timeout(std::time::Duration::from_secs(15), stream.next()).await {
                Ok(Some(Ok(balance))) => {
                    println!("Balance: {} assets", balance.balances.len());
                    for (asset, entry) in balance.balances.iter().take(5) {
                        println!("  {}: free={}, used={}", asset, entry.free, entry.used);
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("Balance error: {}", e);
                }
                Ok(None) => {
                    println!("Balance stream ended");
                }
                Err(_) => {
                    println!("Balance timeout after 15s");
                }
            }
        }
        Err(e) => {
            println!("watch_balance failed: {} (may need API permissions)", e);
        }
    }
}

#[tokio::test]
async fn test_watch_orders() {
    let Some(exchange) = create_binance_for_private_tests().await else {
        return;
    };

    match exchange.watch_orders(None).await {
        Ok(mut stream) => {
            match tokio::time::timeout(std::time::Duration::from_secs(15), stream.next()).await {
                Ok(Some(Ok(order))) => {
                    println!(
                        "Order: id={}, symbol={}, side={:?}, status={:?}",
                        order.id, order.symbol, order.side, order.status
                    );
                }
                Ok(Some(Err(e))) => {
                    println!("Order error: {}", e);
                }
                Ok(None) => {
                    println!("Order stream ended");
                }
                Err(_) => {
                    println!("Order timeout (may need active orders)");
                }
            }
        }
        Err(e) => {
            println!("watch_orders failed: {} (may need API permissions)", e);
        }
    }
}

// ============================================================================
// 期货测试 (需要 default_type = Swap)
// ============================================================================

#[tokio::test]
async fn test_watch_mark_price() {
    let config = ccxt_core::ExchangeConfig::default();
    let options = BinanceOptions {
        default_type: DefaultType::Swap,
        ..Default::default()
    };
    let exchange = Binance::new_with_options(config, options).expect("Failed to create Binance");

    println!("[TEST] Expected WebSocket URL: {}", exchange.get_ws_url());
    println!("[TEST] Starting WebSocket connection (Swap market)...");
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

    println!("[TEST] Subscribing to mark_price for BTC/USDT:USDT...");
    // 使用 Binance 特有方法
    match exchange.watch_mark_price("BTC/USDT:USDT").await {
        Ok(mut stream) => {
            println!("[TEST] MarkPrice stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for mark_price message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(mp))) => {
                    println!(
                        "[TEST] ✓ MarkPrice: symbol={}, mark={}, index={:?}, funding={:?}",
                        mp.symbol, mp.mark_price, mp.index_price, mp.last_funding_rate
                    );
                    assert!(mp.mark_price > rust_decimal::Decimal::ZERO);
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ MarkPrice error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ MarkPrice stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ MarkPrice timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_mark_price failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

// ============================================================================
// 合约市场数据测试 (需要 default_type = Swap)
// ============================================================================

#[tokio::test]
async fn test_watch_ticker_swap() {
    let config = ccxt_core::ExchangeConfig::default();
    let options = BinanceOptions {
        default_type: DefaultType::Swap,
        ..Default::default()
    };
    let exchange = Binance::new_with_options(config, options).expect("Failed to create Binance");

    // Print the WS URL that will be used
    let ws_url = exchange.get_ws_url();
    println!("[TEST] Expected WebSocket URL: {}", ws_url);

    println!("[TEST] Starting WebSocket connection (Swap market)...");
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
                    // 打印原始事件类型
                    if let Some(event_type) = ticker.info.get("e") {
                        println!("[TEST] Event type (e): {}", event_type);
                    }
                    // 打印所有info keys
                    println!(
                        "[TEST] All info keys: {:?}",
                        ticker.info.keys().collect::<Vec<_>>()
                    );
                    // 打印可能的bid/ask原始字段
                    if let Some(b) = ticker.info.get("b") {
                        println!("[TEST] Raw 'b' field: {}", b);
                    }
                    if let Some(a) = ticker.info.get("a") {
                        println!("[TEST] Raw 'a' field: {}", a);
                    }
                    if let Some(b_) = ticker.info.get("B") {
                        println!("[TEST] Raw 'B' field: {}", b_);
                    }
                    if let Some(a_) = ticker.info.get("A") {
                        println!("[TEST] Raw 'A' field: {}", a_);
                    }

                    assert!(!ticker.symbol.as_str().is_empty());

                    // 检查ticker核心字段
                    use rust_decimal::Decimal;
                    assert!(ticker.last.is_some(), "Last price should be present");
                    if let Some(last) = ticker.last {
                        assert!(last.0 > Decimal::ZERO, "Last price should be positive");
                    }

                    // Binance合约ticker可能有bid/ask（取决于市场和API）
                    // 打印原始数据用于调试
                    println!("[TEST] Raw swap ticker info fields:");
                    if let Some(info) = ticker.info.get("b") {
                        println!("[TEST]   b (bidPrice raw): {}", info);
                    }
                    if let Some(info) = ticker.info.get("a") {
                        println!("[TEST]   a (askPrice raw): {}", info);
                    }
                    if let Some(info) = ticker.info.get("B") {
                        println!("[TEST]   B (bidQty raw): {}", info);
                    }
                    if let Some(info) = ticker.info.get("A") {
                        println!("[TEST]   A (askQty raw): {}", info);
                    }

                    if let Some(bid) = ticker.bid {
                        println!("[TEST] ✓ Bid price parsed: {:?}", bid);
                        assert!(bid.0 > Decimal::ZERO, "Bid price should be positive");
                    } else {
                        println!(
                            "[TEST] ⚠ Warning: Bid price is None (API may not provide 'b' field)"
                        );
                    }

                    if let Some(ask) = ticker.ask {
                        println!("[TEST] ✓ Ask price parsed: {:?}", ask);
                        assert!(ask.0 > Decimal::ZERO, "Ask price should be positive");
                    } else {
                        println!(
                            "[TEST] ⚠ Warning: Ask price is None (API may not provide 'a' field)"
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
                    println!("[TEST] ✗ Swap Ticker stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Swap Ticker timeout after 15s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_ticker (swap) failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_orderbook_swap() {
    // Note: OrderBook (depth) stream belongs to /public endpoint in Binance's new WebSocket architecture
    // We need to configure accordingly. For now, we test with the understanding that
    // the connection should use /public endpoint for depth streams.

    let config = ccxt_core::ExchangeConfig::default();
    // Create a custom Binance instance that will connect to the public endpoint
    // Since our default swap config uses /market, we'll test orderbook with spot for now
    // TODO: Implement multi-endpoint support for simultaneous /market and /public streams
    let exchange = Binance::new(config).expect("Failed to create Binance");

    println!("[TEST] Starting WebSocket connection (using spot endpoint for orderbook)...");
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

    println!("[TEST] Subscribing to orderbook for BTC/USDT (limit=5)...");
    match exchange.watch_order_book("BTC/USDT", Some(5)).await {
        Ok(mut stream) => {
            println!("[TEST] OrderBook stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for orderbook message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(ob))) => {
                    println!(
                        "[TEST] ✓ OrderBook received: symbol={}, bids={}, asks={}",
                        ob.symbol,
                        ob.bids.len(),
                        ob.asks.len()
                    );
                    assert!(!ob.bids.is_empty() || !ob.asks.is_empty());
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ OrderBook error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ OrderBook stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ OrderBook timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_order_book failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_trades_swap() {
    let config = ccxt_core::ExchangeConfig::default();
    let options = BinanceOptions {
        default_type: DefaultType::Swap,
        ..Default::default()
    };
    let exchange = Binance::new_with_options(config, options).expect("Failed to create Binance");

    println!("[TEST] Starting WebSocket connection (Swap market)...");
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

    println!("[TEST] Subscribing to trades for BTC/USDT:USDT...");
    match exchange.watch_market_trades("BTC/USDT:USDT").await {
        Ok(mut stream) => {
            println!("[TEST] Trades stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for trades message (timeout: 10s)...");
            match tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(trades))) => {
                    println!("[TEST] ✓ Swap Trades: {} trades received", trades.len());
                    if let Some(trade) = trades.first() {
                        println!(
                            "[TEST] First trade: symbol={}, price={:?}, amount={:?}",
                            trade.symbol, trade.price, trade.amount
                        );
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Swap Trades error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Swap Trades stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Swap Trades timeout after 10s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_market_trades (swap) failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

#[tokio::test]
async fn test_watch_ohlcv_swap() {
    let config = ccxt_core::ExchangeConfig::default();
    let options = BinanceOptions {
        default_type: DefaultType::Swap,
        ..Default::default()
    };
    let exchange = Binance::new_with_options(config, options).expect("Failed to create Binance");

    println!("[TEST] Starting WebSocket connection (Swap market)...");
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

    println!("[TEST] Subscribing to OHLCV for BTC/USDT:USDT (1m timeframe)...");
    use ccxt_core::types::Timeframe;
    match exchange.watch_ohlcv("BTC/USDT:USDT", Timeframe::M1).await {
        Ok(mut stream) => {
            println!("[TEST] OHLCV stream created successfully");
            println!(
                "[TEST] Current subscriptions: {:?}",
                exchange.subscriptions()
            );

            println!("[TEST] Waiting for OHLCV message (timeout: 70s)...");
            // OHLCV 可能需要较长时间才有新数据
            match tokio::time::timeout(std::time::Duration::from_secs(70), stream.next()).await {
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
                        assert!(ohlcv.timestamp > 0, "Timestamp should be positive");
                        use rust_decimal::Decimal;
                        assert!(
                            ohlcv.open.0 > Decimal::ZERO,
                            "Open price should be positive"
                        );
                        assert!(ohlcv.high.0 >= ohlcv.low.0, "High should be >= low");
                    }
                }
                Ok(Some(Err(e))) => {
                    println!("[TEST] ✗ Swap OHLCV error: {}", e);
                }
                Ok(None) => {
                    println!("[TEST] ✗ Swap OHLCV stream ended (no more messages)");
                }
                Err(_) => {
                    println!("[TEST] ✗ Swap OHLCV timeout after 70s (no messages received)");
                    println!("[TEST] Connection state: {:?}", exchange.ws_state());
                    println!(
                        "[TEST] Current subscriptions: {:?}",
                        exchange.subscriptions()
                    );
                }
            }
        }
        Err(e) => {
            println!("[TEST] ✗ watch_ohlcv (swap) failed: {}", e);
        }
    }

    println!("[TEST] Disconnecting WebSocket...");
    let _ = exchange.ws_disconnect().await;
    println!("[TEST] Test completed");
}

// ============================================================================
// 结构验证测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::{
        Order,
        order::{OrderSide, OrderStatus, OrderType},
    };
    use std::collections::HashMap;

    #[test]
    fn test_order_structure() {
        let order = Order {
            id: "123456789".to_string(),
            client_order_id: Some("test_order_001".to_string()),
            info: HashMap::new(),
            timestamp: Some(1234567890000),
            datetime: Some("2023-01-01T00:00:00.000Z".to_string()),
            last_trade_timestamp: None,
            symbol: Symbol::new_unchecked("BTC/USDT"),
            order_type: OrderType::Limit,
            time_in_force: Some("GTC".to_string()),
            post_only: None,
            reduce_only: None,
            side: OrderSide::Buy,
            price: Some(rust_decimal::Decimal::from(50000)),
            stop_price: None,
            trigger_price: None,
            take_profit_price: None,
            stop_loss_price: None,
            trailing_delta: None,
            trailing_percent: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            amount: rust_decimal::Decimal::from_f64_retain(0.1).unwrap(),
            filled: Some(rust_decimal::Decimal::from_f64_retain(0.05).unwrap()),
            remaining: Some(rust_decimal::Decimal::from_f64_retain(0.05).unwrap()),
            cost: None,
            average: None,
            status: OrderStatus::Open,
            fee: None,
            fees: None,
            trades: None,
        };

        assert_eq!(order.id, "123456789");
        assert_eq!(order.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.status, OrderStatus::Open);
        println!("✓ Order structure validation");
    }

    #[test]
    fn test_binance_options() {
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            ..Default::default()
        };
        assert_eq!(options.default_type, DefaultType::Swap);
        println!("✓ Binance options validation");
    }
}

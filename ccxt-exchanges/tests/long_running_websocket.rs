//! 长时间运行 WebSocket 稳定性测试
//!
//! **用途**: 验证 WebSocket 连接在长时间运行下的稳定性
//! **运行时间**: 约 60 分钟/交易所
//! **跳过标志**: 默认忽略，需要手动执行
//!
//! # 使用方法
//!
//! ```bash
//! # 运行所有交易所长时间测试
//! cargo test -p ccxt-exchanges --test long_running_websocket -- --ignored --nocapture
//!
//! # 运行单个交易所测试
//! cargo test -p ccxt-exchanges --test long_running_websocket test_binance_long_running -- --ignored --nocapture
//! ```

use ccxt_core::WsExchange;
use ccxt_core::network::ws_client::WsConnectionState;
use ccxt_core::types::common::default_type::DefaultType;
use ccxt_core::utils::logging::{LogConfig, LogLevel, try_init_logging};
use ccxt_exchanges::binance::{Binance, BinanceOptions};
use ccxt_exchanges::bitget::Bitget;
use ccxt_exchanges::bybit::Bybit;
use ccxt_exchanges::gate::Gate;
use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidOptions};
use ccxt_exchanges::okx::Okx;
use futures_util::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// 测试结果统计
#[derive(Clone)]
struct TestStats {
    message_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
    start_time: Arc<Instant>,
}

impl TestStats {
    fn new() -> Self {
        Self {
            message_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            start_time: Arc::new(Instant::now()),
        }
    }

    fn record_message(&self) {
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    fn elapsed_minutes(&self) -> u64 {
        self.start_time.elapsed().as_secs() / 60
    }

    fn elapsed_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// 通用长时间运行测试框架
async fn run_long_running_test<E>(
    exchange_name: &'static str,
    symbol: &'static str,
    exchange: Arc<E>,
    duration_secs: u64,
) where
    E: WsExchange + 'static,
{
    let stats = TestStats::new();

    // 连接
    println!("📡 正在连接 {} WebSocket...", exchange_name);
    exchange.ws_connect().await.expect("Failed to connect");
    println!("✅ WebSocket 连接成功");
    println!("📊 连接状态: {:?}\n", exchange.ws_state());

    // 订阅
    println!("📡 正在订阅 {} Ticker...", symbol);
    let mut stream = exchange
        .watch_ticker(symbol)
        .await
        .expect("Failed to subscribe");
    println!("✅ 订阅成功");
    println!("📊 连接状态: {:?}", exchange.ws_state());
    println!("⏳ 等待 ticker 数据...\n");

    // 启动监控任务
    let monitor_exchange = exchange.clone();
    let monitor_stats = stats.clone();
    let monitor_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let state = monitor_exchange.ws_state();
            let msgs = monitor_stats.message_count.load(Ordering::Relaxed);
            println!(
                "[{}] {} 运行 {} 分钟 | 状态: {:?} | 消息: {} 条",
                chrono::Local::now().format("%H:%M:%S"),
                exchange_name,
                monitor_stats.elapsed_minutes(),
                state,
                msgs
            );

            if monitor_stats.elapsed_seconds() >= duration_secs {
                break;
            }
        }
    });

    // 接收消息
    let mut count = 0u64;
    loop {
        match timeout(Duration::from_secs(30), stream.next()).await {
            Ok(Some(Ok(ticker))) => {
                count += 1;
                stats.record_message();

                if count % 100 == 0 {
                    println!(
                        "[{}] 📨 #{} {}: last={:?}",
                        chrono::Local::now().format("%H:%M:%S"),
                        count,
                        ticker.symbol,
                        ticker.last
                    );
                }
            }
            Ok(Some(Err(e))) => {
                eprintln!("❌ Error: {}", e);
                stats.record_error();
            }
            Ok(None) => {
                eprintln!("⚠️  Stream ended");
                break;
            }
            Err(_) => {
                // 超时，检查状态
                let state = exchange.ws_state();
                if !matches!(state, WsConnectionState::Connected) {
                    eprintln!("⚠️  连接状态: {:?}", state);
                }
            }
        }

        if stats.elapsed_seconds() >= duration_secs {
            break;
        }
    }

    // 停止监控
    monitor_handle.abort();

    // 打印总结
    println!("\n{}", "=".repeat(60));
    println!("📊 {} 测试总结", exchange_name);
    println!("{}", "=".repeat(60));
    println!(
        "⏱️  运行时长: {} 分 {} 秒",
        stats.elapsed_seconds() / 60,
        stats.elapsed_seconds() % 60
    );
    println!(
        "📨 接收消息: {} 条",
        stats.message_count.load(Ordering::Relaxed)
    );
    println!("❌ 错误次数: {}", stats.error_count.load(Ordering::Relaxed));
    println!("{}", "=".repeat(60));

    let _ = exchange.ws_disconnect().await;
}

/// 初始化测试环境
fn init_test_env() {
    let log_config = LogConfig {
        level: LogLevel::Debug,
        ..LogConfig::test()
    };
    try_init_logging(&log_config);
}

/// Binance 现货长时间运行测试（60 分钟）
#[tokio::test]
#[ignore]
async fn test_binance_long_running() {
    init_test_env();
    println!("🚀 启动 Binance 现货长时间运行测试（60 分钟）\n");
    println!("⚠️  注意：使用生产环境，请确保网络畅通\n");

    let config = ccxt_core::ExchangeConfig {
        sandbox: false,
        ..Default::default()
    };
    let exchange = Arc::new(Binance::new(config).expect("Failed to create Binance"));

    run_long_running_test("Binance Spot", "BTC/USDT", exchange, 3600).await;
}

/// Binance 合约长时间运行测试（60 分钟）
#[tokio::test]
#[ignore]
async fn test_binance_swap_long_running() {
    init_test_env();
    println!("🚀 启动 Binance 合约长时间运行测试（60 分钟）\n");
    println!("⚠️  注意：使用生产环境，请确保网络畅通\n");

    let config = ccxt_core::ExchangeConfig {
        sandbox: false,
        ..Default::default()
    };
    let exchange = Arc::new(
        Binance::new_with_options(
            config,
            BinanceOptions {
                default_type: DefaultType::Swap,
                ..Default::default()
            },
        )
        .expect("Failed to create Binance"),
    );

    run_long_running_test("Binance Swap", "BTC/USDT:USDT", exchange, 3600).await;
}

/// OKX 长时间运行测试（60 分钟）
#[tokio::test]
#[ignore]
async fn test_okx_long_running() {
    init_test_env();
    println!("🚀 启动 OKX 长时间运行测试（60 分钟）\n");

    let config = ccxt_core::ExchangeConfig {
        sandbox: false,
        ..Default::default()
    };
    let exchange = Arc::new(Okx::new(config).expect("Failed to create OKX"));

    run_long_running_test("OKX", "BTC/USDT", exchange, 3600).await;
}

/// Bybit 长时间运行测试（60 分钟）
#[tokio::test]
#[ignore]
async fn test_bybit_long_running() {
    init_test_env();
    println!("🚀 启动 Bybit 长时间运行测试（60 分钟）\n");

    let config = ccxt_core::ExchangeConfig {
        sandbox: false,
        ..Default::default()
    };
    let exchange = Arc::new(Bybit::new(config).expect("Failed to create Bybit"));

    run_long_running_test("Bybit", "BTC/USDT", exchange, 3600).await;
}

/// Bitget 长时间运行测试（60 分钟）
#[tokio::test]
#[ignore]
async fn test_bitget_long_running() {
    init_test_env();
    println!("🚀 启动 Bitget 长时间运行测试（60 分钟）\n");
    println!("⚠️  注意：使用生产环境，请确保网络畅通\n");

    let config = ccxt_core::ExchangeConfig {
        sandbox: false,
        ..Default::default()
    };
    let exchange = Arc::new(Bitget::new(config).expect("Failed to create Bitget"));

    run_long_running_test("Bitget", "BTC/USDT", exchange, 3600).await;
}

/// Hyperliquid 长时间运行测试（60 分钟）
#[tokio::test]
#[ignore]
async fn test_hyperliquid_long_running() {
    init_test_env();
    println!("🚀 启动 Hyperliquid 长时间运行测试（60 分钟）\n");
    println!("⚠️  注意：使用生产环境，请确保网络畅通\n");

    let config = ccxt_core::ExchangeConfig {
        sandbox: false,
        ..Default::default()
    };
    let options = HyperLiquidOptions {
        testnet: false,
        ..Default::default()
    };
    let exchange = Arc::new(
        HyperLiquid::new_with_options(config, options, None).expect("Failed to create HyperLiquid"),
    );

    run_long_running_test("Hyperliquid", "BTC/USDC:USDC", exchange, 3600).await;
}

/// Gate 长时间运行测试（60 分钟）
#[tokio::test]
#[ignore]
async fn test_gate_long_running() {
    init_test_env();
    println!("🚀 启动 Gate 长时间运行测试（60 分钟）\n");
    println!("⚠️  注意：使用生产环境，请确保网络畅通\n");

    let config = ccxt_core::ExchangeConfig {
        sandbox: false,
        ..Default::default()
    };
    let exchange = Arc::new(Gate::new(config).expect("Failed to create Gate"));

    run_long_running_test("Gate", "BTC/USDT", exchange, 3600).await;
}

# WebSocket 订阅最佳实践指南

## 📖 目录

- [快速开始](#快速开始)
- [推荐写法 1：基础用法（适合简单场景）](#推荐写法-1基础用法适合简单场景)
- [推荐写法 2：带重连监控（推荐生产使用）](#推荐写法-2带重连监控推荐生产使用)
- [推荐写法 3：完整容错（高可用场景）](#推荐写法-3完整容错高可用场景)
- [框架自动处理的能力](#框架自动处理的能力)
- [网络波动应对策略](#网络波动应对策略)
- [常见问题与解决方案](#常见问题与解决方案)

---

## 🚀 快速开始

### 框架已自动处理的能力

✅ **你不需要手动处理以下问题**：

| 能力 | 说明 | 默认状态 |
|------|------|---------|
| **自动重连** | 断线后自动重连 | ✅ 已启用 |
| **订阅恢复** | 重连后自动恢复所有订阅 | ✅ 已启用 |
| **心跳管理** | 各交易所差异化心跳配置 | ✅ 已配置 |
| **指数退避** | 重连延迟：2s → 4s → 8s → ... → 60s | ✅ 已启用 |
| **容错机制** | 连续 3 次心跳失败才触发重连 | ✅ 已启用 |
| **最大重连次数** | 999 次（几乎无限） | ✅ 已配置 |

---

## 📝 推荐写法 1：基础用法（适合简单场景）

**适用场景**: 测试、原型开发、简单监控

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::ExchangeConfig;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建交易所实例
    let config = ExchangeConfig {
        sandbox: true,  // 测试网
        ..Default::default()
    };
    let exchange = Binance::new(config)?;

    // 2. 连接 WebSocket（框架自动启用自动重连）
    exchange.ws_connect().await?;
    println!("✓ WebSocket connected");

    // 3. 订阅行情
    let mut ticker_stream = exchange.watch_ticker("BTC/USDT").await?;
    println!("✓ Subscribed to BTC/USDT ticker");

    // 4. 接收消息（断线重连后会自动恢复订阅）
    while let Some(result) = ticker_stream.next().await {
        match result {
            Ok(ticker) => {
                println!(
                    "📊 {}: last={:?}, bid={:?}, ask={:?}",
                    ticker.symbol,
                    ticker.last,
                    ticker.bid,
                    ticker.ask
                );
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
                // 注意：这里不需要手动重连！
                // 框架会自动重连并恢复订阅
            }
        }
    }

    Ok(())
}
```

### 优点
- ✅ 代码简洁
- ✅ 开箱即用
- ✅ 框架自动处理重连

### 缺点
- ⚠️ 无法感知重连事件
- ⚠️ 无法监控连接状态

---

## 📝 推荐写法 2：带重连监控（推荐生产使用）⭐

**适用场景**: 生产环境、需要监控连接状态

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::{ExchangeConfig, WsExchange};
use ccxt_core::network::ws_client::{WsEvent, WsConnectionState};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建交易所实例
    let config = ExchangeConfig {
        sandbox: true,
        ..Default::default()
    };
    let exchange = Binance::new(config)?;

    // 2. 设置事件回调（监控重连状态）
    let reconnect_notify = Arc::new(Notify::new());
    let notify_clone = reconnect_notify.clone();

    exchange.set_event_callback(move |event| {
        match event {
            WsEvent::Reconnecting { attempt } => {
                eprintln!("🔄 Reconnecting... (attempt {})", attempt);
            }
            WsEvent::ReconnectSuccess => {
                eprintln!("✅ Reconnected successfully");
            }
            WsEvent::SubscriptionRestored => {
                eprintln!("✅ Subscriptions restored");
                notify_clone.notify_one();  // 通知主循环
            }
            WsEvent::ReconnectFailed { attempt, error, .. } => {
                eprintln!("❌ Reconnect failed (attempt {}): {}", attempt, error);
            }
            WsEvent::PermanentError { error } => {
                eprintln!("💀 Permanent error: {}", error);
            }
            _ => {}
        }
    });

    // 3. 连接 WebSocket
    exchange.ws_connect().await?;
    println!("✓ WebSocket connected");

    // 4. 订阅多个频道
    let mut ticker_stream = exchange.watch_ticker("BTC/USDT").await?;
    let mut orderbook_stream = exchange.watch_order_book("ETH/USDT", Some(10)).await?;
    
    println!("✓ Subscribed to ticker and orderbook");

    // 5. 并发接收多个流的消息
    loop {
        tokio::select! {
            // Ticker 消息
            Some(result) = ticker_stream.next() => {
                match result {
                    Ok(ticker) => {
                        println!(
                            "📊 {}: last={:?}",
                            ticker.symbol,
                            ticker.last
                        );
                    }
                    Err(e) => {
                        eprintln!("❌ Ticker error: {}", e);
                    }
                }
            }
            
            // OrderBook 消息
            Some(result) = orderbook_stream.next() => {
                match result {
                    Ok(book) => {
                        println!(
                            "📚 {}: bids={}, asks={}",
                            book.symbol,
                            book.bids.len(),
                            book.asks.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("❌ OrderBook error: {}", e);
                    }
                }
            }
            
            // 重连完成通知
            _ = reconnect_notify.notified() => {
                println!("🔄 Reconnection complete, streams are ready");
            }
        }
    }
}
```

### 优点
- ✅ 实时监控连接状态
- ✅ 可感知重连事件
- ✅ 支持多流并发处理
- ✅ 生产级别可靠性

### 缺点
- ⚠️ 代码稍复杂

---

## 📝 推荐写法 3：完整容错（高可用场景）🏆

**适用场景**: 金融交易、高频监控、不能容忍数据丢失

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::{ExchangeConfig, WsExchange};
use ccxt_core::network::ws_client::{WsEvent, WsConnectionState};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use tokio::time::{Duration, timeout};

/// 高可用 WebSocket 客户端包装器
struct ResilientWsClient {
    exchange: Binance,
    is_reconnecting: Arc<RwLock<bool>>,
    message_count: Arc<RwLock<u64>>,
    last_message_time: Arc<RwLock<std::time::Instant>>,
}

impl ResilientWsClient {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let exchange = Binance::new(config)?;

        Ok(Self {
            exchange,
            is_reconnecting: Arc::new(RwLock::new(false)),
            message_count: Arc::new(RwLock::new(0)),
            last_message_time: Arc::new(RwLock::new(std::time::Instant::now())),
        })
    }

    async fn setup_monitoring(&self) {
        let is_reconnecting = self.is_reconnecting.clone();
        let message_count = self.message_count.clone();
        let last_message_time = self.last_message_time.clone();

        self.exchange.set_event_callback(move |event| {
            match event {
                WsEvent::Reconnecting { attempt } => {
                    eprintln!("🔄 [{}] Reconnecting... (attempt {})", 
                             chrono::Local::now().format("%H:%M:%S"), attempt);
                    
                    // 标记正在重连
                    let mut flag = is_reconnecting.blocking_write();
                    *flag = true;
                }
                WsEvent::ReconnectSuccess => {
                    eprintln!("✅ [{}] Reconnected", 
                             chrono::Local::now().format("%H:%M:%S"));
                }
                WsEvent::SubscriptionRestored => {
                    eprintln!("✅ [{}] Subscriptions restored", 
                             chrono::Local::now().format("%H:%M:%S"));
                    
                    let mut flag = is_reconnecting.blocking_write();
                    *flag = false;
                }
                WsEvent::ReconnectFailed { attempt, error, .. } => {
                    eprintln!("❌ [{}] Reconnect failed ({}): {}", 
                             chrono::Local::now().format("%H:%M:%S"), attempt, error);
                }
                WsEvent::HeartbeatTimeout { elapsed, timeout } => {
                    eprintln!("⚠️  [{}] Heartbeat timeout: {}ms > {}ms", 
                             chrono::Local::now().format("%H:%M:%S"), elapsed, timeout);
                }
                _ => {}
            }
        });
    }

    /// 检查消息流是否健康
    async fn check_health(&self) -> bool {
        let last_time = self.last_message_time.read().await;
        let elapsed = last_time.elapsed();
        
        // 如果超过 60 秒没有收到消息，认为不健康
        if elapsed > Duration::from_secs(60) {
            eprintln!("⚠️  No messages for {} seconds", elapsed.as_secs());
            return false;
        }
        
        true
    }

    async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. 设置监控
        self.setup_monitoring().await;

        // 2. 连接
        self.exchange.ws_connect().await?;
        println!("✓ WebSocket connected");

        // 3. 订阅
        let mut ticker_stream = self.exchange.watch_ticker("BTC/USDT").await?;
        println!("✓ Subscribed");

        // 4. 启动健康检查任务
        let health_exchange = self.exchange.clone();
        let health_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let state = health_exchange.ws_state();
                println!("📡 Connection state: {:?}", state);
                
                if matches!(state, WsConnectionState::Disconnected | WsConnectionState::Error) {
                    eprintln!("⚠️  Connection is not healthy!");
                }
            }
        });

        // 5. 接收消息（带超时）
        loop {
            match timeout(Duration::from_secs(30), ticker_stream.next()).await {
                Ok(Some(Ok(ticker))) => {
                    // 更新统计
                    {
                        let mut count = self.message_count.write().await;
                        *count += 1;
                    }
                    {
                        let mut last_time = self.last_message_time.write().await;
                        *last_time = std::time::Instant::now();
                    }

                    // 检查是否在重连中
                    let is_reconnecting = self.is_reconnecting.read().await;
                    if *is_reconnecting {
                        println!("🔄 [Reconnecting] Ticker: {:?}", ticker.last);
                    } else {
                        println!(
                            "📊 [{}] #{} {}: last={:?}",
                            chrono::Local::now().format("%H:%M:%S"),
                            self.message_count.read().await,
                            ticker.symbol,
                            ticker.last
                        );
                    }
                }
                Ok(Some(Err(e))) => {
                    eprintln!("❌ Error: {}", e);
                }
                Ok(None) => {
                    eprintln!("⚠️  Stream ended");
                    break;
                }
                Err(_) => {
                    eprintln!("⏱️  Timeout: No messages for 30 seconds");
                    // 检查连接状态
                    let state = self.exchange.ws_state();
                    eprintln!("📡 Current state: {:?}", state);
                }
            }
        }

        health_handle.abort();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ResilientWsClient::new().await?;
    client.run().await
}
```

### 优点
- ✅ 完整的监控和告警
- ✅ 健康检查机制
- ✅ 消息统计
- ✅ 生产级高可用

### 缺点
- ⚠️ 代码复杂度高
- ⚠️ 适合对可靠性要求极高的场景

---

## 🛡️ 框架自动处理的能力

### 1. 自动重连机制

```rust
// 你不需要写这些代码！❌
loop {
    if connection.is_disconnected() {
        exchange.reconnect().await?;
        exchange.resubscribe().await?;
    }
}

// 框架已经自动处理了！✅
exchange.ws_connect().await?;  // 只需调用一次
```

**内部实现**:
- 每秒检查连接状态
- 检测到断开后自动重连
- 指数退避延迟（2s → 4s → 8s → ... → 60s）
- 重连成功后自动恢复所有订阅

---

### 2. 心跳管理

```rust
// 你不需要写这些代码！❌
loop {
    tokio::time::sleep(Duration::from_secs(20)).await;
    exchange.send_ping().await?;
}

// 框架已经自动处理了！✅
// 各交易所已配置最优心跳参数：
// - Binance: ServerInitiated（被动回复）
// - OKX: ClientInitiated, 25s 间隔
// - Bybit: ClientInitiated, 20s 间隔
// - Bitget: ClientInitiated, 30s 间隔
// - Hyperliquid: ClientInitiated, 50s 间隔
```

**内部实现**:
- HeartbeatManager 统一管理
- 容错机制：连续 3 次失败才触发重连
- 容忍短暂网络波动

---

### 3. 订阅恢复

```rust
// 你不需要写这些代码！❌
async fn reconnect() {
    exchange.reconnect().await?;
    
    // 手动恢复订阅
    exchange.watch_ticker("BTC/USDT").await?;
    exchange.watch_order_book("ETH/USDT").await?;
    exchange.watch_trades("SOL/USDT").await?;
}

// 框架已经自动处理了！✅
// 框架会记住所有订阅，重连后自动恢复
```

**内部实现**:
- `pending_subscriptions` 记录所有订阅
- 重连成功后调用 `resubscribe_all()`
- 触发 `WsEvent::SubscriptionRestored` 事件

---

## 🌐 网络波动应对策略

### 场景 1: 短暂网络波动（1-2 秒）

**框架行为**:
- ✅ 心跳容错机制（连续 3 次失败才触发重连）
- ✅ 不会触发重连，连接保持

**你的代码**:
```rust
// 无需任何处理！
// 框架自动容忍短暂波动
```

---

### 场景 2: VPN 断线（5-10 秒）

**框架行为**:
1. 心跳检测失败
2. 状态变为 `Error`
3. 触发自动重连
4. 指数退避延迟（2s → 4s）
5. 重连成功
6. 自动恢复订阅
7. 触发 `SubscriptionRestored` 事件

**你的代码**:
```rust
// 可选：监控重连事件
exchange.set_event_callback(|event| {
    if let WsEvent::SubscriptionRestored = event {
        println!("✅ Subscriptions restored after VPN reconnection");
    }
});
```

---

### 场景 3: 交易所重启（30 秒）

**框架行为**:
1. 连接断开
2. 多次重试（2s → 4s → 8s → 16s → 32s）
3. 交易所恢复后连接成功
4. 自动恢复所有订阅

**你的代码**:
```rust
// 可选：添加超时监控
let mut last_message = Instant::now();

loop {
    match timeout(Duration::from_secs(30), stream.next()).await {
        Ok(Some(Ok(ticker))) => {
            last_message = Instant::now();
            // 处理消息
        }
        Err(_) => {
            if last_message.elapsed() > Duration::from_secs(60) {
                eprintln!("⚠️  No messages for 60 seconds, possible exchange restart");
            }
        }
    }
}
```

---

### 场景 4: 长时间断线（>5 分钟）

**框架行为**:
1. 持续重试（最多 999 次）
2. 延迟逐渐增加到 60 秒
3. 网络恢复后自动重连
4. 恢复订阅

**你的代码**:
```rust
// 可选：添加告警
exchange.set_event_callback(|event| {
    match event {
        WsEvent::ReconnectFailed { attempt, .. } if attempt > 10 => {
            eprintln!("🚨 Reconnect failed {} times, sending alert", attempt);
            // 发送告警通知
        }
        _ => {}
    }
});
```

---

## ❓ 常见问题与解决方案

### Q1: 如何知道当前是否在重连？

```rust
// 方法 1: 检查连接状态
let state = exchange.ws_state();
if matches!(state, WsConnectionState::Reconnecting) {
    println!("Currently reconnecting...");
}

// 方法 2: 使用事件回调
exchange.set_event_callback(|event| {
    if let WsEvent::Reconnecting { attempt } = event {
        println!("Reconnecting... attempt {}", attempt);
    }
});
```

---

### Q2: 如何手动触发重连？

```rust
// 不需要手动触发！框架自动处理

// 如果确实需要（例如强制刷新）：
exchange.ws_disconnect().await?;
tokio::time::sleep(Duration::from_secs(1)).await;
exchange.ws_connect().await?;
```

---

### Q3: 如何知道订阅是否恢复成功？

```rust
// 方法 1: 监听事件
exchange.set_event_callback(|event| {
    if let WsEvent::SubscriptionRestored = event {
        println!("✅ All subscriptions restored");
    }
});

// 方法 2: 检查订阅列表
let subscriptions = exchange.subscriptions();
println!("Active subscriptions: {:?}", subscriptions);
```

---

### Q4: 如何禁用自动重连？

```rust
use ccxt_core::network::ws_client::WsConfig;

let config = ExchangeConfig {
    sandbox: true,
    // 自定义 WebSocket 配置
    ws_config: WsConfig {
        auto_reconnect: false,  // 禁用自动重连
        ..Default::default()
    },
    ..Default::default()
};
```

---

### Q5: 如何调整重连参数？

```rust
use ccxt_core::network::ws_client::{WsConfig, BackoffConfig};
use std::time::Duration;

let config = ExchangeConfig {
    sandbox: true,
    ws_config: WsConfig {
        max_reconnect_attempts: 50,  // 最多重连 50 次
        reconnect_interval: 3000,    // 基础重连间隔 3 秒
        backoff_config: BackoffConfig {
            base_delay: Duration::from_secs(3),
            max_delay: Duration::from_secs(120),
            jitter_factor: 0.3,
        },
        ..Default::default()
    },
    ..Default::default()
};
```

---

### Q6: 如何处理消息流中的错误？

```rust
// ❌ 错误写法：遇到错误就退出
while let Some(result) = stream.next().await {
    let ticker = result?;  // 遇到错误直接返回
}

// ✅ 正确写法：容错处理
loop {
    match stream.next().await {
        Some(Ok(ticker)) => {
            // 处理正常消息
        }
        Some(Err(e)) => {
            eprintln!("❌ Error: {}", e);
            // 继续循环，等待下一条消息
            // 框架会自动处理重连
        }
        None => {
            eprintln!("⚠️  Stream ended");
            break;
        }
    }
}
```

---

### Q7: 多订阅场景下，某个订阅失败会影响其他订阅吗？

```rust
// 不会影响！每个订阅独立管理

let ticker_stream = exchange.watch_ticker("BTC/USDT").await?;     // 成功
let orderbook_stream = exchange.watch_order_book("ETH/USDT").await?;  // 成功
let trades_stream = exchange.watch_trades("INVALID").await?;      // 失败

// 前两个订阅正常工作，第三个失败不影响其他
```

---

### Q8: 如何优雅地关闭 WebSocket？

```rust
// 方法 1: 正常关闭
exchange.ws_disconnect().await?;
println!("WebSocket disconnected");

// 方法 2: 取消所有订阅后关闭
exchange.unsubscribe_all().await?;
exchange.ws_disconnect().await?;

// 方法 3: 程序退出时自动关闭
// Drop trait 会自动处理
```

---

## 📊 推荐配置总结

### 开发环境

```rust
let config = ExchangeConfig {
    sandbox: true,
    verbose: true,  // 启用详细日志
    ..Default::default()
};
```

### 生产环境

```rust
let config = ExchangeConfig {
    sandbox: false,
    verbose: false,
    ws_config: WsConfig {
        max_reconnect_attempts: 999,  // 几乎无限重连
        auto_reconnect: true,         // 启用自动重连
        ..Default::default()
    },
    ..Default::default()
};
```

### 高可用环境

```rust
let config = ExchangeConfig {
    sandbox: false,
    ws_config: WsConfig {
        max_reconnect_attempts: 999,
        auto_reconnect: true,
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 20000,  // 20 秒心跳
        heartbeat_timeout: 10000,   // 10 秒超时
        backoff_config: BackoffConfig {
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.3,  // 30% 抖动
        },
        ..Default::default()
    },
    ..Default::default()
};
```

---

## 🎯 总结

### 你只需要做

1. ✅ 调用 `ws_connect()` 一次
2. ✅ 调用 `watch_*()` 订阅
3. ✅ 使用 `while let` 或 `loop` 接收消息
4. ✅ 处理 `Ok` 和 `Err` 情况

### 框架自动处理

1. ✅ 断线检测和自动重连
2. ✅ 心跳管理
3. ✅ 订阅恢复
4. ✅ 指数退避
5. ✅ 容错机制

### 建议

1. 📝 生产环境使用 **推荐写法 2**（带重连监控）
2. 📝 高可用场景使用 **推荐写法 3**（完整容错）
3. 📝 始终处理 `Err` 情况，不要直接 `?` 传播
4. 📝 添加事件回调监控重连状态
5. 📝 定期检查连接状态

---

**文档版本**: v1.0  
**创建时间**: 2026-04-28  
**维护者**: 开发团队

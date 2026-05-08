//! WebSocket subscription management.
//!
//! 提供订阅管理、消息广播、引用计数和重连恢复功能。
//!
//! # 设计特点
//!
//! - 同步 API 用于状态查询（兼容旧架构）
//! - 异步 API 用于消息广播和订阅者管理
//! - 引用计数支持同一频道多次订阅
//! - pending_subscriptions 支持重连恢复

use crate::error::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{RwLock, mpsc};

/// 订阅者条目
#[derive(Debug)]
struct SubscriberEntry {
    /// 消息发送端
    sender: mpsc::Sender<Value>,
}

/// 订阅频道信息（用于重连恢复）
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    /// 频道名称（用于消息路由的 key，例如 "spot:btcusdt@ticker"）
    pub channel: String,
    /// 交易对符号（原始格式，例如 "BTC/USDT" 或 "btcusdt@ticker"）
    pub symbol: Option<String>,
    /// 额外参数（确保必有值，避免 Option 嵌套）
    pub params: HashMap<String, Value>,
}

/// 订阅管理器配置
#[derive(Debug, Clone)]
pub struct SubscriptionManagerConfig {
    /// 每个频道的消息通道容量
    pub channel_capacity: usize,
    /// 最大订阅数
    pub max_subscriptions: usize,
}

impl Default for SubscriptionManagerConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 256,
            max_subscriptions: super::config::DEFAULT_MAX_SUBSCRIPTIONS,
        }
    }
}

/// 单个频道的完整订阅状态
#[derive(Debug)]
struct SubscriptionState {
    /// 频道信息（用于重连恢复）
    info: SubscriptionInfo,
    /// 引用计数（支持同一频道多次订阅）
    ref_count: usize,
    /// 订阅者列表（用于消息广播）
    subscribers: Vec<SubscriberEntry>,
}

/// 订阅管理器（重构版 - 单一数据源）
///
/// 支持：
/// - 多订阅者广播
/// - 引用计数
/// - 重连恢复
/// - 同步状态查询（兼容旧架构）
///
/// # 设计优势
///
/// - **消除重复**: channel/symbol 只存储一次
/// - **强一致性**: 不会出现数据不同步的问题
/// - **简化逻辑**: `get_pending_subscriptions()` 直接遍历 subscriptions
#[derive(Debug)]
pub struct SubscriptionManager {
    /// 单一数据源: channel_key -> SubscriptionState
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionState>>>,
    /// 配置
    config: SubscriptionManagerConfig,
    /// 总订阅者数量（原子操作，支持同步查询）
    total_subscribers: AtomicUsize,
    /// 频道数量（原子操作，支持同步查询）
    channel_count: AtomicUsize,
}

impl SubscriptionManager {
    /// Creates a new subscription manager with the specified configuration.
    pub fn new(config: SubscriptionManagerConfig) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            config,
            total_subscribers: AtomicUsize::new(0),
            channel_count: AtomicUsize::new(0),
        }
    }

    /// Creates a new subscription manager with the specified maximum capacity.
    #[must_use]
    pub fn with_max_subscriptions(max_subscriptions: usize) -> Self {
        Self::new(SubscriptionManagerConfig {
            max_subscriptions,
            ..Default::default()
        })
    }

    /// Creates a new subscription manager with the default maximum capacity.
    #[must_use]
    pub fn with_default_capacity() -> Self {
        Self::new(SubscriptionManagerConfig::default())
    }

    /// Returns the maximum number of subscriptions allowed.
    #[inline]
    #[must_use]
    pub fn max_subscriptions(&self) -> usize {
        self.config.max_subscriptions
    }

    // ========================================================================
    // 底层 API：简单的订阅记录（用于 WsClient 内部）
    // ========================================================================

    /// 注册订阅（不返回 receiver）
    ///
    /// 用于底层 WsClient 记录订阅状态，支持重连恢复
    pub fn register_subscription(
        &self,
        channel: String,
        symbol: Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<()> {
        let params = params.unwrap_or_default(); // 转换为 HashMap
        let channel_key = Self::make_key(&channel, symbol.as_ref());

        // 使用 try_write 并添加重试逻辑，避免并发时订阅丢失
        let mut subscriptions = loop {
            if let Ok(guard) = self.subscriptions.try_write() {
                break guard;
            }
            // 短暂等待后重试
            std::thread::sleep(std::time::Duration::from_micros(100));
        };

        // 检查订阅数限制
        let current = self.total_subscribers.load(Ordering::Relaxed);
        if current >= self.config.max_subscriptions {
            return Err(Error::resource_exhausted(format!(
                "Maximum subscriptions ({}) reached",
                self.config.max_subscriptions
            )));
        }

        let state =
            subscriptions
                .entry(channel_key.clone())
                .or_insert_with(|| SubscriptionState {
                    info: SubscriptionInfo {
                        channel: channel.clone(),
                        symbol: symbol.clone(),
                        params: params.clone(),
                    },
                    ref_count: 0,
                    subscribers: Vec::new(),
                });

        state.ref_count += 1;
        self.total_subscribers.fetch_add(1, Ordering::Relaxed);

        if state.ref_count == 1 {
            // 新频道
            self.channel_count.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// 注销订阅
    ///
    /// 返回 true 表示这是最后一个订阅者
    pub fn unregister_subscription(&self, channel: &str, symbol: Option<&String>) -> bool {
        let channel_key = Self::make_key(channel, symbol);

        if let Ok(mut subscriptions) = self.subscriptions.try_write() {
            if let Some(state) = subscriptions.get_mut(&channel_key) {
                state.ref_count = state.ref_count.saturating_sub(1);
                self.total_subscribers.fetch_sub(1, Ordering::Relaxed);

                if state.ref_count == 0 {
                    subscriptions.remove(&channel_key);
                    self.channel_count.fetch_sub(1, Ordering::Relaxed);
                    return true;
                }
            }
        }
        false
    }

    // ========================================================================
    // 异步 API：订阅者管理
    // ========================================================================

    /// 添加订阅者，返回消息接收端
    ///
    /// # 参数
    ///
    /// - `channel`: 频道名称
    /// - `symbol`: 交易对符号（可选）
    /// - `params`: 额外参数（可选，默认为空 HashMap）
    ///
    /// # 返回
    ///
    /// 返回消息接收端，可以用于接收该频道的消息
    pub async fn add_subscriber(
        &self,
        channel: String,
        symbol: Option<String>,
        params: Option<HashMap<String, Value>>,
    ) -> Result<mpsc::Receiver<Value>> {
        let params = params.unwrap_or_default(); // 转换为 HashMap
        let channel_key = Self::make_key(&channel, symbol.as_ref());

        // 检查订阅数限制
        let current = self.total_subscribers.load(Ordering::Relaxed);
        if current >= self.config.max_subscriptions {
            return Err(Error::resource_exhausted(format!(
                "Maximum subscriptions ({}) reached",
                self.config.max_subscriptions
            )));
        }

        // 创建独立的消息通道
        let (tx, rx) = mpsc::channel(self.config.channel_capacity);

        // 添加订阅者到单一数据源
        let is_new_channel = {
            let mut subscriptions = self.subscriptions.write().await;
            let state =
                subscriptions
                    .entry(channel_key.clone())
                    .or_insert_with(|| SubscriptionState {
                        info: SubscriptionInfo {
                            channel: channel.clone(),
                            symbol: symbol.clone(),
                            params: params.clone(),
                        },
                        ref_count: 0,
                        subscribers: Vec::new(),
                    });

            let is_new = state.ref_count == 0;
            state.subscribers.push(SubscriberEntry { sender: tx });
            state.ref_count += 1;
            is_new
        };

        // 更新计数器
        self.total_subscribers.fetch_add(1, Ordering::Relaxed);
        if is_new_channel {
            self.channel_count.fetch_add(1, Ordering::Relaxed);
        }

        Ok(rx)
    }

    /// 移除订阅者
    ///
    /// # 返回
    ///
    /// 如果这是最后一个订阅者,返回 true(用于通知上层取消订阅)
    pub async fn remove_subscriber(&self, channel: &str, symbol: Option<&String>) -> Result<bool> {
        let channel_key = Self::make_key(channel, symbol);
        let mut subscriptions = self.subscriptions.write().await;

        if let Some(state) = subscriptions.get_mut(&channel_key) {
            state.ref_count = state.ref_count.saturating_sub(1);

            if state.ref_count == 0 {
                // 最后一个订阅者,移除整个状态
                // 在移除前,先关闭所有订阅者的发送通道
                // 这样接收端的 recv() 会立即返回 None,停止消息流
                if let Some(removed_state) = subscriptions.remove(&channel_key) {
                    // 所有 sender 被 drop,对应的 receiver 会收到 None
                    drop(removed_state);
                    tracing::debug!(
                        channel = %channel_key,
                        "Removed all subscribers and closed channels"
                    );
                }

                self.total_subscribers.fetch_sub(1, Ordering::Relaxed);
                self.channel_count.fetch_sub(1, Ordering::Relaxed);
                Ok(true)
            } else {
                // 还有其他订阅者
                self.total_subscribers.fetch_sub(1, Ordering::Relaxed);
                Ok(false)
            }
        } else {
            Err(Error::invalid_argument(format!(
                "Subscription not found: {}",
                channel_key
            )))
        }
    }

    /// 广播消息给指定频道的所有订阅者
    pub async fn broadcast(&self, channel_key: &str, msg: Value) {
        let subscriptions = self.subscriptions.read().await;

        if let Some(state) = subscriptions.get(channel_key) {
            for entry in &state.subscribers {
                // 使用 try_send 避免阻塞，满则跳过（背压控制）
                if entry.sender.try_send(msg.clone()).is_err() {
                    tracing::debug!(
                        "Channel {} subscriber channel full or closed, skipping",
                        channel_key
                    );
                }
            }
        }
    }

    /// 广播给所有订阅者（所有频道）
    ///
    /// 用于系统消息（如连接状态变化）
    pub async fn broadcast_all(&self, msg: Value) {
        let subscriptions = self.subscriptions.read().await;
        for state in subscriptions.values() {
            for entry in &state.subscribers {
                let _ = entry.sender.try_send(msg.clone());
            }
        }
    }

    // ========================================================================
    // 重连恢复
    // ========================================================================

    /// 获取待恢复的订阅列表（重连用）
    ///
    /// 用于重连后恢复订阅
    pub async fn get_pending_subscriptions(&self) -> Vec<SubscriptionInfo> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions
            .values()
            .filter(|state| state.ref_count > 0)
            .map(|state| state.info.clone())
            .collect()
    }

    /// 清空待恢复的订阅列表
    pub async fn clear_pending_subscriptions(&self) {
        // 注意：在单一数据源设计中，我们不直接清空订阅信息
        // 而是通过 unregister_subscription 来减少引用计数
        // 这个方法保留是为了兼容性，但实际上不做任何操作
        // 真正的清理发生在 ref_count 降为 0 时自动移除
    }

    // ========================================================================
    // 同步 API：状态查询（兼容旧架构）
    // ========================================================================

    /// Returns the current number of active subscriptions.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.total_subscribers.load(Ordering::Relaxed)
    }

    /// Returns the channel count.
    #[inline]
    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channel_count.load(Ordering::Relaxed)
    }

    /// Returns the remaining capacity for new subscriptions.
    #[inline]
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.config
            .max_subscriptions
            .saturating_sub(self.total_subscribers.load(Ordering::Relaxed))
    }

    /// Checks if a subscription exists for the given channel.
    #[inline]
    #[must_use]
    pub fn contains(&self, channel: &str, symbol: Option<&String>) -> bool {
        let key = Self::make_key(channel, symbol);
        // 使用 try_read 避免阻塞，如果无法获取锁则返回 false
        self.subscriptions
            .try_read()
            .map(|subs| subs.contains_key(&key))
            .unwrap_or(false)
    }

    /// Returns a list of all active subscription channel names.
    #[must_use]
    pub fn subscriptions(&self) -> Vec<String> {
        self.subscriptions
            .try_read()
            .map(|subs| subs.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Clears all subscriptions.
    pub async fn clear(&self) {
        self.subscriptions.write().await.clear();
        self.total_subscribers.store(0, Ordering::Relaxed);
        self.channel_count.store(0, Ordering::Relaxed);
    }

    /// 同步清空所有订阅（用于同步上下文）
    ///
    /// # 返回
    ///
    /// - `Ok(())`: 成功清空
    /// - `Err(_)`: 无法获取写锁（有其他操作正在进行）
    pub fn clear_sync(&self) -> Result<()> {
        match self.subscriptions.try_write() {
            Ok(mut subscriptions) => {
                subscriptions.clear();
                self.total_subscribers.store(0, Ordering::Relaxed);
                self.channel_count.store(0, Ordering::Relaxed);
                Ok(())
            }
            Err(_) => Err(Error::network(
                "Failed to acquire write lock for clearing subscriptions",
            )),
        }
    }

    // ========================================================================
    // 统计信息
    // ========================================================================

    /// 获取统计信息
    pub fn stats(&self) -> SubscriptionStats {
        SubscriptionStats {
            total_subscribers: self.total_subscribers.load(Ordering::Relaxed),
            channel_count: self.channel_count.load(Ordering::Relaxed),
            max_subscriptions: self.config.max_subscriptions,
            channel_capacity: self.config.channel_capacity,
        }
    }

    // ========================================================================
    // 辅助方法
    // ========================================================================

    /// 生成频道唯一标识符
    fn make_key(channel: &str, symbol: Option<&String>) -> String {
        match symbol {
            Some(sym) => format!("{}:{}", channel, sym),
            None => channel.to_string(),
        }
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

/// 订阅统计信息
#[derive(Debug, Clone)]
pub struct SubscriptionStats {
    /// 总订阅者数量
    pub total_subscribers: usize,
    /// 频道数量
    pub channel_count: usize,
    /// 最大订阅数
    pub max_subscriptions: usize,
    /// 每个频道的消息通道容量
    pub channel_capacity: usize,
}

// ============================================================================
// 兼容旧 API 的类型别名和方法
// ============================================================================

impl SubscriptionManager {
    /// 检查是否已订阅某频道（异步版本）
    pub async fn is_subscribed_async(&self, channel: &str, symbol: Option<&String>) -> bool {
        let key = Self::make_key(channel, symbol);
        let subscriptions = self.subscriptions.read().await;
        subscriptions.contains_key(&key)
    }

    /// 获取频道的引用计数
    pub async fn get_ref_count(&self, channel: &str, symbol: Option<&String>) -> usize {
        let key = Self::make_key(channel, symbol);
        let subscriptions = self.subscriptions.read().await;
        subscriptions
            .get(&key)
            .map(|state| state.ref_count)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_subscriber() {
        let manager = SubscriptionManager::with_default_capacity();

        let rx = manager
            .add_subscriber("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await
            .unwrap();

        assert_eq!(manager.count(), 1);
        assert!(manager.contains("ticker", Some(&"BTC/USDT".to_string())));

        drop(rx);
    }

    #[tokio::test]
    async fn test_reference_counting() {
        let manager = SubscriptionManager::with_default_capacity();

        let _rx1 = manager
            .add_subscriber("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await
            .unwrap();
        let _rx2 = manager
            .add_subscriber("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await
            .unwrap();

        // 同一频道被订阅两次
        assert_eq!(
            manager
                .get_ref_count("ticker", Some(&"BTC/USDT".to_string()))
                .await,
            2
        );
        assert_eq!(manager.count(), 2);
        assert_eq!(manager.channel_count(), 1);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let manager = SubscriptionManager::with_default_capacity();

        let mut rx = manager
            .add_subscriber("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await
            .unwrap();

        // 广播消息
        let msg = serde_json::json!({"price": 50000.0});
        manager.broadcast("ticker:BTC/USDT", msg.clone()).await;

        // 接收消息
        let received = rx.try_recv().unwrap();
        assert_eq!(received, msg);
    }

    #[tokio::test]
    async fn test_pending_subscriptions() {
        let manager = SubscriptionManager::with_default_capacity();

        let _rx = manager
            .add_subscriber("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await
            .unwrap();

        let pending = manager.get_pending_subscriptions().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].channel, "ticker");
        assert_eq!(pending[0].symbol, Some("BTC/USDT".to_string()));
    }

    #[tokio::test]
    async fn test_remove_subscriber() {
        let manager = SubscriptionManager::with_default_capacity();

        let _rx = manager
            .add_subscriber("ticker".to_string(), Some("BTC/USDT".to_string()), None)
            .await
            .unwrap();
        assert_eq!(manager.count(), 1);

        // 移除订阅者
        let is_last = manager
            .remove_subscriber("ticker", Some(&"BTC/USDT".to_string()))
            .await
            .unwrap();
        assert!(is_last);
        assert_eq!(manager.count(), 0);
    }
}

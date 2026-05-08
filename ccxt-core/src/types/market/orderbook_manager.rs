//! OrderBook 管理器模块
//!
//! 提供统一的 OrderBook 状态管理，支持增量更新和自动重同步

use super::orderbook::{OrderBook, OrderBookDelta};
use crate::error::{Error, Result};
use crate::types::Symbol;
use crate::ws::parser::StreamParserExt;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// OrderBook 管理器
///
/// 管理多个交易对的 OrderBook 状态，支持：
/// - 增量更新应用
/// - 自动重同步
/// - 序列号验证
/// - 快照获取
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct OrderBookManager {
    /// 各交易对的 OrderBook 状态
    orderbooks: Arc<RwLock<HashMap<String, ManagedOrderBook>>>,
    /// 默认最大深度限制
    max_depth: usize,
}

/// 管理的 OrderBook 状态
#[derive(Debug, Clone)]
struct ManagedOrderBook {
    /// OrderBook 数据
    pub orderbook: OrderBook,
    /// 是否为期货市场
    pub is_futures: bool,
    /// 最后更新时间
    pub last_update_time: i64,
    /// 更新计数
    pub update_count: u64,
}

impl OrderBookManager {
    /// 创建新的 OrderBook 管理器
    #[must_use]
    pub fn new() -> Self {
        Self {
            orderbooks: Arc::new(RwLock::new(HashMap::new())),
            max_depth: 1000,
        }
    }

    /// 创建带深度限制的管理器
    #[must_use]
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            orderbooks: Arc::new(RwLock::new(HashMap::new())),
            max_depth,
        }
    }

    /// 初始化 OrderBook（从快照）
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    /// * `orderbook` - 快照数据
    /// * `is_futures` - 是否为期货市场
    pub async fn init_orderbook(
        &self,
        symbol: &Symbol,
        orderbook: OrderBook,
        is_futures: bool,
    ) -> Result<()> {
        let symbol_str = symbol.as_str().to_string();
        let managed = ManagedOrderBook {
            orderbook,
            is_futures,
            last_update_time: chrono::Utc::now().timestamp_millis(),
            update_count: 0,
        };

        let mut orderbooks = self.orderbooks.write().await;
        orderbooks.insert(symbol_str, managed);

        debug!("Initialized orderbook for {}", symbol);
        Ok(())
    }

    /// 应用增量更新
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    /// * `delta` - 增量更新
    ///
    /// # Returns
    /// 更新后的 OrderBook，如果需要重同步返回错误
    pub async fn apply_delta(&self, symbol: &Symbol, delta: &OrderBookDelta) -> Result<OrderBook> {
        let symbol_str = symbol.as_str().to_string();
        let mut orderbooks = self.orderbooks.write().await;

        let managed = orderbooks.get_mut(&symbol_str).ok_or_else(|| {
            crate::error::Error::invalid_request(format!(
                "OrderBook not initialized for {}",
                symbol
            ))
        })?;

        // 应用增量
        if let Err(e) = managed.orderbook.apply_delta(delta, managed.is_futures) {
            return Err(Error::invalid_request(format!(
                "Failed to apply delta: {}",
                e
            )));
        }

        managed.last_update_time = chrono::Utc::now().timestamp_millis();
        managed.update_count += 1;

        // 检查是否需要重同步
        if managed.orderbook.needs_resync {
            warn!("OrderBook for {} needs resync", symbol);
            return Err(crate::error::Error::invalid_request(format!(
                "OrderBook sequence gap detected for {}",
                symbol
            )));
        }

        Ok(managed.orderbook.clone())
    }

    /// 缓冲增量消息（在快照到达前）
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    /// * `delta` - 增量更新
    pub async fn buffer_delta(&self, symbol: &Symbol, delta: OrderBookDelta) -> Result<()> {
        let symbol_str = symbol.as_str().to_string();
        let mut orderbooks = self.orderbooks.write().await;

        if let Some(managed) = orderbooks.get_mut(&symbol_str) {
            managed.orderbook.buffer_delta(delta);
            debug!(
                "Buffered delta for {} (total: {})",
                symbol,
                managed.orderbook.buffered_count()
            );
        } else {
            // 如果 OrderBook 未初始化，创建一个空的并缓冲
            let mut orderbook =
                OrderBook::new(symbol.clone(), chrono::Utc::now().timestamp_millis());
            orderbook.buffer_delta(delta);

            let managed = ManagedOrderBook {
                orderbook,
                is_futures: false,
                last_update_time: chrono::Utc::now().timestamp_millis(),
                update_count: 0,
            };
            orderbooks.insert(symbol_str, managed);
        }

        Ok(())
    }

    /// 处理快照消息（初始化或重置）
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    /// * `snapshot` - 快照数据
    /// * `is_futures` - 是否为期货市场
    pub async fn process_snapshot(
        &self,
        symbol: &Symbol,
        snapshot: OrderBook,
        is_futures: bool,
    ) -> Result<OrderBook> {
        let symbol_str = symbol.as_str().to_string();
        let mut orderbooks = self.orderbooks.write().await;

        // 检查是否有缓冲的增量
        let buffered_deltas = if let Some(existing) = orderbooks.get(&symbol_str) {
            existing.orderbook.buffered_deltas.clone()
        } else {
            std::collections::VecDeque::new()
        };

        let mut managed = ManagedOrderBook {
            orderbook: snapshot,
            is_futures,
            last_update_time: chrono::Utc::now().timestamp_millis(),
            update_count: 0,
        };

        // 恢复缓冲的增量
        managed.orderbook.buffered_deltas = buffered_deltas;

        // 处理缓冲的增量
        let processed = managed
            .orderbook
            .process_buffered_deltas(is_futures)
            .map_err(|e| {
                Error::invalid_request(format!("Failed to process buffered deltas: {}", e))
            })?;
        if processed > 0 {
            info!("Processed {} buffered deltas for {}", processed, symbol);
        }

        let result = managed.orderbook.clone();
        orderbooks.insert(symbol_str, managed);

        Ok(result)
    }

    /// 获取当前 OrderBook
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    ///
    /// # Returns
    /// 当前 OrderBook，如果不存在返回 None
    pub async fn get(&self, symbol: &Symbol) -> Option<OrderBook> {
        let symbol_str = symbol.as_str().to_string();
        let orderbooks = self.orderbooks.read().await;
        orderbooks.get(&symbol_str).map(|m| m.orderbook.clone())
    }

    /// 检查 OrderBook 是否存在
    pub async fn contains(&self, symbol: &Symbol) -> bool {
        let symbol_str = symbol.as_str().to_string();
        let orderbooks = self.orderbooks.read().await;
        orderbooks.contains_key(&symbol_str)
    }

    /// 检查是否需要重同步
    pub async fn needs_resync(&self, symbol: &Symbol) -> bool {
        let symbol_str = symbol.as_str().to_string();
        let orderbooks = self.orderbooks.read().await;
        orderbooks
            .get(&symbol_str)
            .map(|m| m.orderbook.needs_resync)
            .unwrap_or(false)
    }

    /// 重置 OrderBook（用于重同步）
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    pub async fn reset(&self, symbol: &Symbol) -> Result<()> {
        let symbol_str = symbol.as_str().to_string();
        let mut orderbooks = self.orderbooks.write().await;

        if let Some(managed) = orderbooks.get_mut(&symbol_str) {
            managed.orderbook.reset_for_resync();
            info!("Reset orderbook for {} for resync", symbol);
        }

        Ok(())
    }

    /// 删除 OrderBook
    pub async fn remove(&self, symbol: &Symbol) -> Option<OrderBook> {
        let symbol_str = symbol.as_str().to_string();
        let mut orderbooks = self.orderbooks.write().await;
        orderbooks.remove(&symbol_str).map(|m| m.orderbook)
    }

    /// 获取管理的所有 symbol 列表
    pub async fn symbols(&self) -> Vec<String> {
        let orderbooks = self.orderbooks.read().await;
        orderbooks.keys().cloned().collect()
    }

    /// 清空所有 OrderBook
    pub async fn clear(&self) {
        let mut orderbooks = self.orderbooks.write().await;
        orderbooks.clear();
    }

    /// 获取统计信息
    pub async fn stats(&self, symbol: &Symbol) -> Option<OrderBookStats> {
        let symbol_str = symbol.as_str().to_string();
        let orderbooks = self.orderbooks.read().await;
        orderbooks.get(&symbol_str).map(|m| OrderBookStats {
            update_count: m.update_count,
            last_update_time: m.last_update_time,
            buffered_count: m.orderbook.buffered_count(),
            is_synced: m.orderbook.is_synced,
            needs_resync: m.orderbook.needs_resync,
        })
    }
}

impl Default for OrderBookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// OrderBook 统计信息
#[derive(Debug, Clone, Copy)]
pub struct OrderBookStats {
    /// 更新计数
    pub update_count: u64,
    /// 最后更新时间
    pub last_update_time: i64,
    /// 缓冲消息数
    pub buffered_count: usize,
    /// 是否已同步
    pub is_synced: bool,
    /// 是否需要重同步
    pub needs_resync: bool,
}

/// 带自动重同步的 OrderBook 处理器
///
/// 封装 OrderBookManager 和 REST API 快照获取，实现自动重同步
pub struct ManagedOrderBookStream<F> {
    manager: OrderBookManager,
    snapshot_fetcher: F,
}

impl<F, Fut> ManagedOrderBookStream<F>
where
    F: Fn(&Symbol) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = Result<OrderBook>> + Send,
{
    /// 创建新的托管流
    pub fn new(snapshot_fetcher: F) -> Self {
        Self {
            manager: OrderBookManager::new(),
            snapshot_fetcher,
        }
    }

    /// 确保 OrderBook 已初始化（获取快照）
    pub async fn ensure_initialized(&self, symbol: &Symbol, is_futures: bool) -> Result<OrderBook> {
        if self.manager.contains(symbol).await {
            if let Some(ob) = self.manager.get(symbol).await {
                return Ok(ob);
            }
        }

        // 获取快照
        let snapshot = (self.snapshot_fetcher)(symbol).await?;
        self.manager
            .process_snapshot(symbol, snapshot, is_futures)
            .await
    }

    /// 处理消息（自动识别快照/增量）
    ///
    /// # Type Parameters
    /// * `P` - OrderBookDeltaParser 类型
    ///
    /// # Arguments
    /// * `symbol` - 交易对
    /// * `msg` - WebSocket 消息
    /// * `parser` - 解析器
    /// * `is_futures` - 是否为期货市场
    pub async fn process_message<P>(
        &self,
        symbol: &Symbol,
        msg: &Value,
        parser: &P,
        is_futures: bool,
    ) -> Result<OrderBook>
    where
        P: crate::ws::parser::OrderBookDeltaParser + StreamParserExt,
    {
        // 判断消息类型
        if parser.is_snapshot(msg) {
            // 解析快照 - 使用新的泛型 API
            let snapshot = parser.parse_as::<OrderBook>(msg)?;
            self.manager
                .process_snapshot(symbol, snapshot, is_futures)
                .await
        } else if parser.is_delta(msg) {
            // 解析增量
            let delta = parser.parse_delta(msg, symbol)?;

            // 确保已初始化
            if !self.manager.contains(symbol).await {
                // 未初始化，缓冲增量并获取快照
                self.manager.buffer_delta(symbol, delta).await?;
                let snapshot = (self.snapshot_fetcher)(symbol).await?;
                self.manager
                    .process_snapshot(symbol, snapshot, is_futures)
                    .await
            } else {
                // 应用增量
                match self.manager.apply_delta(symbol, &delta).await {
                    Ok(ob) => Ok(ob),
                    Err(e) => {
                        // 需要重同步
                        warn!(
                            "OrderBook delta error for {}: {}, triggering resync",
                            symbol, e
                        );
                        self.manager.reset(symbol).await?;
                        let snapshot = (self.snapshot_fetcher)(symbol).await?;
                        self.manager
                            .process_snapshot(symbol, snapshot, is_futures)
                            .await
                    }
                }
            }
        } else {
            Err(crate::error::Error::invalid_request("Unknown message type"))
        }
    }

    /// 获取当前 OrderBook
    pub async fn get(&self, symbol: &Symbol) -> Option<OrderBook> {
        self.manager.get(symbol).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Amount, OrderBookEntry, Price};
    use rust_decimal_macros::dec;

    fn create_test_orderbook() -> OrderBook {
        let mut ob = OrderBook::new(Symbol::new_unchecked("BTC/USDT"), 1234567890);
        ob.bids = vec![
            OrderBookEntry::new(Price::from(dec!(50000)), Amount::from(dec!(1.5))),
            OrderBookEntry::new(Price::from(dec!(49900)), Amount::from(dec!(2.0))),
        ];
        ob.asks = vec![
            OrderBookEntry::new(Price::from(dec!(50100)), Amount::from(dec!(1.0))),
            OrderBookEntry::new(Price::from(dec!(50200)), Amount::from(dec!(0.5))),
        ];
        ob.nonce = Some(100);
        ob.is_synced = true;
        ob
    }

    fn create_test_delta(first_id: i64, final_id: i64) -> OrderBookDelta {
        OrderBookDelta {
            symbol: Symbol::new_unchecked("BTC/USDT"),
            first_update_id: first_id,
            final_update_id: final_id,
            prev_final_update_id: Some(first_id - 1),
            timestamp: 1234567890,
            bids: vec![OrderBookEntry::new(
                Price::from(dec!(50000)),
                Amount::from(dec!(2.0)),
            )],
            asks: vec![],
        }
    }

    #[tokio::test]
    async fn test_init_orderbook() {
        let manager = OrderBookManager::new();
        let ob = create_test_orderbook();
        let symbol = Symbol::new_unchecked("BTC/USDT");

        manager
            .init_orderbook(&symbol, ob.clone(), false)
            .await
            .unwrap();

        let retrieved = manager.get(&symbol).await.unwrap();
        assert_eq!(retrieved.bids.len(), 2);
        assert_eq!(retrieved.asks.len(), 2);
    }

    #[tokio::test]
    async fn test_apply_delta() {
        let manager = OrderBookManager::new();
        let symbol = Symbol::new_unchecked("BTC/USDT");
        let ob = create_test_orderbook();

        manager.init_orderbook(&symbol, ob, false).await.unwrap();

        // 应用有效的增量
        let delta = create_test_delta(101, 102);
        let updated = manager.apply_delta(&symbol, &delta).await.unwrap();

        assert_eq!(updated.bids[0].amount.as_decimal(), dec!(2.0));
    }

    #[tokio::test]
    async fn test_buffer_delta() {
        let manager = OrderBookManager::new();
        let symbol = Symbol::new_unchecked("BTC/USDT");
        let delta = create_test_delta(101, 102);

        // 缓冲增量（未初始化）
        manager.buffer_delta(&symbol, delta).await.unwrap();

        let stats = manager.stats(&symbol).await.unwrap();
        assert_eq!(stats.buffered_count, 1);
    }

    #[tokio::test]
    async fn test_reset() {
        let manager = OrderBookManager::new();
        let symbol = Symbol::new_unchecked("BTC/USDT");
        let ob = create_test_orderbook();

        manager.init_orderbook(&symbol, ob, false).await.unwrap();

        // 模拟序列错误触发 needs_resync
        let delta = OrderBookDelta {
            symbol: symbol.clone(),
            first_update_id: 1000, // 远大于当前 nonce (100)
            final_update_id: 1001,
            prev_final_update_id: Some(999),
            timestamp: 1234567890,
            bids: vec![],
            asks: vec![],
        };

        // 应用会导致序列错误的增量
        let result = manager.apply_delta(&symbol, &delta).await;
        assert!(result.is_err());

        // 现在 needs_resync 应该为 true
        let stats = manager.stats(&symbol).await.unwrap();
        assert!(stats.needs_resync);

        // 重置后 needs_resync 应该为 false（准备重新同步）
        manager.reset(&symbol).await.unwrap();
        let stats = manager.stats(&symbol).await.unwrap();
        assert!(!stats.needs_resync);
        assert!(!stats.is_synced);
    }
}

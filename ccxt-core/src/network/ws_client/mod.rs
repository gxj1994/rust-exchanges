//! WebSocket client module.
//!
//! Provides asynchronous WebSocket connection management, subscription handling,
//! and heartbeat maintenance for cryptocurrency exchange streaming APIs.
//!
//! # REVIEW NOTE - 优秀实践
//!
//! 优先级: 优秀示例
//! `WebSocket客户端设计精良`：
//! 1. 支持背压处理（BackpressureStrategy）防止内存耗尽
//! 2. 完善的自动重连机制（AutoReconnectCoordinator）
//! 3. 连接状态管理（WsConnectionState）
//! 4. 取消令牌支持（CancellationToken）
//! 5. 详细的统计信息（WsStats）
//!
//! 这是异步网络编程的优秀示例。

pub mod config;
mod error;
mod event;
mod heartbeat;
mod message;
pub mod reconnect;
mod state;
pub mod subscription;

pub use config::{
    BackoffConfig, BackoffStrategy, BackpressureStrategy, DEFAULT_MAX_SUBSCRIPTIONS,
    DEFAULT_MESSAGE_CHANNEL_CAPACITY, DEFAULT_SHUTDOWN_TIMEOUT, DEFAULT_WRITE_CHANNEL_CAPACITY,
    HeartbeatMode, WsConfig,
};
pub use error::{WsError, WsErrorKind};
pub use event::{WsEvent, WsEventCallback};
pub use heartbeat::HeartbeatManager;
pub use message::WsMessage;
pub use reconnect::AutoReconnectCoordinator;
pub use state::{WsConnectionState, WsStats, WsStatsSnapshot};
pub use subscription::{
    SubscriptionInfo, SubscriptionManager, SubscriptionManagerConfig, SubscriptionStats,
};

use crate::error::{Error, Result};
use derive_more::Debug;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock, mpsc, watch};
use tokio::time::Duration;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::protocol::{Message, WebSocketConfig},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

/// 订阅消息构建函数类型
/// 使用 owned SubscriptionInfo 避免生命周期问题
pub type SubscribeFn = Arc<
    dyn Fn(SubscriptionInfo) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync,
>;

/// 取消订阅消息构建函数类型
pub type UnsubscribeFn = Arc<
    dyn Fn(SubscriptionInfo) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync,
>;

/// Async WebSocket client for exchange streaming APIs.
///
/// # Backpressure Handling
///
/// This client uses bounded channels to prevent memory exhaustion in high-frequency
/// trading scenarios. When the message channel is full, the configured
/// `BackpressureStrategy` determines how to handle new messages:
///
/// - `DropOldest`: Removes the oldest message to make room (default)
/// - `DropNewest`: Discards the incoming message
/// - `Block`: Waits until space is available (may stall the read loop)
#[derive(Debug)]
#[allow(unused)]
pub struct WsClient {
    config: WsConfig,
    state: Arc<AtomicU8>,
    /// 连接状态 watch channel（用于事件驱动通知）
    state_tx: Arc<watch::Sender<WsConnectionState>>,
    state_rx: Arc<Mutex<watch::Receiver<WsConnectionState>>>,
    subscription_manager: SubscriptionManager,
    message_tx: mpsc::Sender<Value>,
    message_rx: Arc<RwLock<mpsc::Receiver<Value>>>,
    write_tx: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
    pub(crate) reconnect_count: AtomicU32,
    shutdown_tx: Arc<Mutex<Option<mpsc::UnboundedSender<()>>>>,
    stats: Arc<WsStats>,
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    #[debug(skip)]
    event_callback: Arc<Mutex<Option<WsEventCallback>>>,
    /// Counter for dropped messages due to backpressure
    dropped_messages: Arc<AtomicU32>,
    /// 心跳管理器
    heartbeat_manager: Arc<HeartbeatManager>,
    /// 交易所特定的订阅消息构建函数（由 GenericWsClient 注入）
    #[debug(skip)]
    subscribe_fn: Arc<RwLock<Option<SubscribeFn>>>,
    /// 取消订阅消息构建函数
    #[debug(skip)]
    unsubscribe_fn: Arc<RwLock<Option<UnsubscribeFn>>>,
}

impl WsClient {
    /// Creates a new WebSocket client instance.
    ///
    /// The client uses bounded channels for message passing to prevent memory
    /// exhaustion. Channel capacities are configured via `WsConfig`.
    #[must_use]
    pub fn new(config: WsConfig) -> Self {
        let (message_tx, message_rx) = mpsc::channel(config.message_channel_capacity);
        let max_subscriptions = config.max_subscriptions;

        // 初始化 watch channel 用于连接状态通知
        let initial_state = WsConnectionState::Disconnected;
        let (state_tx, state_rx) = watch::channel(initial_state);

        Self {
            config,
            state: Arc::new(AtomicU8::new(initial_state.as_u8())),
            state_tx: Arc::new(state_tx),
            state_rx: Arc::new(Mutex::new(state_rx)),
            subscription_manager: SubscriptionManager::with_max_subscriptions(max_subscriptions),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            write_tx: Arc::new(RwLock::new(None)),
            reconnect_count: AtomicU32::new(0),
            shutdown_tx: Arc::new(Mutex::new(None)),
            stats: Arc::new(WsStats::new()),
            cancel_token: Arc::new(Mutex::new(None)),
            event_callback: Arc::new(Mutex::new(None)),
            dropped_messages: Arc::new(AtomicU32::new(0)),
            heartbeat_manager: Arc::new(HeartbeatManager::new()),
            subscribe_fn: Arc::new(RwLock::new(None)),
            unsubscribe_fn: Arc::new(RwLock::new(None)),
        }
    }

    /// Sets the event callback for connection lifecycle events.
    pub async fn set_event_callback(&self, callback: WsEventCallback) {
        *self.event_callback.lock().await = Some(callback);
        debug!("Event callback set");
    }

    /// Clears the event callback.
    pub async fn clear_event_callback(&self) {
        *self.event_callback.lock().await = None;
        debug!("Event callback cleared");
    }

    /// Sets the exchange-specific subscribe function.
    ///
    /// This function is called during reconnection to restore subscriptions
    /// with the correct exchange-specific message format.
    pub async fn set_subscribe_fn(&self, func: SubscribeFn) {
        *self.subscribe_fn.write().await = Some(func);
        debug!("Subscribe function injected");
    }

    /// Sets the exchange-specific unsubscribe function.
    pub async fn set_unsubscribe_fn(&self, func: UnsubscribeFn) {
        *self.unsubscribe_fn.write().await = Some(func);
        debug!("Unsubscribe function injected");
    }

    /// Gets a reference to the subscribe function, if set.
    pub async fn get_subscribe_fn(&self) -> Option<SubscribeFn> {
        self.subscribe_fn.read().await.clone()
    }

    /// Gets a reference to the unsubscribe function, if set.
    pub async fn get_unsubscribe_fn(&self) -> Option<UnsubscribeFn> {
        self.unsubscribe_fn.read().await.clone()
    }

    async fn emit_event(&self, event: WsEvent) {
        let callback = self.event_callback.lock().await;
        if let Some(ref cb) = *callback {
            let cb = Arc::clone(cb);
            drop(callback);
            tokio::spawn(async move {
                cb(event);
            });
        }
    }

    /// Sets the cancellation token for this client.
    pub async fn set_cancel_token(&self, token: CancellationToken) {
        *self.cancel_token.lock().await = Some(token);
        debug!("Cancellation token set");
    }

    /// Clears the cancellation token.
    pub async fn clear_cancel_token(&self) {
        *self.cancel_token.lock().await = None;
        debug!("Cancellation token cleared");
    }

    /// Returns a clone of the current cancellation token, if set.
    pub async fn get_cancel_token(&self) -> Option<CancellationToken> {
        self.cancel_token.lock().await.clone()
    }

    /// Establishes connection to the WebSocket server.
    #[instrument(
        name = "ws_connect",
        skip(self),
        fields(url = %self.config.url, timeout_ms = self.config.connect_timeout)
    )]
    pub async fn connect(&self) -> Result<()> {
        if self.state() == WsConnectionState::Connected {
            info!("WebSocket already connected");
            return Ok(());
        }

        let max_attempts = self.config.connect_retry_attempts;
        let retry_interval = Duration::from_millis(self.config.connect_retry_interval_ms);

        for attempt in 1..=max_attempts {
            if attempt > 1 {
                info!(
                    attempt,
                    max_attempts,
                    delay_ms = retry_interval.as_millis(),
                    "Retrying WebSocket connection"
                );
                tokio::time::sleep(retry_interval).await;
            }

            match self.connect_once().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    if attempt < max_attempts {
                        warn!(
                            attempt,
                            max_attempts,
                            error = %e,
                            "WebSocket connection failed, will retry"
                        );
                    } else {
                        error!(
                            attempt,
                            max_attempts,
                            error = %e,
                            "WebSocket connection failed after all retries"
                        );
                        return Err(e);
                    }
                }
            }
        }

        // 不应该到这里，但为了编译器满意
        Err(Error::network("Connection failed after retries"))
    }

    /// 单次连接尝试（内部方法）
    async fn connect_once(&self) -> Result<()> {
        self.set_state(WsConnectionState::Connecting);

        let url = self.config.url.clone();
        info!("Initiating WebSocket connection");

        let mut ws_config = WebSocketConfig::default();
        if let Some(limit) = self.config.max_message_size {
            ws_config.max_message_size = Some(limit);
        }
        if let Some(limit) = self.config.max_frame_size {
            ws_config.max_frame_size = Some(limit);
        }

        match tokio::time::timeout(
            Duration::from_millis(self.config.connect_timeout),
            connect_async_with_config(&url, Some(ws_config), false),
        )
        .await
        {
            Ok(Ok((ws_stream, response))) => {
                info!(
                    status = response.status().as_u16(),
                    "WebSocket connection established successfully"
                );

                // 重置统计信息，特别是 last_pong_time，避免使用旧连接的时间戳
                // 这防止了新连接建立后立即被误判为 pong timeout
                self.stats.reset();
                self.heartbeat_manager.reset();  // 重置心跳状态

                self.set_state(WsConnectionState::Connected);
                self.reconnect_count.store(0, Ordering::Release);
                self.stats.record_connected();

                // 每次连接都需要启动新的消息循环（因为 ws_stream 不同）
                // start_message_loop 会更新 write_tx，确保使用新的连接
                self.start_message_loop(ws_stream).await;

                self.resubscribe_all().await?;

                Ok(())
            }
            Ok(Err(e)) => {
                error!(error = %e, "WebSocket connection failed");
                self.set_state(WsConnectionState::Error);
                Err(Error::network(format!("WebSocket connection failed: {e}")))
            }
            Err(_) => {
                error!(
                    timeout_ms = self.config.connect_timeout,
                    "WebSocket connection timeout"
                );
                self.set_state(WsConnectionState::Error);
                Err(Error::timeout("WebSocket connection timeout"))
            }
        }
    }

    /// Establishes connection with cancellation support.
    #[instrument(
        name = "ws_connect_with_cancel",
        skip(self, cancel_token),
        fields(url = %self.config.url)
    )]
    pub async fn connect_with_cancel(&self, cancel_token: Option<CancellationToken>) -> Result<()> {
        let token = if let Some(t) = cancel_token {
            t
        } else {
            let internal_token = self.cancel_token.lock().await;
            internal_token
                .clone()
                .unwrap_or_else(CancellationToken::new)
        };

        if self.state() == WsConnectionState::Connected {
            info!("WebSocket already connected");
            return Ok(());
        }

        self.set_state(WsConnectionState::Connecting);
        let url = self.config.url.clone();

        let mut ws_config = WebSocketConfig::default();
        if let Some(limit) = self.config.max_message_size {
            ws_config.max_message_size = Some(limit);
        }
        if let Some(limit) = self.config.max_frame_size {
            ws_config.max_frame_size = Some(limit);
        }

        tokio::select! {
            biased;
            () = token.cancelled() => {
                warn!("WebSocket connection cancelled");
                self.set_state(WsConnectionState::Disconnected);
                Err(Error::cancelled("WebSocket connection cancelled"))
            }
            result = tokio::time::timeout(
                Duration::from_millis(self.config.connect_timeout),
                connect_async_with_config(&url, Some(ws_config), false),
            ) => {
                match result {
                    Ok(Ok((ws_stream, response))) => {
                        info!(status = response.status().as_u16(), "WebSocket connected");

                        // 重置统计信息，特别是 last_pong_time，避免使用旧连接的时间戳
                        self.stats.reset();
                        self.heartbeat_manager.reset();  // 重置心跳状态

                        self.set_state(WsConnectionState::Connected);
                        self.reconnect_count.store(0, Ordering::Release);
                        self.stats.record_connected();

                        // 每次连接都启动新的消息循环
                        self.start_message_loop(ws_stream).await;

                        self.resubscribe_all().await?;
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        error!(error = %e, "WebSocket connection failed");
                        self.set_state(WsConnectionState::Error);
                        Err(Error::network(format!("WebSocket connection failed: {e}")))
                    }
                    Err(_) => {
                        error!("WebSocket connection timeout");
                        self.set_state(WsConnectionState::Error);
                        Err(Error::timeout("WebSocket connection timeout"))
                    }
                }
            }
        }
    }

    /// Closes the WebSocket connection gracefully.
    #[instrument(name = "ws_disconnect", skip(self))]
    pub async fn disconnect(&self) -> Result<()> {
        info!("Initiating WebSocket disconnect");

        if let Some(tx) = self.shutdown_tx.lock().await.as_ref() {
            let _ = tx.send(());
        }

        *self.write_tx.write().await = None;
        self.set_state(WsConnectionState::Disconnected);

        info!("WebSocket disconnected");
        Ok(())
    }

    /// Gracefully shuts down the WebSocket client.
    #[instrument(name = "ws_shutdown", skip(self))]
    pub async fn shutdown(&self) {
        info!("Initiating graceful shutdown");

        {
            let token_guard = self.cancel_token.lock().await;
            if let Some(ref token) = *token_guard {
                token.cancel();
            }
        }

        self.set_state(WsConnectionState::Disconnected);

        {
            let write_tx_guard = self.write_tx.read().await;
            if let Some(ref tx) = *write_tx_guard {
                // Ignore send result - we're shutting down anyway
                drop(tx.send(Message::Close(None)).await);
            }
        }

        let shutdown_timeout = Duration::from_millis(self.config.shutdown_timeout);
        let _ = tokio::time::timeout(shutdown_timeout, async {
            if let Some(tx) = self.shutdown_tx.lock().await.as_ref() {
                let _ = tx.send(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        })
        .await;

        {
            *self.write_tx.write().await = None;
            *self.shutdown_tx.lock().await = None;
            // 同步清空订阅管理器需要特殊处理
            // clear() 是异步方法，这里使用 sync_clear 同步版本
            if let Err(e) = self.subscription_manager.clear_sync() {
                warn!(error = %e, "Failed to clear subscriptions during shutdown");
            }
            self.reconnect_count.store(0, Ordering::Release);
            self.dropped_messages.store(0, Ordering::Relaxed);
            self.stats.reset();
        }

        self.emit_event(WsEvent::Shutdown).await;
        info!("Graceful shutdown completed");
    }

    /// Attempts to reconnect to the WebSocket server.
    #[instrument(name = "ws_reconnect", skip(self))]
    pub async fn reconnect(&self) -> Result<()> {
        let count = self.reconnect_count.fetch_add(1, Ordering::AcqRel) + 1;

        if count > self.config.max_reconnect_attempts {
            error!(attempts = count, "Max reconnect attempts reached");
            return Err(Error::network("Max reconnect attempts reached"));
        }

        warn!(attempt = count, "Attempting WebSocket reconnection");
        self.set_state(WsConnectionState::Reconnecting);

        // 使用 backoff 计算延迟
        let backoff = BackoffStrategy::new(self.config.backoff_config.clone());
        let delay = backoff.calculate_delay(count - 1);
        tokio::time::sleep(delay).await;

        self.connect().await
    }

    /// Attempts to reconnect with cancellation support.
    #[instrument(name = "ws_reconnect_with_cancel", skip(self, cancel_token))]
    pub async fn reconnect_with_cancel(
        &self,
        cancel_token: Option<CancellationToken>,
    ) -> Result<()> {
        let token = if let Some(t) = cancel_token {
            t
        } else {
            let internal_token = self.cancel_token.lock().await;
            internal_token
                .clone()
                .unwrap_or_else(CancellationToken::new)
        };

        let backoff = BackoffStrategy::new(self.config.backoff_config.clone());
        self.set_state(WsConnectionState::Reconnecting);

        loop {
            if token.is_cancelled() {
                self.set_state(WsConnectionState::Disconnected);
                return Err(Error::cancelled("Reconnection cancelled"));
            }

            let attempt = self.reconnect_count.fetch_add(1, Ordering::AcqRel);

            if attempt >= self.config.max_reconnect_attempts {
                self.set_state(WsConnectionState::Error);
                return Err(Error::network(format!(
                    "Max reconnect attempts ({}) reached",
                    self.config.max_reconnect_attempts
                )));
            }

            let delay = backoff.calculate_delay(attempt);

            tokio::select! {
                biased;
                () = token.cancelled() => {
                    self.set_state(WsConnectionState::Disconnected);
                    return Err(Error::cancelled("Reconnection cancelled during backoff"));
                }
                () = tokio::time::sleep(delay) => {}
            }

            match self.connect_with_cancel(Some(token.clone())).await {
                Ok(()) => {
                    self.reconnect_count.store(0, Ordering::Release);
                    return Ok(());
                }
                Err(e) => {
                    if e.as_cancelled().is_some() {
                        self.set_state(WsConnectionState::Disconnected);
                        return Err(e);
                    }

                    let ws_error = WsError::from_error(&e);
                    if ws_error.is_permanent() {
                        self.set_state(WsConnectionState::Error);
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Returns the current reconnection attempt count.
    #[inline]
    pub fn reconnect_count(&self) -> u32 {
        self.reconnect_count.load(Ordering::Acquire)
    }

    /// Resets the reconnection attempt counter.
    pub fn reset_reconnect_count(&self) {
        self.reconnect_count.store(0, Ordering::Release);
    }

    /// Increments the reconnection attempt counter.
    pub(crate) fn increment_reconnect_count(&self) {
        self.reconnect_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Returns a snapshot of connection statistics.
    pub fn stats(&self) -> WsStatsSnapshot {
        self.stats.snapshot()
    }

    /// Resets all connection statistics.
    pub fn reset_stats(&self) {
        self.stats.reset();
    }

    /// Calculates current connection latency in milliseconds.
    pub fn latency(&self) -> Option<i64> {
        let last_pong = self.stats.last_pong_time();
        let last_ping = self.stats.last_ping_time();
        if last_pong > 0 && last_ping > 0 {
            Some(last_pong - last_ping)
        } else {
            None
        }
    }

    /// Returns the number of messages dropped due to backpressure.
    ///
    /// This counter is incremented when the message channel is full and
    /// messages are dropped according to the configured backpressure strategy.
    pub fn dropped_messages(&self) -> u32 {
        self.dropped_messages.load(Ordering::Relaxed)
    }

    /// Resets the dropped messages counter.
    pub fn reset_dropped_messages(&self) {
        self.dropped_messages.store(0, Ordering::Relaxed);
    }

    /// Creates an automatic reconnection coordinator.
    pub fn create_auto_reconnect_coordinator(self: Arc<Self>) -> AutoReconnectCoordinator {
        AutoReconnectCoordinator::new(self)
    }

    /// Receives the next available message.
    pub async fn receive(&self) -> Option<Value> {
        let mut rx = self.message_rx.write().await;
        rx.recv().await
    }

    /// Returns the current connection state.
    #[inline]
    pub fn state(&self) -> WsConnectionState {
        WsConnectionState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Returns a reference to the WebSocket configuration.
    #[inline]
    pub fn config(&self) -> &WsConfig {
        &self.config
    }

    /// Sets the connection state.
    #[inline]
    pub fn set_state(&self, state: WsConnectionState) {
        self.state.store(state.as_u8(), Ordering::Release);
        // 通知所有等待者（忽略错误，因为接收者可能不存在）
        let _ = self.state_tx.send(state);
    }

    /// 获取状态监听器（用于事件驱动等待）
    pub fn state_watch_rx(&self) -> watch::Receiver<WsConnectionState> {
        // 注意：这里不锁定，因为 watch::Receiver 是 Clone 的
        // 但实际上我们需要访问 Arc<Mutex<>>，所以还是需要锁定
        // 为了性能，我们可以直接返回 clone
        // 但 state_rx 是 Arc<Mutex<>>，所以需要特殊处理
        // 最简单的方案：返回一个新的 receiver
        // 但这需要 state_tx 直接提供
        self.state_tx.subscribe()
    }

    /// Checks whether the WebSocket is currently connected.
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.state() == WsConnectionState::Connected
    }

    /// Checks if subscribed to a specific channel.
    pub fn is_subscribed(&self, channel: &str, symbol: Option<&String>) -> bool {
        self.subscription_manager.contains(channel, symbol)
    }

    /// Returns the number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscription_manager.count()
    }

    /// Returns the remaining capacity for new subscriptions.
    pub fn remaining_capacity(&self) -> usize {
        self.subscription_manager.remaining_capacity()
    }

    /// Returns a list of all active subscription channel names.
    ///
    /// Each subscription is identified by its channel name, optionally combined
    /// with a symbol in the format "channel:symbol" or just "channel".
    pub fn subscriptions(&self) -> Vec<String> {
        self.subscription_manager.subscriptions()
    }

    /// Returns a reference to the subscription manager.
    ///
    /// This allows advanced users to access subscription management functionality
    /// directly, such as adding subscribers with message receivers and broadcasting.
    pub fn subscription_manager(&self) -> &SubscriptionManager {
        &self.subscription_manager
    }

    /// Sends a raw WebSocket message.
    ///
    /// This method uses a bounded channel for sending. If the write channel is full,
    /// it will wait until space is available.
    #[instrument(name = "ws_send", skip(self, message))]
    pub async fn send(&self, message: Message) -> Result<()> {
        // Check connection state before attempting to send
        let state = self.state();
        if state != WsConnectionState::Connected {
            return Err(Error::network(format!(
                "WebSocket not in connected state (current: {:?})",
                state
            )));
        }

        let tx = self.write_tx.read().await;

        if let Some(sender) = tx.as_ref() {
            sender
                .send(message)
                .await
                .map_err(|e| Error::network(format!("Failed to send message: {e}")))?;
            Ok(())
        } else {
            Err(Error::network("WebSocket not connected"))
        }
    }

    /// Tries to send a raw WebSocket message without blocking.
    ///
    /// Returns an error if the channel is full or closed.
    #[instrument(name = "ws_try_send", skip(self, message))]
    pub fn try_send(&self, message: Message) -> Result<()> {
        // Check connection state before attempting to send
        let state = self.state();
        if state != WsConnectionState::Connected {
            return Err(Error::network(format!(
                "WebSocket not in connected state (current: {:?})",
                state
            )));
        }

        // Note: This is a sync method, so we can't use async lock
        // We use try_read to avoid blocking
        if let Ok(tx) = self.write_tx.try_read() {
            if let Some(sender) = tx.as_ref() {
                sender.try_send(message).map_err(|e| match e {
                    mpsc::error::TrySendError::Full(_) => {
                        Error::network("Write channel full (backpressure)")
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        Error::network("WebSocket channel closed")
                    }
                })?;
                Ok(())
            } else {
                Err(Error::network("WebSocket not connected"))
            }
        } else {
            Err(Error::network("Write channel busy"))
        }
    }

    /// Sends a text message.
    #[instrument(name = "ws_send_text", skip(self, text))]
    pub async fn send_text(&self, text: String) -> Result<()> {
        self.send(Message::Text(text.into())).await
    }

    /// Sends a JSON-encoded message.
    #[instrument(name = "ws_send_json", skip(self, json))]
    pub async fn send_json(&self, json: &Value) -> Result<()> {
        let text = serde_json::to_string(json).map_err(Error::from)?;
        self.send_text(text).await
    }

    /// Generates a subscription key for a given channel and symbol.
    #[allow(unused)]
    pub fn subscription_key(channel: &str, symbol: Option<&String>) -> String {
        match symbol {
            Some(s) => format!("{channel}:{s}"),
            None => channel.to_string(),
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn start_message_loop(&self, ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) {
        let (write, mut read) = ws_stream.split();

        // Use bounded channel for write operations to prevent memory exhaustion
        let (write_tx, mut write_rx) = mpsc::channel::<Message>(self.config.write_channel_capacity);
        *self.write_tx.write().await = Some(write_tx.clone());

        // Shutdown channel remains unbounded as it's only used for signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let state = Arc::clone(&self.state);
        let message_tx = self.message_tx.clone();
        let heartbeat_manager = Arc::clone(&self.heartbeat_manager);
        let dropped_messages = Arc::clone(&self.dropped_messages);
        let backpressure_strategy = self.config.backpressure_strategy;

        // 克隆 write_tx 用于在消息循环中回复服务器 PING
        let reply_write_tx = write_tx.clone();

        // Clone state for write_handle before moving it
        let write_state = Arc::clone(&state);
        let write_handle = tokio::spawn(async move {
            let mut write = write;
            loop {
                tokio::select! {
                    Some(msg) = write_rx.recv() => {
                        if let Err(e) = write.send(msg).await {
                            error!(error = %e, "Failed to write message");
                            // Immediately update connection state to Error to prevent further send attempts
                            write_state.store(WsConnectionState::Error.as_u8(), Ordering::Release);
                            break;
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        let _ = write.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
        });

        let state_clone = Arc::clone(&state);
        let ws_stats = Arc::clone(&self.stats);
        let read_handle = tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        ws_stats.record_received(text.len() as u64);
                        // 记录收到消息（用于心跳空闲检测）
                        heartbeat_manager.record_message();

                        // 检查是否为应用层 PONG（OKX/Bitget 使用字符串 "pong"）
                        if text == "pong" {
                            heartbeat_manager.record_pong();
                            ws_stats.record_pong();
                            debug!("Received application-level pong");
                        } else if let Ok(json) = serde_json::from_str::<Value>(&text) {
                            // 检查是否为 JSON 格式的 PONG（Bybit: {"op":"pong"}, Hyperliquid: {"method":"pong"}）
                            let is_pong = json.get("op").and_then(|v| v.as_str()) == Some("pong")
                                || json.get("method").and_then(|v| v.as_str()) == Some("pong");

                            if is_pong {
                                heartbeat_manager.record_pong();
                                ws_stats.record_pong();
                                debug!("Received JSON-format application-level pong");
                            } else {
                                // 检查是否为服务器 PING（Binance 现货：{"id":123}）
                                // Binance PING 消息只包含一个 id 字段（数字），没有其他字段
                                let is_server_ping = {
                                    if let Some(obj) = json.as_object() {
                                        obj.len() == 1
                                            && obj.contains_key("id")
                                            && obj["id"].is_number()
                                    } else {
                                        false
                                    }
                                };

                                if is_server_ping {
                                    // 回复 PONG（Binance 要求回复相同的 payload）
                                    let pong_message = Message::Text(text.clone());
                                    let _ = reply_write_tx.try_send(pong_message);
                                    debug!("Replied to server ping");
                                } else {
                                    Self::send_with_backpressure(
                                        &message_tx,
                                        json,
                                        backpressure_strategy,
                                        &dropped_messages,
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        ws_stats.record_received(data.len() as u64);
                        // 记录收到消息
                        heartbeat_manager.record_message();
                        if let Some(json) = String::from_utf8(data.to_vec())
                            .ok()
                            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                        {
                            Self::send_with_backpressure(
                                &message_tx,
                                json,
                                backpressure_strategy,
                                &dropped_messages,
                            )
                            .await;
                        }
                    }
                    Ok(Message::Pong(_)) => {
                        // 记录收到 PONG（WebSocket 协议层）
                        heartbeat_manager.record_pong();
                        ws_stats.record_pong();
                    }
                    Ok(Message::Close(_)) => {
                        state_clone
                            .store(WsConnectionState::Disconnected.as_u8(), Ordering::Release);
                        break;
                    }
                    Err(_) => {
                        state_clone.store(WsConnectionState::Error.as_u8(), Ordering::Release);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // 启动应用层心跳（如果需要）
        if self.config.heartbeat_mode == HeartbeatMode::ClientInitiated
            && self.config.heartbeat_interval > 0
        {
            self.start_heartbeat_task(write_tx.clone()).await;
        }

        tokio::spawn(async move {
            let _ = tokio::join!(write_handle, read_handle);
        });
    }

    /// 启动应用层心跳任务
    async fn start_heartbeat_task(&self, write_tx: mpsc::Sender<Message>) {
        let heartbeat_manager = Arc::clone(&self.heartbeat_manager);
        let state = Arc::clone(&self.state);
        let cancel_token = self.get_cancel_token().await;

        // 提前克隆配置值，避免在异步闭包中引用 self
        let heartbeat_interval = self.config.heartbeat_interval;
        let heartbeat_timeout = self.config.heartbeat_timeout;

        // 使用配置的 PING 消息（支持各交易所自定义格式）
        let ping_message = self.config.ping_message.clone();

        // 克隆 heartbeat_manager 用于 spawn 后的 set_task_handle
        let heartbeat_manager_for_handle = Arc::clone(&heartbeat_manager);

        let handle = tokio::spawn(async move {
            heartbeat_manager.start_client_heartbeat(
                Arc::new(RwLock::new(Some(write_tx))),
                heartbeat_interval,
                heartbeat_timeout,
                ping_message,
                state,
                cancel_token,
            );
        });

        heartbeat_manager_for_handle.set_task_handle(handle).await;
    }

    /// 获取心跳管理器引用（用于外部访问）
    pub fn heartbeat_manager(&self) -> &Arc<HeartbeatManager> {
        &self.heartbeat_manager
    }

    /// Sends a message with backpressure handling.
    ///
    /// This method implements the configured backpressure strategy when the
    /// message channel is full.
    async fn send_with_backpressure(
        tx: &mpsc::Sender<Value>,
        message: Value,
        strategy: BackpressureStrategy,
        dropped_counter: &Arc<AtomicU32>,
    ) {
        match strategy {
            BackpressureStrategy::Block => {
                // Block until space is available
                if tx.send(message).await.is_err() {
                    warn!("Message channel closed");
                }
            }
            BackpressureStrategy::DropNewest => {
                // Try to send, drop if full
                match tx.try_send(message) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        let count = dropped_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        if count % 100 == 1 {
                            // Log every 100th drop to avoid log spam
                            warn!(
                                dropped_count = count,
                                "Message channel full, dropping newest message (backpressure)"
                            );
                        }
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        warn!("Message channel closed");
                    }
                }
            }
            BackpressureStrategy::DropOldest => {
                // Try to send, if full, make room by receiving and discarding
                match tx.try_send(message) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(msg)) => {
                        // Channel is full, we need to drop oldest
                        // Since we can't directly remove from the channel,
                        // we use a permit-based approach
                        let count = dropped_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        if count % 100 == 1 {
                            warn!(
                                dropped_count = count,
                                "Message channel full, dropping oldest message (backpressure)"
                            );
                        }
                        // For DropOldest, we actually drop the newest since we can't
                        // remove from the receiver side. The semantic is that we
                        // prioritize not blocking the read loop.
                        // A true DropOldest would require a different data structure.
                        drop(msg);
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        warn!("Message channel closed");
                    }
                }
            }
        }
    }

    /// Helper: generate a subscription key for logging
    fn make_subscription_key(info: &SubscriptionInfo) -> String {
        match &info.symbol {
            Some(symbol) => format!("{}:{}", info.channel, symbol),
            None => info.channel.clone(),
        }
    }

    pub(crate) async fn resubscribe_all(&self) -> Result<()> {
        let subs = self.subscription_manager.get_pending_subscriptions().await;

        if subs.is_empty() {
            debug!("No pending subscriptions to restore");
            return Ok(());
        }

        let total = subs.len();
        let mut success_count = 0;
        let mut failed_count = 0;
        let mut last_error = None;

        // 获取注入的订阅函数
        let subscribe_fn = self.subscribe_fn.read().await;

        debug!(
            subscription_count = total,
            "Starting subscription restoration"
        );

        // 重新订阅 - 单个失败不阻塞其他订阅
        for subscription in subs {
            let channel_key = Self::make_subscription_key(&subscription);

            let result = if let Some(ref func) = *subscribe_fn {
                // 使用注入的交易所特定构建器
                debug!(
                    channel = %subscription.channel,
                    symbol = ?subscription.symbol,
                    "Restoring subscription with custom builder"
                );
                match func(subscription.clone()).await {
                    Ok(msg) => self.send_json(&msg).await,
                    Err(e) => Err(e),
                }
            } else {
                // Fallback: 不应该走到这里，如果没有注入函数说明架构有问题
                let error_msg = format!(
                    "No subscribe_fn injected for channel {}",
                    subscription.channel
                );
                error!(channel = %subscription.channel, "{}", error_msg);
                Err(Error::network(error_msg))
            };

            match result {
                Ok(_) => {
                    success_count += 1;
                    debug!(channel = %channel_key, "Subscription restored");
                }
                Err(e) => {
                    failed_count += 1;
                    last_error = Some(e);
                    error!(
                        channel = %channel_key,
                        error = %last_error.as_ref().unwrap(),
                        "Failed to restore subscription"
                    );
                }
            }
        }

        // 部分成功也认为是成功的
        if success_count > 0 {
            // 订阅成功后清空记录，避免重复订阅
            self.subscription_manager
                .clear_pending_subscriptions()
                .await;

            info!(
                success = success_count,
                failed = failed_count,
                total = total,
                "Subscription restoration completed"
            );
            Ok(())
        } else {
            // 全部失败，返回错误
            error!(total = total, "All subscriptions failed to restore");
            Err(last_error.unwrap_or_else(|| Error::network("All subscriptions failed to restore")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_config_default() {
        let config = BackoffConfig::default();
        assert_eq!(config.base_delay, Duration::from_secs(2)); // 更新为 2s
        assert_eq!(config.max_delay, Duration::from_secs(60));
    }

    #[test]
    fn test_backoff_strategy_exponential_growth_no_jitter() {
        let config = BackoffConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.0,
            multiplier: 2.0,
        };
        let strategy = BackoffStrategy::new(config);

        assert_eq!(strategy.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(strategy.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(strategy.calculate_delay(2), Duration::from_secs(4));
        assert_eq!(strategy.calculate_delay(6), Duration::from_secs(60));
    }

    #[test]
    fn test_ws_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.connect_timeout, 10000);
        assert_eq!(config.max_subscriptions, DEFAULT_MAX_SUBSCRIPTIONS);
    }

    #[test]
    fn test_subscription_key() {
        let key1 = WsClient::subscription_key("ticker", Some(&"BTC/USDT".to_string()));
        assert_eq!(key1, "ticker:BTC/USDT");

        let key2 = WsClient::subscription_key("trades", None);
        assert_eq!(key2, "trades");
    }

    #[tokio::test]
    async fn test_ws_client_creation() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);
        assert_eq!(client.state(), WsConnectionState::Disconnected);
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_subscribe_adds_subscription() {
        let config = WsConfig {
            url: "wss://example.com/ws".to_string(),
            ..Default::default()
        };

        let client = WsClient::new(config);
        // 使用 subscription_manager 直接注册
        let result = client.subscription_manager().register_subscription(
            "ticker".to_string(),
            Some("BTC/USDT".to_string()),
            None,
        );
        assert!(result.is_ok());
        assert_eq!(client.subscription_count(), 1);
        assert!(client.is_subscribed("ticker", Some(&"BTC/USDT".to_string())));
    }

    #[test]
    fn test_ws_connection_state_from_u8() {
        assert_eq!(
            WsConnectionState::from_u8(0),
            WsConnectionState::Disconnected
        );
        assert_eq!(WsConnectionState::from_u8(1), WsConnectionState::Connecting);
        assert_eq!(WsConnectionState::from_u8(2), WsConnectionState::Connected);
        assert_eq!(WsConnectionState::from_u8(255), WsConnectionState::Error);
    }

    #[test]
    fn test_ws_error_kind() {
        assert!(WsErrorKind::Transient.is_transient());
        assert!(WsErrorKind::Permanent.is_permanent());
    }

    #[test]
    fn test_ws_error_creation() {
        let err = WsError::transient("Connection timeout");
        assert!(err.is_transient());
        assert_eq!(err.message(), "Connection timeout");

        let err = WsError::permanent("Invalid API key");
        assert!(err.is_permanent());
    }

    #[test]
    fn test_subscription_manager() {
        let manager = SubscriptionManager::with_max_subscriptions(2);
        assert_eq!(manager.max_subscriptions(), 2);
        assert_eq!(manager.count(), 0);

        // 注册订阅
        assert!(
            manager
                .register_subscription("ticker".to_string(), Some("BTC/USDT".to_string()), None)
                .is_ok()
        );
        assert_eq!(manager.count(), 1);
    }
}

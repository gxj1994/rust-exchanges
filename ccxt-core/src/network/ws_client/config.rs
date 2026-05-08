//! WebSocket configuration types.

use rand::RngExt;
use std::time::Duration;
use tokio_tungstenite::tungstenite::protocol::Message;

/// Exponential backoff configuration for reconnection.
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Base delay for first retry (default: 1 second)
    pub base_delay: Duration,
    /// Maximum delay cap (default: 60 seconds)
    pub max_delay: Duration,
    /// Jitter factor (0.0 - 1.0, default: 0.25 for 25%)
    pub jitter_factor: f64,
    /// Multiplier for exponential growth (default: 2.0)
    pub multiplier: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(2), // 从 1s 改为 2s，减少频繁重连
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.25,
            multiplier: 2.0,
        }
    }
}

/// Calculates retry delay with exponential backoff and jitter.
#[derive(Debug, Clone)]
pub struct BackoffStrategy {
    config: BackoffConfig,
}

impl BackoffStrategy {
    /// Creates a new backoff strategy with the given configuration.
    #[must_use]
    pub fn new(config: BackoffConfig) -> Self {
        Self { config }
    }

    /// Creates a new backoff strategy with default configuration.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(BackoffConfig::default())
    }

    /// Returns a reference to the underlying configuration.
    #[must_use]
    pub fn config(&self) -> &BackoffConfig {
        &self.config
    }

    /// Calculates delay for the given attempt number.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    #[must_use]
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_ms = self.config.base_delay.as_millis() as f64;
        let multiplier = self.config.multiplier;
        let max_ms = self.config.max_delay.as_millis() as f64;

        let exponential_delay_ms = base_ms * multiplier.powi(attempt as i32);
        let capped_delay_ms = exponential_delay_ms.min(max_ms);

        let jitter_ms = if self.config.jitter_factor > 0.0 {
            let jitter_range = capped_delay_ms * self.config.jitter_factor;
            rand::rng().random::<f64>() * jitter_range
        } else {
            0.0
        };

        Duration::from_millis((capped_delay_ms + jitter_ms) as u64)
    }

    /// Calculates the base delay (without jitter) for the given attempt number.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    #[must_use]
    pub fn calculate_delay_without_jitter(&self, attempt: u32) -> Duration {
        let base_ms = self.config.base_delay.as_millis() as f64;
        let multiplier = self.config.multiplier;
        let max_ms = self.config.max_delay.as_millis() as f64;

        let exponential_delay_ms = base_ms * multiplier.powi(attempt as i32);
        let capped_delay_ms = exponential_delay_ms.min(max_ms);

        Duration::from_millis(capped_delay_ms as u64)
    }
}

/// Default maximum number of subscriptions.
pub const DEFAULT_MAX_SUBSCRIPTIONS: usize = 100;

/// Default shutdown timeout in milliseconds.
pub const DEFAULT_SHUTDOWN_TIMEOUT: u64 = 5000;

/// Default message channel capacity.
///
/// This is the maximum number of messages that can be buffered before
/// backpressure is applied. A value of 1000 provides a good balance
/// between memory usage and handling burst traffic.
pub const DEFAULT_MESSAGE_CHANNEL_CAPACITY: usize = 1000;

/// Default write channel capacity.
///
/// This is the maximum number of outgoing messages that can be buffered.
/// A smaller value (100) is used since writes are typically less frequent
/// than incoming messages.
pub const DEFAULT_WRITE_CHANNEL_CAPACITY: usize = 100;

/// Backpressure strategy when message channel is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackpressureStrategy {
    /// Drop the oldest message in the queue (default).
    /// This ensures the most recent data is always available.
    #[default]
    DropOldest,
    /// Drop the newest message (the one being sent).
    /// This preserves message ordering but may lose recent updates.
    DropNewest,
    /// Block until space is available.
    /// Warning: This can cause the WebSocket read loop to stall.
    Block,
}

/// 心跳模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeartbeatMode {
    /// 服务器主动发送 PING，客户端被动回复
    /// 适用于：Binance 现货、Binance 合约
    #[default]
    ServerInitiated,

    /// 客户端主动发送 PING
    /// 适用于：OKX、Bybit、Bitget
    ClientInitiated,

    /// 使用 WebSocket 协议层心跳（tokio-tungstenite 自动处理）
    /// 适用于：部分特殊场景
    ProtocolLevel,
}

/// WebSocket connection configuration.
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// WebSocket server URL
    pub url: String,
    /// Connection timeout in milliseconds
    pub connect_timeout: u64,
    /// 心跳模式
    pub heartbeat_mode: HeartbeatMode,
    /// 应用层心跳发送间隔（毫秒）
    /// 仅当 heartbeat_mode = ClientInitiated 时生效
    /// 例如：OKX=25000, Bybit=20000, Bitget=30000
    pub heartbeat_interval: u64,
    /// 应用层心跳超时（毫秒）
    /// 发送 PING 后，等待 PONG 的最大时间
    /// 超过此时间未收到 PONG，判定连接异常
    pub heartbeat_timeout: u64,
    /// Maximum reconnection attempts before giving up
    pub max_reconnect_attempts: u32,
    /// Enable automatic reconnection on disconnect
    pub auto_reconnect: bool,
    /// Enable message compression
    pub enable_compression: bool,
    /// Exponential backoff configuration for reconnection.
    pub backoff_config: BackoffConfig,
    /// Maximum number of subscriptions allowed.
    pub max_subscriptions: usize,
    /// Shutdown timeout in milliseconds.
    pub shutdown_timeout: u64,
    /// Message channel capacity (incoming messages buffer size).
    ///
    /// When this limit is reached, backpressure is applied according to
    /// `backpressure_strategy`. Default: 1000 messages.
    pub message_channel_capacity: usize,
    /// Write channel capacity (outgoing messages buffer size).
    ///
    /// Default: 100 messages.
    pub write_channel_capacity: usize,
    /// Strategy to use when message channel is full.
    ///
    /// Default: `DropOldest` to ensure most recent data is available.
    pub backpressure_strategy: BackpressureStrategy,
    /// Maximum message size in bytes.
    ///
    /// Default: 128 MiB (134,217,728 bytes) if None.
    pub max_message_size: Option<usize>,
    /// Maximum frame size in bytes.
    ///
    /// Default: 32 MiB (33,554,432 bytes) if None.
    pub max_frame_size: Option<usize>,
    /// 应用层心跳消息内容
    ///
    /// 用于 ClientInitiated 模式下的心跳发送。
    /// 各交易所可以配置自己的心跳消息格式：
    /// - OKX/Bitget: `Message::Text("ping".into())`
    /// - Bybit: `Message::Text(r#"{"op":"ping"}"#.into())`
    /// - Hyperliquid: `Message::Text(r#"{"method":"ping"}"#.into())`
    ///
    /// Default: `Message::Text("ping".into())`
    pub ping_message: Message,
    /// 初始连接重试次数
    ///
    /// 在首次建立连接时，如果连接失败，会自动重试。
    /// 这与 `max_reconnect_attempts`（运行中断连后的重连）不同。
    ///
    /// Default: `3`
    pub connect_retry_attempts: u32,
    /// 初始连接重试间隔（毫秒）
    ///
    /// 每次重试之间的等待时间。
    /// 使用固定间隔，不使用指数退避（因为初始连接时还没有连接状态）。
    ///
    /// Default: `1000` (1 秒)
    pub connect_retry_interval_ms: u64,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            connect_timeout: 10000,
            heartbeat_mode: HeartbeatMode::ServerInitiated, // 默认被动回复
            heartbeat_interval: 0,                          // 默认禁用（由各交易所配置）
            heartbeat_timeout: 0,                           // 默认禁用
            max_reconnect_attempts: 999, // Nearly unlimited reconnection attempts
            auto_reconnect: true,        // Enabled by default for better reliability
            enable_compression: false,
            backoff_config: BackoffConfig::default(),
            max_subscriptions: DEFAULT_MAX_SUBSCRIPTIONS,
            shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
            message_channel_capacity: 2000, // Increased to handle burst traffic
            write_channel_capacity: DEFAULT_WRITE_CHANNEL_CAPACITY,
            backpressure_strategy: BackpressureStrategy::default(),
            max_message_size: Some(128 * 1024 * 1024),
            max_frame_size: Some(32 * 1024 * 1024),
            ping_message: Message::Text("ping".into()), // 默认心跳消息
            connect_retry_attempts: 3,                  // 默认重试 3 次
            connect_retry_interval_ms: 1000,            // 默认间隔 1 秒
        }
    }
}

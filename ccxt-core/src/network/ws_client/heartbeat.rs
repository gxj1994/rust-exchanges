//! WebSocket 心跳管理器
//!
//! 统一管理应用层心跳机制，支持多种心跳模式：
//! - ServerInitiated: 服务器发送 PING，客户端回复 PONG
//! - ClientInitiated: 客户端主动发送 PING
//! - ProtocolLevel: 使用 WebSocket 协议层心跳

use crate::network::ws_client::WsConnectionState;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

/// 心跳管理器
#[derive(Debug)]
pub struct HeartbeatManager {
    /// 最后收到消息的时间戳（毫秒）
    last_message_time: Arc<AtomicI64>,
    /// 最后发送 PING 的时间戳（毫秒）
    last_ping_time: Arc<AtomicI64>,
    /// 最后收到 PONG 的时间戳（毫秒）
    last_pong_time: Arc<AtomicI64>,
    /// 连续心跳失败次数
    consecutive_failures: Arc<AtomicU32>,
    /// 心跳任务句柄
    task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl HeartbeatManager {
    /// 创建新的心跳管理器
    pub fn new() -> Self {
        Self {
            last_message_time: Arc::new(AtomicI64::new(0)),
            last_ping_time: Arc::new(AtomicI64::new(0)),
            last_pong_time: Arc::new(AtomicI64::new(0)),
            consecutive_failures: Arc::new(AtomicU32::new(0)),
            task_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// 记录收到消息
    pub fn record_message(&self) {
        self.last_message_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// 记录发送 PING
    pub fn record_ping(&self) {
        self.last_ping_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    /// 记录收到 PONG
    pub fn record_pong(&self) {
        self.last_pong_time
            .store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
        // 重置失败计数器
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }

    /// 获取最后消息时间
    pub fn last_message_time(&self) -> i64 {
        self.last_message_time.load(Ordering::Relaxed)
    }

    /// 获取最后 PONG 时间
    pub fn last_pong_time(&self) -> i64 {
        self.last_pong_time.load(Ordering::Relaxed)
    }

    /// 获取连续失败次数
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    /// 启动客户端主动心跳（OKX、Bybit、Bitget）
    ///
    /// # 参数
    /// - `write_tx`: 写通道，用于发送 PING 消息
    /// - `interval_ms`: 心跳发送间隔（毫秒）
    /// - `timeout_ms`: 心跳超时时间（毫秒）
    /// - `ping_message`: PING 消息内容
    /// - `state`: 连接状态
    /// - `cancel_token`: 取消令牌
    pub fn start_client_heartbeat(
        &self,
        write_tx: Arc<RwLock<Option<mpsc::Sender<Message>>>>,
        interval_ms: u64,
        timeout_ms: u64,
        ping_message: Message,
        state: Arc<std::sync::atomic::AtomicU8>,
        cancel_token: Option<CancellationToken>,
    ) {
        // 注意：last_message_time 暂不使用，预留用于未来扩展
        let _last_message_time = Arc::clone(&self.last_message_time);
        let last_ping_time = Arc::clone(&self.last_ping_time);
        let last_pong_time = Arc::clone(&self.last_pong_time);
        let consecutive_failures = Arc::clone(&self.consecutive_failures);

        // 最大连续失败次数
        const MAX_CONSECUTIVE_FAILURES: u32 = 3;

        // 启动心跳任务
        let _handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));

            debug!(
                interval_ms = interval_ms,
                timeout_ms = timeout_ms,
                "Client heartbeat task started"
            );

            loop {
                interval.tick().await;

                // 检查取消
                if cancel_token
                    .as_ref()
                    .map(|t| t.is_cancelled())
                    .unwrap_or(false)
                {
                    debug!("Heartbeat task cancelled");
                    break;
                }

                // 检查连接状态
                if state.load(Ordering::Acquire) != WsConnectionState::Connected as u8 {
                    debug!("Connection not in Connected state, skipping heartbeat");
                    continue;
                }

                let now = chrono::Utc::now().timestamp_millis();
                let last_pong = last_pong_time.load(Ordering::Relaxed);
                let last_ping = last_ping_time.load(Ordering::Relaxed);

                // 检查 PONG 超时
                // 修复：即使 last_pong 为 0，如果发送过 PING 也应该检测超时
                let should_check_timeout = if last_pong > 0 {
                    // 收到过 PONG，检查距离上次 PONG 的时间
                    true
                } else if last_ping > 0 {
                    // 没收到 PONG，但发送过 PING，也应该检查超时
                    true
                } else {
                    // 还没发送过 PING，跳过检查
                    false
                };

                if should_check_timeout {
                    // 使用 last_pong 或 last_ping 中较新的一个作为基准
                    let reference_time = if last_pong > 0 { last_pong } else { last_ping };
                    let elapsed = now - reference_time;

                    if elapsed > timeout_ms as i64 {
                        warn!(
                            elapsed_ms = elapsed,
                            timeout_ms = timeout_ms,
                            last_pong = last_pong,
                            last_ping = last_ping,
                            "Heartbeat timeout - no PONG received"
                        );

                        // 连续失败检测
                        let failures = consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                        if failures >= MAX_CONSECUTIVE_FAILURES {
                            error!(
                                failures = failures,
                                "Heartbeat failed {} times, marking connection as error", failures
                            );
                            // 标记连接为 Error 状态，触发重连
                            state.store(WsConnectionState::Error as u8, Ordering::Release);
                            break;
                        }
                    }
                }

                // 发送 PING
                if let Ok(tx_guard) = write_tx.try_read() {
                    if let Some(tx) = tx_guard.as_ref() {
                        match tx.try_send(ping_message.clone()) {
                            Ok(()) => {
                                last_ping_time.store(now, Ordering::Relaxed);
                                debug!("Sent heartbeat ping");
                            }
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                warn!("Write channel full, skipping heartbeat");
                            }
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                debug!("Write channel closed, stopping heartbeat");
                                break;
                            }
                        }
                    } else {
                        debug!("Write tx not initialized, skipping heartbeat");
                    }
                } else {
                    debug!("Failed to acquire write tx lock, skipping heartbeat");
                }
            }

            debug!("Client heartbeat task terminated");
        });

        // 存储任务句柄
        // 注意：这里需要同步存储，但 RwLock 需要异步操作
        // 我们在外部通过 async 方法设置
    }

    /// 设置心跳任务句柄（异步）
    pub async fn set_task_handle(&self, handle: tokio::task::JoinHandle<()>) {
        *self.task_handle.write().await = Some(handle);
    }

    /// 停止心跳任务
    pub async fn stop(&self) {
        if let Some(handle) = self.task_handle.write().await.take() {
            handle.abort();
            debug!("Heartbeat task stopped");
        }
    }

    /// 重置心跳状态
    pub fn reset(&self) {
        self.last_message_time.store(0, Ordering::Relaxed);
        self.last_ping_time.store(0, Ordering::Relaxed);
        self.last_pong_time.store(0, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
    }
}

impl Default for HeartbeatManager {
    fn default() -> Self {
        Self::new()
    }
}

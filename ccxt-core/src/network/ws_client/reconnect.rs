//! Automatic reconnection coordinator.

use super::config::{BackoffConfig, BackoffStrategy};
use super::error::WsError;
use super::event::{WsEvent, WsEventCallback};
use super::state::WsConnectionState;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{Duration, interval};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

/// Automatic reconnection coordinator for WebSocket connections.
pub struct AutoReconnectCoordinator {
    client: Arc<super::WsClient>,
    enabled: Arc<AtomicBool>,
    reconnect_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    event_callback: Option<WsEventCallback>,
    cancel_token: Arc<Mutex<Option<CancellationToken>>>,
}

impl std::fmt::Debug for AutoReconnectCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoReconnectCoordinator")
            .field("enabled", &self.enabled.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl AutoReconnectCoordinator {
    /// Creates a new automatic reconnection coordinator.
    pub fn new(client: Arc<super::WsClient>) -> Self {
        Self {
            client,
            enabled: Arc::new(AtomicBool::new(false)),
            reconnect_task: Arc::new(Mutex::new(None)),
            event_callback: None,
            cancel_token: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets the event callback for reconnection events.
    pub fn with_callback(mut self, callback: WsEventCallback) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// Sets the cancellation token for stopping reconnection.
    #[must_use]
    pub fn with_cancel_token(self, token: CancellationToken) -> Self {
        let _ = self.cancel_token.try_lock().map(|mut guard| {
            *guard = Some(token);
        });
        self
    }

    /// Sets the cancellation token asynchronously.
    pub async fn set_cancel_token(&self, token: CancellationToken) {
        let mut guard = self.cancel_token.lock().await;
        *guard = Some(token);
    }

    /// Returns whether auto-reconnect is currently enabled.
    #[inline]
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// 暂停自动重连（用于手动断开场景）
    ///
    /// 与 `stop()` 的区别：
    /// - `pause()`: 暂停重连，但保留 coordinator 状态，可以 `resume()` 恢复
    /// - `stop()`: 完全停止，需要重新 `start()` 才能恢复
    pub async fn pause(&self) {
        if self.enabled.swap(false, Ordering::SeqCst) {
            debug!("Auto-reconnect coordinator paused");
        }
        // 注意：不取消 cancel_token，不终止 reconnect_task
        // 只是让 enabled = false，reconnect_loop 会自然退出
    }

    /// 恢复自动重连（用于手动重连后场景）
    ///
    /// 必须在 `pause()` 后调用，如果 coordinator 从未启动过，此方法无效
    pub async fn resume(&self) {
        if self.enabled.swap(true, Ordering::SeqCst) {
            debug!("Auto-reconnect already enabled, ignoring resume");
            return;
        }

        debug!("Resuming auto-reconnect coordinator");

        // 如果 reconnect_task 已经结束，需要重新启动
        let task = self.reconnect_task.lock().await;
        if task.as_ref().map_or(true, |h| h.is_finished()) {
            // Task 已结束，需要重新启动
            drop(task); // 释放锁
            self.start().await;
        }
    }

    /// Starts the automatic reconnection coordinator.
    pub async fn start(&self) {
        if self.enabled.swap(true, Ordering::SeqCst) {
            info!("Auto-reconnect already started");
            return;
        }

        info!("Starting auto-reconnect coordinator with exponential backoff");

        let client = Arc::clone(&self.client);
        let enabled = Arc::clone(&self.enabled);
        let callback = self.event_callback.clone();
        let cancel_token = self.cancel_token.lock().await.clone();

        let backoff_config = client.config().backoff_config.clone();

        let handle = tokio::spawn(async move {
            Self::reconnect_loop(client, enabled, callback, backoff_config, cancel_token).await;
        });

        *self.reconnect_task.lock().await = Some(handle);
    }

    /// Stops the automatic reconnection coordinator.
    pub async fn stop(&self) {
        if !self.enabled.swap(false, Ordering::SeqCst) {
            info!("Auto-reconnect already stopped");
            return;
        }

        info!("Stopping auto-reconnect coordinator");

        if let Some(token) = self.cancel_token.lock().await.as_ref() {
            token.cancel();
        }

        let mut task = self.reconnect_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn reconnect_loop(
        client: Arc<super::WsClient>,
        enabled: Arc<AtomicBool>,
        callback: Option<WsEventCallback>,
        backoff_config: BackoffConfig,
        cancel_token: Option<CancellationToken>,
    ) {
        let mut check_interval = interval(Duration::from_secs(1));
        let backoff = BackoffStrategy::new(backoff_config);
        let mut last_error: Option<String> = None;

        loop {
            if cancel_token
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled)
            {
                info!("Auto-reconnect cancelled via token");
                break;
            }

            check_interval.tick().await;

            if !enabled.load(Ordering::SeqCst) {
                debug!("Auto-reconnect disabled, exiting loop");
                break;
            }

            let state = client.state();

            if matches!(
                state,
                WsConnectionState::Disconnected | WsConnectionState::Error
            ) {
                let attempt = client.reconnect_count();
                let max_attempts = client.config().max_reconnect_attempts;

                if attempt >= max_attempts {
                    error!(
                        attempts = attempt,
                        max = max_attempts,
                        "Max reconnect attempts reached, stopping auto-reconnect"
                    );

                    if let Some(ref cb) = callback {
                        cb(WsEvent::ReconnectExhausted {
                            total_attempts: attempt,
                            last_error: last_error
                                .clone()
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        });
                    }
                    break;
                }

                let delay = backoff.calculate_delay(attempt);

                #[allow(clippy::cast_possible_truncation)]
                {
                    info!(
                        attempt = attempt + 1,
                        max = max_attempts,
                        delay_ms = delay.as_millis() as u64,
                        state = ?state,
                        "Connection lost, attempting reconnect with exponential backoff"
                    );
                }

                if let Some(ref cb) = callback {
                    cb(WsEvent::Reconnecting {
                        attempt: attempt + 1,
                        delay,
                        error: last_error.clone(),
                    });
                }

                if let Some(ref token) = cancel_token {
                    tokio::select! {
                        biased;
                        () = token.cancelled() => {
                            info!("Auto-reconnect cancelled during backoff delay");
                            break;
                        }
                        () = tokio::time::sleep(delay) => {}
                    }
                } else {
                    tokio::time::sleep(delay).await;
                }

                if cancel_token
                    .as_ref()
                    .is_some_and(CancellationToken::is_cancelled)
                {
                    info!("Auto-reconnect cancelled after backoff delay");
                    break;
                }

                client.set_state(WsConnectionState::Reconnecting);

                match client.connect().await {
                    Ok(()) => {
                        info!(attempt = attempt + 1, "Reconnection successful");
                        client.reset_reconnect_count();
                        last_error = None;

                        if let Some(ref cb) = callback {
                            cb(WsEvent::ReconnectSuccess);
                        }

                        // 注意: resubscribe_all() 已经在 client.connect() -> connect_once() 内部调用
                        // 不需要在此处再次调用，避免重复订阅
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        let ws_error = WsError::from_error(&e);
                        let is_permanent = ws_error.is_permanent();

                        error!(
                            attempt = attempt + 1,
                            error = %e,
                            is_permanent = is_permanent,
                            "Reconnection failed"
                        );

                        last_error = Some(error_msg.clone());
                        client.increment_reconnect_count();

                        if let Some(ref cb) = callback {
                            cb(WsEvent::ReconnectFailed {
                                attempt: attempt + 1,
                                error: error_msg,
                                is_permanent,
                            });
                        }

                        if is_permanent {
                            if let Some(ref cb) = callback {
                                cb(WsEvent::PermanentError {
                                    error: e.to_string(),
                                });
                            }
                            break;
                        }
                    }
                }
            }
        }

        info!("Auto-reconnect loop terminated");
    }

    /// Convert into a stream (requires `tokio-stream`).
    #[cfg(feature = "websocket")]
    pub fn into_stream(self) -> impl futures::Stream<Item = serde_json::Value> {
        let client = self.client.clone();
        let coordinator = Arc::new(self);

        async_stream::stream! {
            // Start reconnection in background
            coordinator.start().await;

            loop {
                if let Some(msg) = client.receive().await {
                    yield msg;
                } else {
                    // Check if we should stop
                    if !coordinator.is_enabled() {
                        break;
                    }
                    // Wait a bit before retry if channel closed (shouldn't happen with reconnect)
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            coordinator.stop().await;
        }
    }
}

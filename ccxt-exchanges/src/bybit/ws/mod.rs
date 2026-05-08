//! Bybit WebSocket v2 实现
//!
//! 使用统一架构的 Bybit WebSocket 客户端

mod builder;
mod parser;

pub use builder::BybitSubscriptionBuilder;
pub use parser::BybitStreamParser;

// 从 network 模块导入 WsEndpointProvider（统一管理）
pub use crate::bybit::network::endpoint_router::BybitWsEndpointProvider;

use crate::bybit::auth::BybitWsAuth;
use ccxt_core::network::ws_client::{HeartbeatMode, WsConfig};
use ccxt_core::ws::{GenericWsClient, NoAuth};
use tokio_tungstenite::tungstenite::protocol::Message;

/// Bybit WebSocket 客户端类型别名（无认证）
///
/// 用于公共频道订阅
pub type BybitWsClient =
    GenericWsClient<BybitSubscriptionBuilder, BybitStreamParser, BybitWsEndpointProvider, NoAuth>;

/// Bybit WebSocket 客户端类型别名（带认证）
///
/// 用于私有频道订阅（balance, orders, account_trades 等）
pub type BybitWsClientAuth = GenericWsClient<
    BybitSubscriptionBuilder,
    BybitStreamParser,
    BybitWsEndpointProvider,
    BybitWsAuth,
>;

/// 创建 Bybit WebSocket 客户端（无认证）
///
/// 用于公共频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
pub fn create_bybit_ws_client(is_sandbox: bool) -> BybitWsClient {
    // Bybit 心跳配置：客户端主动发送，20 秒间隔（官方要求）
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 20000,                              // 20 秒
        heartbeat_timeout: 10000,                               // 10 秒超时
        ping_message: Message::Text(r#"{"op":"ping"}"#.into()), // Bybit 格式
        ..Default::default()
    };

    BybitWsClient::with_config(
        BybitSubscriptionBuilder,
        BybitStreamParser,
        BybitWsEndpointProvider::new(is_sandbox),
        NoAuth,
        ws_config,
    )
}

/// 创建 Bybit WebSocket 客户端（带认证）
///
/// 用于私有频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
/// - `auth`: Bybit WebSocket 认证策略
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::bybit::{BybitAuth, BybitWsAuth};
/// use ccxt_exchanges::bybit::ws::create_bybit_ws_client_auth;
///
/// let auth = BybitAuth::new(
///     "api-key".to_string(),
///     "secret".to_string(),
/// );
/// let ws_auth = BybitWsAuth::new(auth);
/// let client = create_bybit_ws_client_auth(false, ws_auth);
/// ```
pub fn create_bybit_ws_client_auth(is_sandbox: bool, auth: BybitWsAuth) -> BybitWsClientAuth {
    // Bybit 心跳配置：客户端主动发送，20 秒间隔
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 20000,                              // 20 秒
        heartbeat_timeout: 10000,                               // 10 秒超时
        ping_message: Message::Text(r#"{"op":"ping"}"#.into()), // Bybit 格式
        ..Default::default()
    };

    BybitWsClientAuth::with_config(
        BybitSubscriptionBuilder,
        BybitStreamParser,
        BybitWsEndpointProvider::new(is_sandbox),
        auth,
        ws_config,
    )
}

//! OKX WebSocket v2 实现
//!
//! 使用统一架构的 OKX WebSocket 客户端

mod builder;
mod parser;

pub use builder::OkxSubscriptionBuilder;
pub use parser::OkxStreamParser;

// 从 network 模块导入 WsEndpointProvider（统一管理）
pub use crate::okx::network::endpoint_router::OkxWsEndpointProvider;

use crate::okx::auth::OkxWsAuth;
use ccxt_core::network::ws_client::{HeartbeatMode, WsConfig};
use ccxt_core::ws::{GenericWsClient, NoAuth};
use tokio_tungstenite::tungstenite::protocol::Message;

/// OKX WebSocket 客户端类型别名（无认证）
///
/// 用于公共频道订阅
pub type OkxWsClient =
    GenericWsClient<OkxSubscriptionBuilder, OkxStreamParser, OkxWsEndpointProvider, NoAuth>;

/// OKX WebSocket 客户端类型别名（带认证）
///
/// 用于私有频道订阅（balance, orders, account_trades 等）
pub type OkxWsClientAuth =
    GenericWsClient<OkxSubscriptionBuilder, OkxStreamParser, OkxWsEndpointProvider, OkxWsAuth>;

/// 创建 OKX WebSocket 客户端（无认证）
///
/// 用于公共频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
pub fn create_okx_ws_client(is_sandbox: bool) -> OkxWsClient {
    // OKX 心跳配置：客户端主动发送，25 秒间隔（< 30s 要求）
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 25000,                  // 25 秒
        heartbeat_timeout: 30000,                   // 30 秒超时（给予足够的响应时间）
        ping_message: Message::Text("ping".into()), // OKX 使用字符串 "ping"
        ..Default::default()
    };

    OkxWsClient::with_config(
        OkxSubscriptionBuilder,
        OkxStreamParser,
        OkxWsEndpointProvider::new(is_sandbox),
        NoAuth,
        ws_config,
    )
}

/// 创建 OKX WebSocket 客户端（带认证）
///
/// 用于私有频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
/// - `auth`: OKX WebSocket 认证策略
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::okx::{OkxAuth, OkxWsAuth};
/// use ccxt_exchanges::okx::ws::create_okx_ws_client_auth;
///
/// let auth = OkxAuth::new(
///     "api-key".to_string(),
///     "secret".to_string(),
///     "passphrase".to_string(),
/// );
/// let ws_auth = OkxWsAuth::new(auth);
/// let client = create_okx_ws_client_auth(false, ws_auth);
/// ```
pub fn create_okx_ws_client_auth(is_sandbox: bool, auth: OkxWsAuth) -> OkxWsClientAuth {
    // OKX 心跳配置：客户端主动发送，25 秒间隔
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 25000,                  // 25 秒
        heartbeat_timeout: 30000,                   // 30 秒超时（给予足够的响应时间）
        ping_message: Message::Text("ping".into()), // OKX 使用字符串 "ping"
        ..Default::default()
    };

    OkxWsClientAuth::with_config(
        OkxSubscriptionBuilder,
        OkxStreamParser,
        OkxWsEndpointProvider::new(is_sandbox),
        auth,
        ws_config,
    )
}

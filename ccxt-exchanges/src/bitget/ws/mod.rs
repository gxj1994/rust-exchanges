//! Bitget WebSocket v2 实现
//!
//! 使用统一架构的 Bitget WebSocket 客户端

mod builder;
mod parser;

pub use builder::BitgetSubscriptionBuilder;
pub use parser::BitgetStreamParser;

// 从 network 模块导入 WsEndpointProvider（统一管理）
pub use crate::bitget::network::endpoint_router::BitgetWsEndpointProvider;

use crate::bitget::auth::BitgetWsAuth;
use ccxt_core::network::ws_client::{HeartbeatMode, WsConfig};
use ccxt_core::ws::{GenericWsClient, NoAuth};

/// Bitget WebSocket 客户端类型别名（无认证）
///
/// 用于公共频道订阅
pub type BitgetWsClient = GenericWsClient<
    BitgetSubscriptionBuilder,
    BitgetStreamParser,
    BitgetWsEndpointProvider,
    NoAuth,
>;

/// Bitget WebSocket 客户端类型别名（带认证）
///
/// 用于私有频道订阅（balance, orders, account_trades 等）
pub type BitgetWsClientAuth = GenericWsClient<
    BitgetSubscriptionBuilder,
    BitgetStreamParser,
    BitgetWsEndpointProvider,
    BitgetWsAuth,
>;

/// 创建 Bitget WebSocket 客户端（无认证）
///
/// 用于公共频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
pub fn create_bitget_ws_client(is_sandbox: bool) -> BitgetWsClient {
    // Bitget 心跳配置：客户端主动发送，30 秒间隔（官方要求）
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 30000, // 30 秒
        heartbeat_timeout: 10000,  // 10 秒超时
        ..Default::default()
    };

    BitgetWsClient::with_config(
        BitgetSubscriptionBuilder,
        BitgetStreamParser,
        BitgetWsEndpointProvider::new(is_sandbox),
        NoAuth,
        ws_config,
    )
}

/// 创建 Bitget WebSocket 客户端（带认证）
///
/// 用于私有频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
/// - `auth`: Bitget WebSocket 认证策略
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::bitget::{BitgetAuth, BitgetWsAuth};
/// use ccxt_exchanges::bitget::ws::create_bitget_ws_client_auth;
///
/// let auth = BitgetAuth::new(
///     "api-key".to_string(),
///     "secret".to_string(),
///     "passphrase".to_string(),
/// );
/// let ws_auth = BitgetWsAuth::new(auth);
/// let client = create_bitget_ws_client_auth(false, ws_auth);
/// ```
pub fn create_bitget_ws_client_auth(is_sandbox: bool, auth: BitgetWsAuth) -> BitgetWsClientAuth {
    // Bitget 心跳配置：客户端主动发送，30 秒间隔
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 30000, // 30 秒
        heartbeat_timeout: 10000,  // 10 秒超时
        ..Default::default()
    };

    BitgetWsClientAuth::with_config(
        BitgetSubscriptionBuilder,
        BitgetStreamParser,
        BitgetWsEndpointProvider::new(is_sandbox),
        auth,
        ws_config,
    )
}

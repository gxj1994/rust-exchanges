//! Hyperliquid WebSocket v2 实现
//!
//! 使用统一架构的 Hyperliquid WebSocket 客户端

mod builder;
mod parser;

pub use builder::HyperliquidSubscriptionBuilder;
pub use parser::HyperliquidStreamParser;

// 从 network 模块导入 WsEndpointProvider（统一管理）
pub use crate::hyperliquid::network::endpoint_router::HyperliquidWsEndpointProvider;

use crate::hyperliquid::auth::HyperliquidWsAuth;
use ccxt_core::network::ws_client::{HeartbeatMode, WsConfig};
use ccxt_core::ws::{GenericWsClient, NoAuth};
use tokio_tungstenite::tungstenite::protocol::Message;

/// Hyperliquid WebSocket 客户端类型别名（无认证）
///
/// 用于公共频道订阅
pub type HyperliquidWsClient = GenericWsClient<
    HyperliquidSubscriptionBuilder,
    HyperliquidStreamParser,
    HyperliquidWsEndpointProvider,
    NoAuth,
>;

/// Hyperliquid WebSocket 客户端类型别名（带认证）
///
/// 用于私有频道订阅（orders, account 等）
pub type HyperliquidWsClientAuth = GenericWsClient<
    HyperliquidSubscriptionBuilder,
    HyperliquidStreamParser,
    HyperliquidWsEndpointProvider,
    HyperliquidWsAuth,
>;

/// 创建 Hyperliquid WebSocket 客户端（无认证）
///
/// 用于公共频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
pub fn create_hyperliquid_ws_client(is_sandbox: bool) -> HyperliquidWsClient {
    // Hyperliquid 心跳配置：客户端主动发送，50 秒间隔（< 60s 要求）
    // 仅低频频道需要，高频频道（如 L2Book）不需要
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ClientInitiated,
        heartbeat_interval: 50000,                                  // 50 秒
        heartbeat_timeout: 10000,                                   // 10 秒超时
        ping_message: Message::Text(r#"{"method":"ping"}"#.into()), // Hyperliquid 格式
        ..Default::default()
    };

    HyperliquidWsClient::with_config(
        HyperliquidSubscriptionBuilder,
        HyperliquidStreamParser,
        HyperliquidWsEndpointProvider::new(is_sandbox),
        NoAuth,
        ws_config,
    )
}

/// 创建 Hyperliquid WebSocket 客户端（带认证）
///
/// 用于私有频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
/// - `auth`: Hyperliquid WebSocket 认证策略
///
/// # 示例
///
/// ```rust,ignore
/// use ccxt_exchanges::hyperliquid::{HyperLiquidAuth, HyperliquidWsAuth};
/// use ccxt_exchanges::hyperliquid::ws::create_hyperliquid_ws_client_auth;
///
/// let auth = HyperLiquidAuth::from_private_key(
///     "0x1234..."
/// )?;
/// let ws_auth = HyperliquidWsAuth::new(auth, true);
/// let client = create_hyperliquid_ws_client_auth(false, ws_auth);
/// ```
pub fn create_hyperliquid_ws_client_auth(
    is_sandbox: bool,
    auth: HyperliquidWsAuth,
) -> HyperliquidWsClientAuth {
    HyperliquidWsClientAuth::new(
        HyperliquidSubscriptionBuilder,
        HyperliquidStreamParser,
        HyperliquidWsEndpointProvider::new(is_sandbox),
        auth,
    )
}

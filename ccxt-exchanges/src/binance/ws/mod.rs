//! Binance WebSocket v2 实现
//!
//! 使用统一架构的 Binance WebSocket 客户端

mod builder;
mod parser;

pub use builder::BinanceSubscriptionBuilder;
pub use parser::BinanceStreamParser;

// 从 network 模块导入 WsEndpointProvider（统一管理）
pub use crate::binance::network::endpoint_router::BinanceWsEndpointProvider;

use crate::binance::auth::BinanceWsAuth;
use ccxt_core::network::ws_client::config::{HeartbeatMode, WsConfig};
use ccxt_core::ws::{GenericWsClient, NoAuth};

/// Binance WebSocket 客户端类型别名（无认证）
///
/// 用于公共频道订阅
pub type BinanceWsClient = GenericWsClient<
    BinanceSubscriptionBuilder,
    BinanceStreamParser,
    BinanceWsEndpointProvider,
    NoAuth,
>;

/// Binance WebSocket 客户端类型别名（带认证）
///
/// 用于私有频道订阅（balance, orders, account_trades 等）
/// 需要 listenKey 管理
pub type BinanceWsClientAuth = GenericWsClient<
    BinanceSubscriptionBuilder,
    BinanceStreamParser,
    BinanceWsEndpointProvider,
    BinanceWsAuth,
>;

/// 创建 Binance WebSocket 客户端（无认证）
///
/// 用于公共频道订阅
///
/// # 参数
///
/// - `is_sandbox`: 是否使用沙箱环境
pub fn create_binance_ws_client(is_sandbox: bool) -> BinanceWsClient {
    // Binance 心跳配置：
    // - 现货：服务器每 20 秒发送应用层 PING ({"id":123})，客户端需回复相同 payload
    // - 合约：使用 WebSocket 协议层心跳
    // 使用 ServerInitiated 模式，等待服务器的 PING 并回复
    let ws_config = WsConfig {
        heartbeat_mode: HeartbeatMode::ServerInitiated, // 等待服务器 PING
        heartbeat_timeout: 25000,                       // 25 秒超时（略大于服务器的 20 秒）
        ..Default::default()
    };

    BinanceWsClient::with_config(
        BinanceSubscriptionBuilder,
        BinanceStreamParser,
        BinanceWsEndpointProvider::new(is_sandbox),
        NoAuth,
        ws_config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_aliases() {
        // 验证类型别名正确编译
        fn _assert_ws_client<T: ccxt_core::ws::WsEndpointProvider>() {}
        _assert_ws_client::<BinanceWsEndpointProvider>();
    }
}

//! WebSocket 统一架构模块
//!
//! 提供统一的 WebSocket 框架，用于加密货币交易所的流式 API。
//!
//! # 架构设计
//!
//! 本模块采用 Trait + 泛型组合模式，抽离公共组件：
//! - `SubscriptionBuilder`: 订阅消息构建（解决订阅格式差异）
//! - `StreamParser`: 消息解析（解决消息格式差异）
//! - `WsEndpointProvider`: WebSocket 端点提供（解决多 URL 问题）
//! - `WsAuthCore`: 认证策略（解决认证模式差异）
//! - `GenericWsClient`: 泛型客户端（组合以上 trait）
//!
//! # 认证架构
//!
//! 采用接口隔离原则 (ISP)，认证功能拆分为多个小 trait：
//! - `WsAuthCore`: 认证核心（必需）
//! - `TokenProvider`: Token 预认证（Binance, Kraken, KuCoin）
//! - `MessageAuthenticator`: 消息认证（OKX, Bybit, Bitget）
//! - `SubscribeAuthenticator`: 订阅时认证（Coinbase）
//! - `RequestSigner`: 请求级签名（Hyperliquid）
//!
//! # 订阅管理
//!
//! `SubscriptionManager` 已迁移到 `network::ws_client` 模块。
//! 请使用 `WsClient.subscription_manager()` 访问订阅管理功能。
//!
//! # 示例
//!
//! ```rust,ignore
//! use ccxt_core::ws::*;
//!
//! // 定义交易所特定的实现
//! pub struct OkxSubscriptionBuilder;
//! pub struct OkxStreamParser;
//! pub struct OkxWsEndpointProvider;
//! pub struct OkxWsAuth;  // 实现 MessageAuthenticator
//!
//! // 类型别名（带认证）
//! pub type OkxWsClient = GenericWsClient<
//!     OkxSubscriptionBuilder,
//!     OkxStreamParser,
//!     OkxWsEndpointProvider,
//!     OkxWsAuth,  // 认证策略
//! >;
//! ```

pub mod auth;
pub mod core;
pub mod parser;
pub mod subscription;

// 重导出核心类型
pub use auth::{
    AddressSubscription, AuthMode, AuthToken, ConnectionLifecycle, MessageAuthenticator, NoAuth,
    RequestSigner, SubscribeAuthenticator, TokenDelivery, TokenProvider, WsAuthCore,
};
pub use core::{GenericWsClient, WsContext, WsEndpointProvider};
pub use parser::{
    ExchangeMessage, OrderBookDeltaParser, OrderBookMessageType, ParseResult, Parseable,
    ParsedMessage, StreamParser, StreamParserExt, define_parseable,
};
pub use subscription::{BroadcastStrategy, ChannelType, SubscriptionBuilder, SubscriptionChannel};

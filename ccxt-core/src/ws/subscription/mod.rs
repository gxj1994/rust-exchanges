//! 订阅管理模块
//!
//! 提供订阅构建器 trait、订阅频道类型等
//!
//! # 注意
//!
//! `SubscriptionManager` 已迁移到 `network::ws_client` 模块。
//! 请使用 `WsClient.subscription_manager()` 访问订阅管理功能。

mod r#trait;
mod types;

pub use r#trait::SubscriptionBuilder;
pub use types::{BroadcastStrategy, ChannelType, MarketType, SubscriptionChannel};

//! WebSocket 核心模块
//!
//! 提供泛型客户端和端点提供者

mod client;
mod endpoint;

pub use client::GenericWsClient;
pub use endpoint::{WsContext, WsEndpointProvider};

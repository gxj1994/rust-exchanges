//! WebSocket 认证策略模块
//!
//! 采用接口隔离原则 (ISP) 设计，将认证功能拆分为多个小 trait，
//! 交易所按需实现所需的认证能力。
//!
//! # 架构
//!
//! ```text
//! WsAuthCore (必需)
//! ├── TokenProvider (Token 预认证)
//! │   └── Binance, Kraken, KuCoin
//! ├── MessageAuthenticator (消息认证)
//! │   └── OKX, Bybit, Bitget
//! ├── SubscribeAuthenticator (订阅时认证)
//! │   └── Coinbase
//! └── RequestSigner (请求级签名)
//!     └── Hyperliquid
//! ```
//!
//! # 示例
//!
//! ```rust,ignore
//! // OKX 消息认证
//! let auth = OkxWsAuth::new(api_key, passphrase, secret_key);
//! let client = GenericWsClient::new(
//!     OkxSubscriptionBuilder,
//!     OkxStreamParser,
//!     OkxWsEndpointProvider,
//!     auth,  // 传入认证策略
//! );
//! client.connect_with_login().await?;
//! ```

mod no_auth;
mod traits;

pub use no_auth::NoAuth;
pub use traits::{
    AddressSubscription, AuthMode, AuthToken, ConnectionLifecycle, MessageAuthenticator,
    RequestSigner, SubscribeAuthenticator, TokenDelivery, TokenProvider, WsAuthCore,
};

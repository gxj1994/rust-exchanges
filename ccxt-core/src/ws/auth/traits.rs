//! WebSocket 认证策略 Trait 定义
//!
//! 基于 Interface Segregation Principle (ISP) 设计，
//! 每个 trait 只负责一种认证模式。

use crate::error::Result;
use serde_json::Value;
use std::future::Future;

/// 认证模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// 无认证
    None,
    /// Token 预认证（连接前获取 Token）
    TokenPreAuth,
    /// 消息认证（连接后发送登录消息）
    MessageAuth,
    /// 订阅时认证（订阅消息携带签名）
    SubscribeAuth,
    /// 请求级签名（每个请求单独签名）
    RequestSign,
    /// 地址订阅（DEX 专用）
    AddressSubscribe,
}

/// 认证令牌
#[derive(Debug, Clone)]
pub struct AuthToken {
    /// 令牌值
    pub token: String,
    /// 过期时间（毫秒时间戳，None 表示永不过期）
    pub expires_at: Option<i64>,
}

impl AuthToken {
    /// 创建新的认证令牌
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            expires_at: None,
        }
    }

    /// 创建带过期时间的令牌
    pub fn with_expiry(token: impl Into<String>, expires_at: i64) -> Self {
        Self {
            token: token.into(),
            expires_at: Some(expires_at),
        }
    }

    /// 检查令牌是否已过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            now >= expires
        } else {
            false
        }
    }
}

/// Token 传递方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenDelivery {
    /// URL 路径中传递 (Binance: wss://stream.binance.com/ws/{listenKey})
    UrlPath,
    /// URL 查询参数传递 (KuCoin: wss://ws.kucoin.com?token={token})
    UrlQuery,
    /// 订阅消息中传递 (Kraken: subscription.token)
    SubscribeMessage,
    /// 连接 Header 中传递
    ConnectHeaders,
}

// ============================================================================
// 核心 Trait
// ============================================================================

/// WebSocket 认证核心（必需实现）
///
/// 所有认证策略必须实现此 trait，用于标识认证模式。
pub trait WsAuthCore: Clone + Send + Sync + 'static {
    /// 返回认证模式
    fn mode(&self) -> AuthMode;

    /// 是否需要认证
    fn needs_auth(&self) -> bool {
        self.mode() != AuthMode::None
    }
}

// ============================================================================
// Token 预认证模式
// ============================================================================

/// Token 提供者（Token 预认证模式）
///
/// 适用于需要在连接前获取 Token 的交易所：
/// - Binance (listenKey)
/// - Kraken (WebSocket token)
/// - KuCoin (connect token)
///
/// # 示例
///
/// ```rust,ignore
/// impl TokenProvider for BinanceWsAuth {
///     fn delivery(&self) -> TokenDelivery { TokenDelivery::UrlPath }
///     async fn get_token(&self) -> Result<AuthToken> {
///         // POST /api/v3/userDataStream 创建 listenKey
///     }
///     fn needs_renewal(&self) -> bool { true }
///     async fn renew(&self) -> Result<()> {
///         // PUT /api/v3/userDataStream?listenKey={key} 续期
///     }
/// }
/// ```
pub trait TokenProvider: WsAuthCore {
    /// 返回 Token 传递方式
    fn delivery(&self) -> TokenDelivery;

    /// 获取认证 Token
    ///
    /// 对于 Binance，这会创建 listenKey
    /// 对于 KuCoin，这会从 REST API 获取 connect token
    fn get_token(&self) -> impl Future<Output = Result<AuthToken>> + Send;

    /// 修改 URL 以包含 Token
    ///
    /// 根据 `delivery()` 的返回值决定如何修改：
    /// - UrlPath: 追加到路径
    /// - UrlQuery: 添加查询参数
    fn modify_url(&self, url: &str, token: &AuthToken) -> String;

    /// 是否需要续期
    ///
    /// Binance listenKey 需要每 30 分钟续期一次
    fn needs_renewal(&self) -> bool {
        false
    }

    /// 续期 Token
    ///
    /// 对于 Binance，这会发送 PUT 请求续期 listenKey
    fn renew(&self) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }

    /// 清理 Token（连接关闭时调用）
    ///
    /// 对于 Binance，这会发送 DELETE 请求删除 listenKey
    fn cleanup(&self) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }

    /// 续期间隔（秒）
    ///
    /// 默认 30 分钟，交易所可覆盖
    fn renewal_interval_secs(&self) -> u64 {
        1800 // 30 minutes
    }
}

// ============================================================================
// 消息认证模式
// ============================================================================

/// 消息认证器（消息认证模式）
///
/// 适用于连接后发送登录消息的交易所：
/// - OKX
/// - Bybit
/// - Bitget
/// - Gate.io
///
/// # 示例
///
/// ```rust,ignore
/// impl MessageAuthenticator for OkxWsAuth {
///     fn login_message(&self) -> Option<Value> {
///         Some(json!({
///             "op": "login",
///             "args": [{
///                 "apiKey": self.api_key,
///                 "passphrase": self.passphrase,
///                 "timestamp": timestamp,
///                 "sign": signature
///             }]
///         }))
///     }
///     fn is_success(&self, msg: &Value) -> bool {
///         msg.get("event") == Some(&json!("login"))
///             && msg.get("code") == Some(&json!("0"))
///     }
/// }
/// ```
pub trait MessageAuthenticator: WsAuthCore {
    /// 构建登录消息
    ///
    /// 返回 None 表示不需要登录
    fn login_message(&self) -> Option<Value>;

    /// 检查是否登录成功
    fn is_success(&self, msg: &Value) -> bool;

    /// 检查是否登录失败
    fn is_failure(&self, msg: &Value) -> bool;

    /// 登录超时（毫秒）
    fn login_timeout_ms(&self) -> u64 {
        10000 // 10 seconds
    }
}

// ============================================================================
// 订阅时认证模式
// ============================================================================

/// 订阅认证器（订阅时认证模式）
///
/// 适用于订阅消息需要携带签名的交易所：
/// - Coinbase
///
/// # 示例
///
/// ```rust,ignore
/// impl SubscribeAuthenticator for CoinbaseWsAuth {
///     async fn authenticate(&self, subscribe: Value) -> Result<Value> {
///         // 添加签名到订阅消息
///         let signature = self.sign_subscribe(&subscribe)?;
///         let mut result = subscribe;
///         result["signature"] = json!(signature);
///         Ok(result)
///     }
/// }
/// ```
pub trait SubscribeAuthenticator: WsAuthCore {
    /// 认证订阅消息
    ///
    /// 接收原始订阅消息，返回带签名的订阅消息
    fn authenticate(&self, subscribe: Value) -> impl Future<Output = Result<Value>> + Send;
}

// ============================================================================
// 请求级签名模式
// ============================================================================

/// 请求签名器（请求级签名模式）
///
/// 适用于每个请求都需要单独签名的交易所：
/// - Hyperliquid
///
/// # 示例
///
/// ```rust,ignore
/// impl RequestSigner for HyperliquidWsAuth {
///     fn sign(&self, request: &mut Value) -> Result<()> {
///         let action = request["action"].as_str().unwrap_or("");
///         let nonce = chrono::Utc::now().timestamp_millis();
///         let signature = self.sign_action(action, nonce)?;
///         request["nonce"] = json!(nonce);
///         request["signature"] = json!(signature);
///         Ok(())
///     }
/// }
/// ```
pub trait RequestSigner: WsAuthCore {
    /// 签名请求
    ///
    /// 直接修改请求对象，添加签名字段
    fn sign(&self, request: &mut Value) -> Result<()>;
}

// ============================================================================
// 地址订阅模式（DEX）
// ============================================================================

/// 地址订阅（DEX 专用）
///
/// 适用于需要按地址订阅的 DEX：
/// - dYdX
///
/// # 示例
///
/// ```rust,ignore
/// impl AddressSubscription for DydxWsAuth {
///     fn address(&self) -> Option<String> {
///         Some(self.wallet_address.clone())
///     }
/// }
/// ```
pub trait AddressSubscription: WsAuthCore {
    /// 返回钱包地址
    fn address(&self) -> Option<String>;
}

// ============================================================================
// 连接生命周期钩子
// ============================================================================

/// 连接生命周期钩子（可选）
///
/// 提供连接生命周期的回调钩子，用于：
/// - 连接前准备
/// - 连接后清理
/// - 断开连接时的处理
///
/// # 示例
///
/// ```rust,ignore
/// impl ConnectionLifecycle for MyAuth {
///     async fn on_connect(&self) -> Result<()> {
///         tracing::info!("Preparing to connect...");
///         Ok(())
///     }
///     async fn on_connected(&self) -> Result<()> {
///         tracing::info!("Connected successfully");
///         Ok(())
///     }
///     async fn on_disconnect(&self) -> Result<()> {
///         // 清理资源
///         self.cleanup().await
///     }
/// }
/// ```
pub trait ConnectionLifecycle: Clone + Send + Sync + 'static {
    /// 连接前调用
    fn on_connect(&self) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }

    /// 连接成功后调用
    fn on_connected(&self) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }

    /// 断开连接时调用
    fn on_disconnect(&self) -> impl Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }
}

//! 统一端点管理模块
//!
//! 提供REST和WebSocket端点的统一管理，消除重复代码。
//!
//! # 架构设计
//!
//! 采用"单一数据源，双向接口适配"的设计理念：
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │              ExchangeEndpoints                      │
//! │  ┌─────────────────┐  ┌──────────────────┐         │
//! │  │ RestEndpoints   │  │  WsEndpoints     │         │
//! │  │ - spot          │  │  - public        │         │
//! │  │ - linear_swap   │  │  - private       │         │
//! │  │ - inverse_swap  │  │  - by_market     │         │
//! │  │ - option        │  │  - by_channel    │         │
//! │  │ - special       │  │                  │         │
//! │  └─────────────────┘  └──────────────────┘         │
//! └─────────────────────────────────────────────────────┘
//!                      │
//!         ┌────────────┴────────────┐
//!         │                         │
//!         ▼                         ▼
//! ┌──────────────────┐   ┌──────────────────────┐
//! │ ExchangeEndpoint │   │ WsEndpointProvider   │
//! │ Manager          │   │ (自动实现)           │
//! └──────────────────┘   └──────────────────────┘
//! ```
//!
//! # 核心特性
//!
//! 1. **单一数据源**: REST和WebSocket端点统一配置
//! 2. **自动桥接**: 实现 `ExchangeEndpointManager` 后自动获得 `WsEndpointProvider` 能力
//! 3. **类型安全**: 强类型枚举替代字符串HashMap
//! 4. **零重复代码**: sandbox切换、端点路由等逻辑只写一次
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use ccxt_core::network::endpoint_manager::{
//!     ExchangeEndpointManager, ExchangeEndpoints, RestEndpoints, WsEndpoints,
//! };
//!
//! // 交易所实现
//! impl ExchangeEndpointManager for MyExchange {
//!     fn endpoints(&self) -> &ExchangeEndpoints {
//!         &MY_ENDPOINTS
//!     }
//! }
//!
//! // WsEndpointProvider 自动实现！
//! // 可以直接用于 GenericWsClient
//! ```

use crate::types::market::Market;
use crate::ws::WsContext;
use crate::ws::WsEndpointProvider;
use std::collections::HashMap;

// ============================================================================
// WebSocket频道类型
// ============================================================================

/// WebSocket频道类型
///
/// 用于区分不同用途的WebSocket端点
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WsChannel {
    /// 公共频道 (行情数据)
    Public,
    /// 私有频道 (账户数据)
    Private,
    /// 商业频道 (OKX特有)
    Business,
}

impl WsChannel {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Business => "business",
        }
    }
}

// ============================================================================
// 统一端点配置结构体
// ============================================================================

/// 交易所端点统一配置
///
/// REST和WebSocket端点的单一数据源。
/// 所有端点访问都通过此结构体，确保数据一致性。
///
/// # 示例
///
/// ```rust,ignore
/// static ENDPOINTS: Lazy<ExchangeEndpoints> = Lazy::new(|| {
///     ExchangeEndpoints {
///         rest: RestEndpoints {
///             spot: "https://api.example.com/v3",
///             linear_swap: Some("https://fapi.example.com/v1"),
///             inverse_swap: Some("https://dapi.example.com/v1"),
///             option: None,
///             special: HashMap::new(),
///         },
///         websocket: WsEndpoints {
///             public: "wss://stream.example.com/ws",
///             private: "wss://stream.example.com/ws/private",
///             by_market: None,
///             by_channel: None,
///         },
///     }
/// });
/// ```
#[derive(Debug, Clone)]
pub struct ExchangeEndpoints {
    /// REST端点配置
    pub rest: RestEndpoints,
    /// WebSocket端点配置
    pub websocket: WsEndpoints,
}

/// REST端点集合
///
/// 包含交易所所有REST API端点。
/// 使用 `&'static str` 实现零拷贝，URL在编译期已知。
#[derive(Debug, Clone)]
pub struct RestEndpoints {
    /// 现货REST端点
    pub spot: &'static str,
    /// 线性合约REST端点 (USDT本位)
    pub linear_swap: Option<&'static str>,
    /// 反向合约REST端点 (币本位)
    pub inverse_swap: Option<&'static str>,
    /// 期权REST端点
    pub option: Option<&'static str>,
    /// 特殊端点 (如Binance SAPI, PAPI等)
    pub special: HashMap<&'static str, &'static str>,
}

/// WebSocket端点集合
///
/// 包含交易所所有WebSocket端点。
/// 支持按市场类型或频道类型区分。
#[derive(Debug, Clone)]
pub struct WsEndpoints {
    /// 公共WS端点 (默认)
    pub public: &'static str,
    /// 私有WS端点
    pub private: &'static str,
    /// 按市场类型区分的WS端点 (如Binance)
    ///
    /// 例如:
    /// - Spot → wss://stream.example.com/ws
    /// - Swap → wss://fstream.example.com/ws
    pub by_market: Option<HashMap<crate::ws::subscription::MarketType, &'static str>>,
    /// 按频道类型区分的WS端点 (如OKX)
    ///
    /// 例如:
    /// - Public → wss://ws.example.com/v5/public
    /// - Private → wss://ws.example.com/v5/private
    /// - Business → wss://ws.example.com/v5/business
    pub by_channel: Option<HashMap<WsChannel, &'static str>>,
}

// ============================================================================
// 统一端点管理器 Trait
// ============================================================================

/// 统一端点管理器 Trait
///
/// 提供REST和WebSocket端点的统一访问接口。
/// 实现此Trait后，会自动获得 `WsEndpointProvider` 能力。
///
/// # 核心方法
///
/// - `endpoints()`: 获取端点配置 (唯一数据源)
/// - `rest_endpoint_for_market()`: 根据市场类型获取REST端点
/// - `ws_endpoint_for_context()`: 根据上下文获取WS端点
///
/// # 自动实现
///
/// 实现此Trait后，编译器会自动实现 `WsEndpointProvider`，
/// 无需手动编写桥接代码。
///
/// # 示例
///
/// ```rust,ignore
/// impl ExchangeEndpointManager for Binance {
///     fn endpoints(&self) -> &ExchangeEndpoints {
///         if self.is_sandbox() {
///             &TESTNET_ENDPOINTS
///         } else {
///             &PRODUCTION_ENDPOINTS
///         }
///     }
/// }
///
/// // WsEndpointProvider 自动实现！
/// ```
pub trait ExchangeEndpointManager: Send + Sync {
    /// 获取端点配置 (核心方法)
    ///
    /// 这是唯一的数据源，所有端点访问都通过此方法。
    /// 通常返回静态引用，使用 `Lazy<ExchangeEndpoints>` 初始化。
    fn endpoints(&self) -> &ExchangeEndpoints;

    // ========================================================================
    // REST端点访问方法
    // ========================================================================

    /// 根据市场类型获取REST端点
    ///
    /// # 路由逻辑
    ///
    /// | 市场类型 | linear | inverse | 返回端点 |
    /// |---------|--------|---------|---------|
    /// | Spot | - | - | spot |
    /// | Swap/Futures | true | false | linear_swap |
    /// | Swap/Futures | false | true | inverse_swap |
    /// | Option | - | - | option |
    ///
    /// # 参数
    ///
    /// - `market`: 市场对象，包含市场类型信息
    ///
    /// # 返回
    ///
    /// REST API端点URL
    fn rest_endpoint_for_market(&self, market: &Market) -> &'static str {
        let endpoints = self.endpoints();
        match market.market_type {
            crate::types::market::MarketType::Spot => endpoints.rest.spot,
            crate::types::market::MarketType::Swap | crate::types::market::MarketType::Futures => {
                let is_linear = market.linear.unwrap_or(true);
                if is_linear {
                    endpoints.rest.linear_swap.unwrap_or(endpoints.rest.spot)
                } else {
                    endpoints.rest.inverse_swap.unwrap_or(endpoints.rest.spot)
                }
            }
            crate::types::market::MarketType::Option => {
                endpoints.rest.option.unwrap_or(endpoints.rest.spot)
            }
        }
    }

    /// 获取默认REST端点
    ///
    /// 默认返回spot端点。
    /// 交易所可以override此方法提供更智能的默认值。
    fn default_rest_endpoint(&self) -> &'static str {
        self.endpoints().rest.spot
    }

    /// 获取所有REST端点
    ///
    /// 返回包含所有REST端点的HashMap。
    /// 用于API文档、调试等场景。
    fn all_rest_endpoints(&self) -> HashMap<&'static str, &'static str> {
        let endpoints = self.endpoints();
        let mut map = HashMap::new();

        map.insert("spot", endpoints.rest.spot);
        if let Some(url) = endpoints.rest.linear_swap {
            map.insert("linear_swap", url);
        }
        if let Some(url) = endpoints.rest.inverse_swap {
            map.insert("inverse_swap", url);
        }
        if let Some(url) = endpoints.rest.option {
            map.insert("option", url);
        }

        // 合并特殊端点
        for (key, value) in &endpoints.rest.special {
            map.insert(key, value);
        }

        map
    }

    // ========================================================================
    // WebSocket端点访问方法
    // ========================================================================

    /// 根据上下文获取WebSocket端点
    ///
    /// # 选择逻辑
    ///
    /// 1. 优先按市场类型选择 (by_market)
    /// 2. 其次按频道类型选择 (by_channel)
    /// 3. 默认返回公共端点 (public)
    ///
    /// # 参数
    ///
    /// - `context`: WebSocket上下文，包含市场类型等信息
    ///
    /// # 返回
    ///
    /// WebSocket URL
    fn ws_endpoint_for_context(&self, context: &WsContext) -> String {
        let endpoints = self.endpoints();

        // 优先按市场类型
        if let Some(by_market) = &endpoints.websocket.by_market {
            if let Some(market_type) = context.market_type {
                if let Some(url) = by_market.get(&market_type) {
                    return url.to_string();
                }
            }
        }

        // 其次按频道类型
        if let Some(by_channel) = &endpoints.websocket.by_channel {
            let channel = if context.is_private {
                WsChannel::Private
            } else {
                WsChannel::Public
            };
            if let Some(url) = by_channel.get(&channel) {
                return url.to_string();
            }
        }

        // 默认返回公共WS
        endpoints.websocket.public.to_string()
    }

    /// 获取公共WebSocket端点
    ///
    /// # 参数
    ///
    /// - `context`: WebSocket上下文
    ///
    /// # 返回
    ///
    /// 公共WebSocket URL
    fn ws_public_endpoint(&self, context: &WsContext) -> String {
        let endpoints = self.endpoints();

        // 支持按市场类型区分
        if let Some(by_market) = &endpoints.websocket.by_market {
            if let Some(market_type) = context.market_type {
                if let Some(url) = by_market.get(&market_type) {
                    return url.to_string();
                }
            }
        }

        endpoints.websocket.public.to_string()
    }

    /// 获取私有WebSocket端点
    ///
    /// # 参数
    ///
    /// - `context`: WebSocket上下文
    ///
    /// # 返回
    ///
    /// 私有WebSocket URL
    fn ws_private_endpoint(&self, _context: &WsContext) -> String {
        self.endpoints().websocket.private.to_string()
    }

    /// 获取所有WebSocket端点
    ///
    /// 返回包含所有WebSocket端点的HashMap。
    /// 用于API文档、调试等场景。
    fn all_ws_endpoints(&self) -> HashMap<&'static str, &'static str> {
        let endpoints = self.endpoints();
        let mut map = HashMap::new();

        map.insert("public", endpoints.websocket.public);
        map.insert("private", endpoints.websocket.private);

        if let Some(by_market) = &endpoints.websocket.by_market {
            for (market_type, url) in by_market {
                let key = match market_type {
                    crate::ws::subscription::MarketType::Spot => "ws_spot",
                    crate::ws::subscription::MarketType::Swap => "ws_swap",
                    crate::ws::subscription::MarketType::Future => "ws_futures",
                    crate::ws::subscription::MarketType::Option => "ws_option",
                };
                map.insert(key, url);
            }
        }

        if let Some(by_channel) = &endpoints.websocket.by_channel {
            for (channel, url) in by_channel {
                let key = match channel {
                    WsChannel::Public => "ws_public",
                    WsChannel::Private => "ws_private",
                    WsChannel::Business => "ws_business",
                };
                map.insert(key, url);
            }
        }

        map
    }
}

// ============================================================================
// WsEndpointProvider 自动实现 (核心创新!)
// ============================================================================

/// 为所有实现 `ExchangeEndpointManager` 的类型自动实现 `WsEndpointProvider`
///
/// 这是REST和WebSocket端点统一的关键！
///
/// # 工作原理
///
/// 通过Rust的默认实现特性，编译器会自动为所有实现 `ExchangeEndpointManager`
/// 的类型生成 `WsEndpointProvider` 的实现，无需手动编写桥接代码。
///
/// # 优势
///
/// 1. **零重复代码**: 交易所只需实现一个Trait
/// 2. **自动桥接**: REST和WS共用同一数据源
/// 3. **零运行时开销**: 编译器会内联这些调用
/// 4. **向后兼容**: GenericWsClient无需修改
///
/// # 示例
///
/// ```rust,ignore
/// // 交易所只需实现 ExchangeEndpointManager
/// impl ExchangeEndpointManager for Binance {
///     fn endpoints(&self) -> &ExchangeEndpoints { ... }
/// }
///
/// // WsEndpointProvider 自动实现！
/// // 可以直接用于 GenericWsClient
/// let client = GenericWsClient::new(
///     builder,
///     parser,
///     binance,  // ← 直接使用交易所实例
///     auth,
/// );
/// ```
impl<T> WsEndpointProvider for T
where
    T: ExchangeEndpointManager + Clone + 'static,
{
    fn ws_public_url(&self, context: &WsContext) -> String {
        // 自动桥接到 ExchangeEndpointManager
        self.ws_public_endpoint(context)
    }

    fn ws_private_url(&self, context: &WsContext) -> String {
        // 自动桥接到 ExchangeEndpointManager
        self.ws_private_endpoint(context)
    }

    fn supports_multiple_urls(&self) -> bool {
        let endpoints = self.endpoints();
        endpoints.websocket.by_market.is_some() || endpoints.websocket.by_channel.is_some()
    }

    fn all_urls(&self) -> Vec<String> {
        self.all_ws_endpoints()
            .values()
            .map(|s| s.to_string())
            .collect()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;
    use crate::prelude::Symbol;
    use crate::types::market::{Market, MarketType};
    use crate::ws::subscription::MarketType as WsMarketType;
    use rust_decimal_macros::dec;

    // 测试用的端点配置
    static TEST_ENDPOINTS: LazyLock<ExchangeEndpoints> = LazyLock::new(|| ExchangeEndpoints {
        rest: RestEndpoints {
            spot: "https://api.test.com/v3",
            linear_swap: Some("https://fapi.test.com/v1"),
            inverse_swap: Some("https://dapi.test.com/v1"),
            option: Some("https://eapi.test.com/v1"),
            special: {
                let mut map = HashMap::new();
                map.insert("sapi", "https://sapi.test.com/v1");
                map
            },
        },
        websocket: WsEndpoints {
            public: "wss://stream.test.com/ws",
            private: "wss://stream.test.com/ws/private",
            by_market: Some({
                let mut map = HashMap::new();
                map.insert(WsMarketType::Spot, "wss://stream.test.com/ws");
                map.insert(WsMarketType::Swap, "wss://fstream.test.com/ws");
                map.insert(WsMarketType::Future, "wss://dstream.test.com/ws");
                map.insert(WsMarketType::Option, "wss://eoptions.test.com/ws");
                map
            }),
            by_channel: None,
        },
    });

    // 测试用的交易所结构体
    #[derive(Clone)]
    struct TestExchange;

    impl ExchangeEndpointManager for TestExchange {
        fn endpoints(&self) -> &ExchangeEndpoints {
            &TEST_ENDPOINTS
        }
    }

    #[test]
    fn test_rest_endpoint_for_spot() {
        let exchange = TestExchange;
        let market = Market::new_spot(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT"),
            "BTC".to_string(),
            "USDT".to_string(),
        );

        let url = exchange.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://api.test.com/v3");
    }

    #[test]
    fn test_rest_endpoint_for_linear_swap() {
        let exchange = TestExchange;
        let market = Market::new_swap(
            "BTCUSDT".to_string(),
            Symbol::new_unchecked("BTC/USDT:USDT"),
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            dec!(1.0),
        );

        let url = exchange.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://fapi.test.com/v1");
    }

    #[test]
    fn test_rest_endpoint_for_inverse_swap() {
        let exchange = TestExchange;
        let mut market = Market::new_swap(
            "BTCUSD".to_string(),
            Symbol::new_unchecked("BTC/USD:BTC"),
            "BTC".to_string(),
            "USD".to_string(),
            "BTC".to_string(),
            dec!(100.0),
        );
        market.linear = Some(false);
        market.inverse = Some(true);

        let url = exchange.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://dapi.test.com/v1");
    }

    #[test]
    fn test_rest_endpoint_for_option() {
        let exchange = TestExchange;
        let market = Market {
            id: "BTC-OPTION".to_string(),
            symbol: Symbol::new_unchecked("BTC-OPTION"),
            market_type: MarketType::Option,
            base: "BTC".to_string(),
            quote: "USDT".to_string(),
            ..Default::default()
        };

        let url = exchange.rest_endpoint_for_market(&market);
        assert_eq!(url, "https://eapi.test.com/v1");
    }

    #[test]
    fn test_ws_endpoint_public() {
        let exchange = TestExchange;
        let context = WsContext::new();

        let url = exchange.ws_public_endpoint(&context);
        assert_eq!(url, "wss://stream.test.com/ws");
    }

    #[test]
    fn test_ws_endpoint_by_market_type() {
        let exchange = TestExchange;
        let mut context = WsContext::new();
        context.market_type = Some(WsMarketType::Swap);

        let url = exchange.ws_public_endpoint(&context);
        assert_eq!(url, "wss://fstream.test.com/ws");
    }

    #[test]
    fn test_all_rest_endpoints() {
        let exchange = TestExchange;
        let endpoints = exchange.all_rest_endpoints();

        assert_eq!(endpoints.get("spot"), Some(&"https://api.test.com/v3"));
        assert_eq!(
            endpoints.get("linear_swap"),
            Some(&"https://fapi.test.com/v1")
        );
        assert_eq!(
            endpoints.get("inverse_swap"),
            Some(&"https://dapi.test.com/v1")
        );
        assert_eq!(endpoints.get("sapi"), Some(&"https://sapi.test.com/v1"));
    }

    #[test]
    fn test_all_ws_endpoints() {
        let exchange = TestExchange;
        let endpoints = exchange.all_ws_endpoints();

        assert_eq!(endpoints.get("public"), Some(&"wss://stream.test.com/ws"));
        assert_eq!(
            endpoints.get("private"),
            Some(&"wss://stream.test.com/ws/private")
        );
        assert_eq!(endpoints.get("ws_spot"), Some(&"wss://stream.test.com/ws"));
        assert_eq!(endpoints.get("ws_swap"), Some(&"wss://fstream.test.com/ws"));
    }

    #[test]
    fn test_ws_endpoint_provider_auto_impl() {
        let exchange = TestExchange;
        let context = WsContext::new();

        // 验证 WsEndpointProvider 自动实现
        let public_url = exchange.ws_public_url(&context);
        assert_eq!(public_url, "wss://stream.test.com/ws");

        let private_url = exchange.ws_private_url(&context);
        assert_eq!(private_url, "wss://stream.test.com/ws/private");

        // 验证 supports_multiple_urls
        assert!(exchange.supports_multiple_urls());

        // 验证 all_urls
        let urls = exchange.all_urls();
        assert!(!urls.is_empty());
        assert!(urls.contains(&"wss://stream.test.com/ws".to_string()));
    }

    #[test]
    fn test_default_rest_endpoint() {
        let exchange = TestExchange;
        let url = exchange.default_rest_endpoint();
        assert_eq!(url, "https://api.test.com/v3");
    }
}

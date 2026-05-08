//! StreamParser Trait 定义
//!
//! 用于解析交易所特定的消息格式
//!
//! # 迁移指南
//!
//! 本模块正在从"多方法"设计迁移到"泛型"设计：
//!
//! ## 旧设计（已标记为 deprecated）
//! ```rust,ignore
//! pub trait StreamParser {
//!     fn parse_ticker(&self, msg: &Value) -> Result<Ticker>;
//!     fn parse_orderbook(&self, msg: &Value) -> Result<OrderBook>;
//!     // ... 每新增一个类型就要加一个方法
//! }
//! ```
//!
//! ## 新设计（推荐）
//! ```rust,ignore
//! pub trait StreamParserV2: StreamParser {
//!     fn parse_as<T: Parseable>(&self, msg: &Value) -> Result<T>;
//! }
//!
//! // 使用
//! let ticker = parser.parse_as::<Ticker>(msg)?;
//! let orderbook = parser.parse_as::<OrderBook>(msg)?;
//! ```
//!
//! ## 迁移步骤
//! 1. 为你的类型实现 `Parseable` trait
//! 2. 在 `StreamParser` 实现中添加 `parse_as` 方法
//! 3. 测试新接口
//! 4. 逐步替换旧方法调用

use crate::error::Result;
use crate::types::{Balance, BidAsk, MarkPrice, Ohlcv, Order, OrderBook, Position, Ticker, Trade};
use serde_json::Value;

/// 交易所特定的扩展消息
///
/// 用于封装交易所私有/特殊类型的消息数据。
/// 调用方可根据 `exchange_id` 和 `channel` 进行特定处理。
#[derive(Debug, Clone)]
pub struct ExchangeMessage {
    /// 交易所标识 (e.g., "okx", "bybit", "binance")
    pub exchange_id: &'static str,
    /// 消息频道名称
    pub channel: String,
    /// 原始 JSON 数据
    pub data: Value,
}

impl ExchangeMessage {
    /// 创建新的交易所特定消息
    pub fn new(exchange_id: &'static str, channel: impl Into<String>, data: Value) -> Self {
        Self {
            exchange_id,
            channel: channel.into(),
            data,
        }
    }

    /// 尝试将 data 解析为指定类型
    ///
    /// # Example
    /// ```ignore
    /// use ccxt_core::types::Trade;
    /// if let Some(trade) = msg.parse_as::<Trade>() {
    ///     println!("Trade: {:?}", trade);
    /// }
    /// ```
    pub fn parse_as<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_value(self.data.clone()).ok()
    }
}

/// Parsed message types
///
/// Unified message types covering standard data from all exchanges.
/// For exchange-specific private messages, use the `ExchangeSpecific` variant.
#[derive(Debug, Clone)]
pub enum ParsedMessage {
    /// Ticker market data
    Ticker(Ticker),
    /// Multiple tickers
    Tickers(Vec<Ticker>),
    /// Order book
    OrderBook(OrderBook),
    /// Trade records
    Trades(Vec<Trade>),
    /// K-line data (can be multiple)
    Ohlcv(Vec<Ohlcv>),
    /// Account balance
    Balance(Balance),
    /// Order update
    Order(Order),
    /// Mark price
    MarkPrice(MarkPrice),
    /// Best bid/ask
    BidsAsks(BidAsk),
    /// Position update
    Positions(Vec<Position>),
    /// Heartbeat/keep-alive
    Heartbeat,
    /// Subscription confirmation
    SubscriptionConfirm {
        /// Subscribed channel
        channel: String,
    },
    /// Authentication success
    AuthSuccess,
    /// Error
    Error {
        /// Error code
        code: Option<i32>,
        /// Error message
        message: String,
    },
    /// Exchange-specific message
    ///
    /// 用于封装交易所私有/特殊类型的消息，如：
    /// - OKX: 账户配置变更、风险状态
    /// - Binance: 用户数据流事件
    /// - Bybit: 钱包余额变动
    ///
    /// 调用方可根据 `exchange_id` 和 `channel` 进行类型安全的解析。
    ExchangeSpecific(ExchangeMessage),
    /// 未知消息（原始 JSON）
    ///
    /// 当消息无法识别时使用，保留原始数据供调试或手动处理。
    Unknown(Value),
}

impl ParsedMessage {
    /// 检查是否为心跳消息
    pub fn is_heartbeat(&self) -> bool {
        matches!(self, Self::Heartbeat)
    }

    /// 检查是否为错误消息
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// 检查是否为数据消息
    pub fn is_data(&self) -> bool {
        matches!(
            self,
            Self::Ticker(_)
                | Self::Tickers(_)
                | Self::OrderBook(_)
                | Self::Trades(_)
                | Self::Ohlcv(_)
                | Self::Balance(_)
                | Self::Order(_)
                | Self::MarkPrice(_)
                | Self::BidsAsks(_)
                | Self::Positions(_)
                | Self::ExchangeSpecific(_)
        )
    }

    /// 检查是否为交易所特定消息
    pub fn is_exchange_specific(&self) -> bool {
        matches!(self, Self::ExchangeSpecific(_))
    }

    /// 检查是否为指定交易所的特定消息
    pub fn is_exchange(&self, exchange_id: &str) -> bool {
        match self {
            Self::ExchangeSpecific(msg) => msg.exchange_id == exchange_id,
            _ => false,
        }
    }

    /// 尝试获取 Ticker
    pub fn as_ticker(&self) -> Option<&Ticker> {
        match self {
            Self::Ticker(t) => Some(t),
            _ => None,
        }
    }

    /// 尝试获取订单簿
    pub fn as_orderbook(&self) -> Option<&OrderBook> {
        match self {
            Self::OrderBook(ob) => Some(ob),
            _ => None,
        }
    }

    /// 尝试获取成交记录
    pub fn as_trades(&self) -> Option<&[Trade]> {
        match self {
            Self::Trades(t) => Some(t),
            _ => None,
        }
    }

    /// 尝试获取 K线
    pub fn as_ohlcv(&self) -> Option<&[Ohlcv]> {
        match self {
            Self::Ohlcv(o) => Some(o),
            _ => None,
        }
    }

    /// 尝试获取余额
    pub fn as_balance(&self) -> Option<&Balance> {
        match self {
            Self::Balance(b) => Some(b),
            _ => None,
        }
    }

    /// 尝试获取订单
    pub fn as_order(&self) -> Option<&Order> {
        match self {
            Self::Order(o) => Some(o),
            _ => None,
        }
    }

    /// 尝试获取交易所特定消息
    pub fn as_exchange_specific(&self) -> Option<&ExchangeMessage> {
        match self {
            Self::ExchangeSpecific(msg) => Some(msg),
            _ => None,
        }
    }

    /// 尝试获取原始 JSON 数据
    ///
    /// 对于 `Unknown` 和 `ExchangeSpecific` 变体，返回其原始数据。
    pub fn as_raw(&self) -> Option<&Value> {
        match self {
            Self::Unknown(v) => Some(v),
            Self::ExchangeSpecific(msg) => Some(&msg.data),
            _ => None,
        }
    }
}

/// 流解析器 Trait
///
/// 每个交易所实现自己的消息解析逻辑。
///
/// # 实现示例
///
/// ```rust,ignore
/// use ccxt_core::ws::{StreamParser, ParsedMessage};
///
/// pub struct OkxStreamParser;
///
/// impl StreamParser for OkxStreamParser {
///     fn parse(&self, msg: &Value) -> Result<ParsedMessage> {
///         // 解析 OKX 消息格式
///         let arg = msg.get("arg").ok_or(...)?;
///         let channel = arg.get("channel").and_then(|v| v.as_str());
///
///         match channel {
///             Some("tickers") => {
///                 let ticker = self.parse_ticker(msg)?;
///                 Ok(ParsedMessage::Ticker(ticker))
///             }
///             // ...
///         }
///     }
/// }
/// ```
pub trait StreamParser: Clone + Send + Sync + 'static {
    /// 解析消息
    ///
    /// 将原始 JSON 消息解析为统一的消息类型
    fn parse(&self, msg: &Value) -> Result<ParsedMessage>;
}

// ============================================================================
// 新设计：泛型解析（渐进式迁移）
// ============================================================================

/// 可解析类型标记 trait
///
/// 用于标识可以通过 StreamParser 解析的类型。
/// 实现此 trait 的类型可以使用 `StreamParserExt::parse_as` 方法解析。
///
/// # 示例
/// ```rust,ignore
/// // 为 Ticker 实现 Parseable
/// impl Parseable for Ticker {
///     const TYPE_NAME: &'static str = "ticker";
/// }
///
/// // 使用
/// let ticker = parser.parse_as::<Ticker>(msg)?;
/// ```
pub trait Parseable: Sized + Send + 'static {
    /// 类型名称（用于错误信息和调试）
    const TYPE_NAME: &'static str;
}

// 为现有类型实现 Parseable
impl Parseable for Ticker {
    const TYPE_NAME: &'static str = "ticker";
}

impl Parseable for OrderBook {
    const TYPE_NAME: &'static str = "orderbook";
}

impl Parseable for Trade {
    const TYPE_NAME: &'static str = "trade";
}

impl Parseable for Ohlcv {
    const TYPE_NAME: &'static str = "ohlcv";
}

impl Parseable for Balance {
    const TYPE_NAME: &'static str = "balance";
}

impl Parseable for Order {
    const TYPE_NAME: &'static str = "order";
}

impl Parseable for MarkPrice {
    const TYPE_NAME: &'static str = "mark_price";
}

impl Parseable for BidAsk {
    const TYPE_NAME: &'static str = "bid_ask";
}

impl Parseable for Position {
    const TYPE_NAME: &'static str = "position";
}

// 为 Vec<T> 实现 Parseable（用于 trades, ohlcv, positions）
impl<T: Parseable> Parseable for Vec<T> {
    const TYPE_NAME: &'static str = T::TYPE_NAME;
}

/// StreamParser 扩展 trait - 泛型解析方法
///
/// 这是新的推荐接口，使用泛型方法替代多个具体方法。
/// 实现者可以选择实现此 trait 来获得更灵活的解析能力。
///
/// # 设计优势
///
/// 1. **开闭原则**: 新增类型不需要修改 trait
/// 2. **类型安全**: 编译时检查解析类型
/// 3. **代码复用**: 避免为每个类型写重复代码
///
/// # 实现示例
///
/// ```rust,ignore
/// use ccxt_core::ws::{StreamParser, StreamParserExt, Parseable};
///
/// pub struct MyParser;
///
/// impl StreamParser for MyParser {
///     fn parse(&self, msg: &Value) -> Result<WsMessage> {
///         // 路由逻辑
///     }
///
///     fn parse_ticker(&self, msg: &Value) -> Result<Ticker> {
///         // 旧方法实现
///     }
///     // ... 其他旧方法
/// }
///
/// impl StreamParserExt for MyParser {
///     fn parse_as<T: Parseable>(&self, msg: &Value) -> Result<T> {
///         // 使用类型名称分发到具体解析方法
///         match T::TYPE_NAME {
///             "ticker" => self.parse_ticker(msg).map(|t| {
///                 // 安全转换：我们知道 T 是 Ticker
///                 unsafe { std::mem::transmute_copy(&t) }
///             }),
///             // ... 其他类型
///             _ => Err(Error::invalid_request(format!(
///                 "Unsupported type: {}", T::TYPE_NAME
///             ))),
///         }
///     }
/// }
/// ```
pub trait StreamParserExt: StreamParser {
    /// 泛型解析方法
    ///
    /// 根据类型参数 T 解析消息为对应类型。
    ///
    /// # Type Parameters
    /// * `T` - 目标类型，必须实现 `Parseable`
    ///
    /// # Arguments
    /// * `msg` - JSON 消息
    ///
    /// # Returns
    /// 解析后的类型 T
    ///
    /// # Example
    /// ```rust,ignore
    /// let ticker = parser.parse_as::<Ticker>(msg)?;
    /// let orderbook = parser.parse_as::<OrderBook>(msg)?;
    /// ```
    fn parse_as<T: Parseable>(&self, msg: &Value) -> Result<T>;
}

/// 解析结果类型
///
/// 用于表示解析操作的结果，支持成功、不支持、错误三种状态。
#[derive(Debug)]
pub enum ParseResult<T> {
    /// 解析成功
    Parsed(T),
    /// 此解析器不支持该类型
    NotSupported,
    /// 解析失败
    Error(crate::error::Error),
}

impl<T> ParseResult<T> {
    /// 转换为 Result，将 NotSupported 视为错误
    pub fn into_result(self) -> Result<T> {
        match self {
            ParseResult::Parsed(t) => Ok(t),
            ParseResult::NotSupported => Err(crate::error::Error::invalid_request(
                "Parser does not support this type",
            )),
            ParseResult::Error(e) => Err(e),
        }
    }

    /// 检查是否解析成功
    pub fn is_parsed(&self) -> bool {
        matches!(self, ParseResult::Parsed(_))
    }

    /// 检查是否不支持
    pub fn is_not_supported(&self) -> bool {
        matches!(self, ParseResult::NotSupported)
    }
}

// ============================================================================
// 辅助宏：简化 Parseable 实现
// ============================================================================

/// 为类型自动生成 Parseable 实现的宏
///
/// # 示例
/// ```rust,ignore
/// define_parseable!(MyCustomType, "custom_type");
/// ```
#[macro_export]
macro_rules! define_parseable {
    ($type:ty, $name:expr) => {
        impl $crate::ws::parser::Parseable for $type {
            const TYPE_NAME: &'static str = $name;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_is_heartbeat() {
        let msg = ParsedMessage::Heartbeat;
        assert!(msg.is_heartbeat());
        assert!(!msg.is_error());
        assert!(!msg.is_data());
    }

    #[test]
    fn test_ws_message_is_error() {
        let msg = ParsedMessage::Error {
            code: Some(400),
            message: "Bad request".to_string(),
        };
        assert!(msg.is_error());
        assert!(!msg.is_heartbeat());
        assert!(!msg.is_data());
    }

    #[test]
    fn test_ws_message_is_data() {
        let ticker = Ticker::default();
        let msg = ParsedMessage::Ticker(ticker);
        assert!(msg.is_data());
        assert!(!msg.is_heartbeat());
        assert!(!msg.is_error());
    }

    #[test]
    fn test_exchange_message_new() {
        let data = serde_json::json!({"key": "value"});
        let msg = ExchangeMessage::new("okx", "account-config", data.clone());

        assert_eq!(msg.exchange_id, "okx");
        assert_eq!(msg.channel, "account-config");
        assert_eq!(msg.data, data);
    }

    #[test]
    fn test_exchange_message_parse_as() {
        #[derive(Debug, serde::Deserialize)]
        struct TestData {
            key: String,
        }

        let data = serde_json::json!({"key": "value"});
        let msg = ExchangeMessage::new("okx", "test", data);

        let parsed: Option<TestData> = msg.parse_as();
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().key, "value");
    }

    #[test]
    fn test_ws_message_exchange_specific() {
        let data = serde_json::json!({"event": "accountConfig"});
        let exchange_msg = ExchangeMessage::new("okx", "account-config", data.clone());
        let msg = ParsedMessage::ExchangeSpecific(exchange_msg);

        assert!(msg.is_exchange_specific());
        assert!(msg.is_exchange("okx"));
        assert!(!msg.is_exchange("bybit"));
        assert!(msg.is_data());

        let specific = msg.as_exchange_specific();
        assert!(specific.is_some());
        let specific = specific.unwrap();
        assert_eq!(specific.exchange_id, "okx");
        assert_eq!(specific.channel, "account-config");
    }

    #[test]
    fn test_ws_message_as_raw() {
        // Unknown variant
        let data = serde_json::json!({"unknown": true});
        let msg = ParsedMessage::Unknown(data.clone());
        assert_eq!(msg.as_raw(), Some(&data));

        // ExchangeSpecific variant
        let exchange_msg = ExchangeMessage::new("okx", "test", data.clone());
        let msg = ParsedMessage::ExchangeSpecific(exchange_msg);
        assert_eq!(msg.as_raw(), Some(&data));

        // Ticker variant - no raw data
        let msg = ParsedMessage::Ticker(Ticker::default());
        assert!(msg.as_raw().is_none());
    }
}

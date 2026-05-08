//! 订阅相关类型定义

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 频道类型枚举
///
/// 定义所有交易所通用的频道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Ticker 行情
    Ticker,
    /// 多个 Ticker 行情
    Tickers,
    /// 订单簿
    OrderBook,
    /// 成交记录
    Trades,
    /// K线数据
    Kline,
    /// 账户余额（私有）
    Balance,
    /// 订单更新（私有）
    Orders,
    /// 用户成交（私有）
    MyTrades,
    /// 用户事件（私有）
    UserEvents,
    /// 标记价格
    MarkPrice,
    /// 最优买卖价
    BidsAsks,
    /// 持仓更新（私有）
    Positions,
    /// 自定义频道
    Custom,
}

impl Default for ChannelType {
    fn default() -> Self {
        Self::Ticker
    }
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ticker => write!(f, "ticker"),
            Self::Tickers => write!(f, "tickers"),
            Self::OrderBook => write!(f, "orderbook"),
            Self::Trades => write!(f, "trades"),
            Self::Kline => write!(f, "kline"),
            Self::Balance => write!(f, "balance"),
            Self::Orders => write!(f, "orders"),
            Self::MyTrades => write!(f, "account_trades"),
            Self::UserEvents => write!(f, "user_events"),
            Self::MarkPrice => write!(f, "mark_price"),
            Self::BidsAsks => write!(f, "bids_asks"),
            Self::Positions => write!(f, "positions"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// 市场类型枚举
///
/// 用于区分不同的市场类型（现货、合约等）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketType {
    /// 现货
    Spot,
    /// 永续合约（USDT/USDC 保证金）
    Swap,
    /// 交割合约
    Future,
    /// 期权
    Option,
}

impl Default for MarketType {
    fn default() -> Self {
        Self::Spot
    }
}

impl std::fmt::Display for MarketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spot => write!(f, "spot"),
            Self::Swap => write!(f, "swap"),
            Self::Future => write!(f, "future"),
            Self::Option => write!(f, "option"),
        }
    }
}

/// 订阅频道定义
///
/// 通用的订阅频道结构，可转换为交易所特定格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionChannel {
    /// 频道类型
    pub channel_type: ChannelType,
    /// 交易对符号（如 "BTC/USDT"）
    pub symbol: String,
    /// 额外参数（如 K线周期、订单簿深度等）
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, Value>,
    /// 市场类型（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_type: Option<MarketType>,
    /// 是否为私有频道
    #[serde(default)]
    pub is_private: bool,
}

impl SubscriptionChannel {
    /// 根据 symbol 格式自动推导市场类型
    ///
    /// 统一 Symbol 格式规范 (CCXT Standard)：
    ///
    /// | 市场         | 格式                        | 示例                      | MarketType |
    /// |-------------|-----------------------------|--------------------------|------------|
    /// | 现货        | `BASE/QUOTE`                | `BTC/USDT`               | Spot       |
    /// | 正向永续    | `BASE/QUOTE:QUOTE`          | `BTC/USDT:USDT`          | Swap       |
    /// | 反向永续    | `BASE/QUOTE:BASE`           | `BTC/USD:BTC`            | Swap       |
    /// | 正向期货    | `BASE/QUOTE:QUOTE-YYMMDD`   | `BTC/USDT:USDT-241231`   | Future     |
    /// | 反向期货    | `BASE/QUOTE:BASE-YYMMDD`    | `BTC/USD:BTC-241231`     | Future     |
    ///
    /// 空字符串返回 None，表示市场类型不适用（如私有频道）。
    pub fn detect_market_type(symbol: &str) -> Option<MarketType> {
        if symbol.is_empty() {
            return None;
        }

        // 不含 ':' → 现货
        if !symbol.contains(':') {
            return Some(MarketType::Spot);
        }

        // 含 ':' → 合约，进一步区分永续/交割
        // 交割合约格式: BASE/QUOTE:{SETTLE}-YYMMDD
        // 检查最后一个 '-' 后是否为 6 位日期数字
        let has_expiry = symbol.contains('-') && {
            symbol
                .rsplit('-')
                .next()
                .map(|last| last.len() == 6 && last.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
        };

        if has_expiry {
            Some(MarketType::Future)
        } else {
            Some(MarketType::Swap)
        }
    }

    /// 创建新的订阅频道
    ///
    /// 自动根据 symbol 格式推导 market_type：
    /// - `BTC/USDT` → Spot
    /// - `BTC/USDT:USDT` → Swap
    /// - `BTC/USDT:USDT-241231` → Future
    /// - 空字符串 → None
    pub fn new(channel_type: ChannelType, symbol: impl Into<String>) -> Self {
        let symbol = symbol.into();
        let market_type = Self::detect_market_type(&symbol);
        Self {
            channel_type,
            symbol,
            params: HashMap::new(),
            market_type,
            is_private: false,
        }
    }

    /// 创建 Ticker 频道
    pub fn ticker(symbol: impl Into<String>) -> Self {
        Self::new(ChannelType::Ticker, symbol)
    }

    /// 创建订单簿频道
    pub fn orderbook(symbol: impl Into<String>) -> Self {
        Self::new(ChannelType::OrderBook, symbol)
    }

    /// 创建成交记录频道
    pub fn trades(symbol: impl Into<String>) -> Self {
        Self::new(ChannelType::Trades, symbol)
    }

    /// 创建 K线频道
    pub fn kline(symbol: impl Into<String>, interval: &str) -> Self {
        let symbol = symbol.into();
        let market_type = Self::detect_market_type(&symbol);
        let mut params = HashMap::new();
        params.insert("interval".to_string(), Value::String(interval.to_string()));
        Self {
            channel_type: ChannelType::Kline,
            symbol,
            params,
            market_type,
            is_private: false,
        }
    }

    /// 创建余额频道（私有）
    pub fn balance() -> Self {
        Self {
            channel_type: ChannelType::Balance,
            symbol: String::new(),
            params: HashMap::new(),
            market_type: None,
            is_private: true,
        }
    }

    /// 创建订单频道（私有）
    pub fn orders(symbol: Option<String>) -> Self {
        Self {
            channel_type: ChannelType::Orders,
            symbol: symbol.unwrap_or_default(),
            params: HashMap::new(),
            market_type: None,
            is_private: true,
        }
    }

    /// 创建用户成交频道（私有）
    pub fn account_trades(symbol: Option<String>) -> Self {
        Self {
            channel_type: ChannelType::MyTrades,
            symbol: symbol.unwrap_or_default(),
            params: HashMap::new(),
            market_type: None,
            is_private: true,
        }
    }

    /// 创建标记价格频道（合约专属）
    pub fn mark_price(symbol: impl Into<String>) -> Self {
        let symbol = symbol.into();
        let market_type = Self::detect_market_type(&symbol);
        Self {
            channel_type: ChannelType::MarkPrice,
            symbol,
            params: HashMap::new(),
            market_type,
            is_private: false,
        }
    }

    /// 创建最优买卖价频道
    pub fn bids_asks(symbol: impl Into<String>) -> Self {
        Self::new(ChannelType::BidsAsks, symbol)
    }

    /// 创建持仓频道（私有，合约专属）
    pub fn positions(symbol: Option<String>) -> Self {
        let symbol = symbol.unwrap_or_default();
        let market_type = Self::detect_market_type(&symbol);
        Self {
            channel_type: ChannelType::Positions,
            symbol,
            params: HashMap::new(),
            market_type,
            is_private: true,
        }
    }

    /// 设置市场类型
    pub fn with_market_type(mut self, market_type: MarketType) -> Self {
        self.market_type = Some(market_type);
        self
    }

    /// 设置为私有频道
    pub fn with_private(mut self) -> Self {
        self.is_private = true;
        self
    }

    /// 添加参数
    pub fn with_param(mut self, key: impl Into<String>, value: Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// 生成频道唯一标识符
    ///
    /// 格式: `channel_type:symbol` 或 `channel_type:symbol:param1=value1`
    pub fn to_key(&self) -> String {
        let base = match self.symbol.is_empty() {
            true => self.channel_type.to_string(),
            false => format!("{}:{}", self.channel_type, self.symbol),
        };

        if self.params.is_empty() {
            base
        } else {
            let params_str = self
                .params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",");
            format!("{}:{}", base, params_str)
        }
    }

    /// 检查是否为私有频道
    pub fn is_private(&self) -> bool {
        self.is_private
            || matches!(
                self.channel_type,
                ChannelType::Balance
                    | ChannelType::Orders
                    | ChannelType::MyTrades
                    | ChannelType::UserEvents
                    | ChannelType::Positions
            )
    }
}

/// 广播策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BroadcastStrategy {
    /// 广播给所有订阅者
    Broadcast,
    /// 只发送给第一个订阅者
    FirstOnly,
    /// 轮询发送
    RoundRobin,
}

impl Default for BroadcastStrategy {
    fn default() -> Self {
        Self::Broadcast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_display() {
        assert_eq!(ChannelType::Ticker.to_string(), "ticker");
        assert_eq!(ChannelType::OrderBook.to_string(), "orderbook");
        assert_eq!(ChannelType::Kline.to_string(), "kline");
    }

    #[test]
    fn test_market_type_display() {
        assert_eq!(MarketType::Spot.to_string(), "spot");
        assert_eq!(MarketType::Swap.to_string(), "swap");
    }

    #[test]
    fn test_subscription_channel_ticker() {
        let ch = SubscriptionChannel::ticker("BTC/USDT");
        assert_eq!(ch.channel_type, ChannelType::Ticker);
        assert_eq!(ch.symbol, "BTC/USDT");
        assert!(!ch.is_private());
        assert_eq!(ch.to_key(), "ticker:BTC/USDT");
    }

    #[test]
    fn test_subscription_channel_kline() {
        let ch = SubscriptionChannel::kline("BTC/USDT", "1m");
        assert_eq!(ch.channel_type, ChannelType::Kline);
        assert_eq!(ch.params.get("interval").unwrap().as_str().unwrap(), "1m");
        assert!(ch.to_key().contains("interval"));
    }

    #[test]
    fn test_subscription_channel_private() {
        let ch = SubscriptionChannel::balance();
        assert_eq!(ch.channel_type, ChannelType::Balance);
        assert!(ch.is_private());
    }

    #[test]
    fn test_subscription_channel_with_market_type() {
        let ch = SubscriptionChannel::ticker("BTC/USDT").with_market_type(MarketType::Swap);
        assert_eq!(ch.market_type, Some(MarketType::Swap));
    }

    // ── market_type 自动推导全覆盖测试 ──

    #[test]
    fn test_market_type_spot() {
        assert_eq!(
            SubscriptionChannel::ticker("BTC/USDT").market_type,
            Some(MarketType::Spot)
        );
        assert_eq!(
            SubscriptionChannel::new(ChannelType::Ticker, "ETH/USDT").market_type,
            Some(MarketType::Spot)
        );
    }

    #[test]
    fn test_market_type_swap_linear() {
        // 正向永续: BASE/QUOTE:QUOTE
        assert_eq!(
            SubscriptionChannel::ticker("BTC/USDT:USDT").market_type,
            Some(MarketType::Swap)
        );
    }

    #[test]
    fn test_market_type_swap_inverse() {
        // 反向永续: BASE/QUOTE:BASE
        assert_eq!(
            SubscriptionChannel::ticker("BTC/USD:BTC").market_type,
            Some(MarketType::Swap)
        );
    }

    #[test]
    fn test_market_type_future_linear() {
        // 正向期货: BASE/QUOTE:QUOTE-YYMMDD
        assert_eq!(
            SubscriptionChannel::ticker("BTC/USDT:USDT-241231").market_type,
            Some(MarketType::Future)
        );
        assert_eq!(
            SubscriptionChannel::ticker("ETH/USDT:USDT-250628").market_type,
            Some(MarketType::Future)
        );
    }

    #[test]
    fn test_market_type_future_inverse() {
        // 反向期货: BASE/QUOTE:BASE-YYMMDD
        assert_eq!(
            SubscriptionChannel::ticker("BTC/USD:BTC-241231").market_type,
            Some(MarketType::Future)
        );
    }

    #[test]
    fn test_market_type_empty() {
        assert_eq!(SubscriptionChannel::balance().market_type, None);
    }

    #[test]
    fn test_market_type_fake_expiry() {
        // 不含 ':' 但有 '-' 的 symbol 不视为期货
        assert_eq!(
            SubscriptionChannel::ticker("BTC-USDT").market_type,
            Some(MarketType::Spot)
        );
        // '-' 后不是 6 位数字也不视为期货
        assert_eq!(
            SubscriptionChannel::ticker("BTC/USDT:USDT-ABC").market_type,
            Some(MarketType::Swap)
        );
    }
}

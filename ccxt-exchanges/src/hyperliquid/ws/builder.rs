//! Hyperliquid 订阅构建器

use ccxt_core::error::Result;
use ccxt_core::ws::{ChannelType, SubscriptionBuilder, SubscriptionChannel};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Hyperliquid WebSocket 订阅构建器
///
/// 将通用订阅请求转换为 Hyperliquid API 格式
#[derive(Debug, Clone, Copy)]
pub struct HyperliquidSubscriptionBuilder;

impl HyperliquidSubscriptionBuilder {
    /// 将统一符号格式 (BTC/USDC:USDC) 转换为 Hyperliquid 格式 (BTC)
    /// Hyperliquid 使用 coin 名称，不是交易对
    fn to_hyperliquid_symbol(symbol: &str) -> String {
        // 提取 base currency (如 BTC/USDC:USDC -> BTC)
        symbol.split('/').next().unwrap_or(symbol).to_string()
    }

    /// 获取 Hyperliquid 频道名称
    fn get_channel_name(channel_type: ChannelType, params: &HashMap<String, Value>) -> String {
        match channel_type {
            ChannelType::Ticker => "allMids".to_string(),
            ChannelType::Tickers => "allMids".to_string(),
            ChannelType::OrderBook => "l2Book".to_string(),
            ChannelType::Trades => "trades".to_string(),
            ChannelType::Kline => "candle".to_string(),
            ChannelType::BidsAsks => "bbo".to_string(),
            ChannelType::UserEvents => "userEvents".to_string(),
            ChannelType::Balance => "userEvents".to_string(),
            ChannelType::Orders => "orderUpdates".to_string(),
            ChannelType::MyTrades => "userFills".to_string(),
            ChannelType::Custom => params
                .get("channel")
                .and_then(|c| c.as_str())
                .unwrap_or("allMids")
                .to_string(),
            _ => "allMids".to_string(),
        }
    }
}

impl SubscriptionBuilder for HyperliquidSubscriptionBuilder {
    /// 构建订阅消息
    ///
    /// # Hyperliquid 订阅格式
    ///
    /// ```json
    /// {
    ///     "method": "subscribe",
    ///     "subscription": {
    ///         "type": "l2Book",
    ///         "coin": "BTC"
    ///     }
    /// }
    /// ```
    fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        // Hyperliquid 每次只能订阅一个频道
        // 如果有多个频道，返回第一个
        if let Some(channel) = channels.first() {
            let channel_name = Self::get_channel_name(channel.channel_type, &channel.params);

            let mut subscription = serde_json::Map::new();
            subscription.insert("type".to_string(), json!(channel_name));

            // 添加 coin 参数（如果需要）
            if !channel.symbol.is_empty()
                && channel.channel_type != ChannelType::Ticker
                && channel.channel_type != ChannelType::Tickers
                && channel.channel_type != ChannelType::Balance
            {
                let coin = Self::to_hyperliquid_symbol(&channel.symbol);
                subscription.insert("coin".to_string(), json!(coin));
            }

            // K线需要 interval 参数
            if channel.channel_type == ChannelType::Kline {
                if let Some(interval) = channel.params.get("interval").and_then(|i| i.as_str()) {
                    subscription.insert("interval".to_string(), json!(interval));
                }
            }

            // BidsAsks (bbo) 需要 coin 参数，已在上面的逻辑中处理

            // 私有频道需要 user 参数
            if matches!(
                channel.channel_type,
                ChannelType::UserEvents | ChannelType::Orders | ChannelType::MyTrades
            ) {
                if let Some(address) = channel.params.get("address").and_then(|a| a.as_str()) {
                    subscription.insert("user".to_string(), json!(address));
                }
            }

            return Ok(json!({
                "method": "subscribe",
                "subscription": subscription
            }));
        }

        Ok(json!({}))
    }

    /// 构建取消订阅消息
    fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        if let Some(channel) = channels.first() {
            let channel_name = Self::get_channel_name(channel.channel_type, &channel.params);

            let mut subscription = serde_json::Map::new();
            subscription.insert("type".to_string(), json!(channel_name));

            if !channel.symbol.is_empty()
                && channel.channel_type != ChannelType::Ticker
                && channel.channel_type != ChannelType::Tickers
                && channel.channel_type != ChannelType::Balance
            {
                let coin = Self::to_hyperliquid_symbol(&channel.symbol);
                subscription.insert("coin".to_string(), json!(coin));
            }

            // K线需要 interval 参数
            if channel.channel_type == ChannelType::Kline {
                if let Some(interval) = channel.params.get("interval").and_then(|i| i.as_str()) {
                    subscription.insert("interval".to_string(), json!(interval));
                }
            }

            // 私有频道需要 user 参数
            if matches!(
                channel.channel_type,
                ChannelType::UserEvents | ChannelType::Orders | ChannelType::MyTrades
            ) {
                if let Some(address) = channel.params.get("address").and_then(|a| a.as_str()) {
                    subscription.insert("user".to_string(), json!(address));
                }
            }

            return Ok(json!({
                "method": "unsubscribe",
                "subscription": subscription
            }));
        }

        Ok(json!({}))
    }

    /// 从消息中提取频道标识
    fn extract_channel(&self, msg: &Value) -> Option<String> {
        let channel = msg.get("channel")?.as_str()?;

        // 对于 K线，频道名包含 coin (如 candle:BTC)
        // 注意: Hyperliquid candle 消息的 data 中使用 "s" 字段表示 coin
        // 因此我们使用 candle:coin 格式，而不是 candle:coin:interval
        if channel == "candle" {
            if let Some(data) = msg.get("data") {
                // candle 消息使用 "s" 字段表示 coin
                if let Some(coin) = data.get("s").and_then(|c| c.as_str()) {
                    return Some(format!("{}:{}", channel, coin));
                }
            }
        }

        // 对于其他频道，返回 channel:coin
        if let Some(data) = msg.get("data") {
            // 尝试从 data.coin 获取（对象格式：l2Book、userFills 等）
            if let Some(coin) = data.get("coin").and_then(|c| c.as_str()) {
                return Some(format!("{}:{}", channel, coin));
            }
            // 如果 data 是数组格式（如 trades 的 WsTrade[] 直接数组），
            // 从第一个元素的 coin 字段提取
            if let Some(arr) = data.as_array() {
                if let Some(first) = arr.first() {
                    if let Some(coin) = first.get("coin").and_then(|c| c.as_str()) {
                        return Some(format!("{}:{}", channel, coin));
                    }
                }
            }
        }

        Some(channel.to_string())
    }

    /// 从订阅参数生成预期的频道标识
    fn extract_channel_from_subscription(
        &self,
        channel_type: &ChannelType,
        symbol: &str,
        params: &HashMap<String, Value>,
    ) -> String {
        let channel_name = Self::get_channel_name(*channel_type, params);

        // allMids (Ticker/Tickers) 订阅不包含 coin 参数，返回所有币种的数据
        // 因此频道 key 只使用 channel_name，不附加 coin 后缀
        if matches!(channel_type, ChannelType::Ticker | ChannelType::Tickers) {
            return channel_name;
        }

        if symbol.is_empty() {
            channel_name
        } else {
            let coin = Self::to_hyperliquid_symbol(symbol);

            // K线、BidsAsks等使用 channel:coin 格式
            format!("{}:{}", channel_name, coin)
        }
    }

    fn rebuild_subscription_channel(
        &self,
        info: &ccxt_core::network::ws_client::subscription::SubscriptionInfo,
    ) -> ccxt_core::error::Result<ccxt_core::ws::subscription::SubscriptionChannel> {
        use ccxt_core::ws::subscription::{ChannelType, SubscriptionChannel};

        // Hyperliquid 格式：info.channel 是 "l2Book", "trades", "ticker" 等
        let channel_type = match info.channel.as_str() {
            "ticker" | "tickers" => ChannelType::Ticker,
            "trades" => ChannelType::Trades,
            "l2Book" | "orderbook" => ChannelType::OrderBook,
            "candle" | "candle1m" => ChannelType::Kline,
            "account" | "balance" => ChannelType::Balance,
            "orders" => ChannelType::Orders,
            _ => {
                tracing::warn!(
                    channel = %info.channel,
                    "Unknown Hyperliquid channel type, defaulting to Ticker"
                );
                ChannelType::Ticker
            }
        };

        Ok(SubscriptionChannel {
            channel_type,
            symbol: info.symbol.clone().unwrap_or_default(),
            params: info.params.clone(),
            market_type: Some(ccxt_core::ws::subscription::MarketType::Swap), // Hyperliquid 主要是合约
            is_private: info.channel == "account"
                || info.channel == "balance"
                || info.channel == "orders",
        })
    }

    /// 构建心跳消息
    ///
    /// Hyperliquid 使用 ping 方法
    fn build_ping(&self) -> Option<Value> {
        Some(json!({"method": "ping"}))
    }

    /// 构建心跳响应
    fn build_pong(&self, _msg: &Value) -> Option<Value> {
        None // Hyperliquid 服务端会自动响应 pong
    }

    /// 检查是否为心跳消息
    fn is_ping(&self, msg: &Value) -> bool {
        msg.get("method")
            .and_then(|m| m.as_str())
            .map(|m| m == "ping")
            .unwrap_or(false)
    }

    /// 检查是否为订阅确认
    fn is_subscription_confirm(&self, msg: &Value) -> bool {
        msg.get("method")
            .and_then(|m| m.as_str())
            .map(|m| m == "subscribe" || m == "unsubscribe")
            .unwrap_or(false)
            && msg.get("channel").is_some()
    }

    /// 检查是否为错误消息
    fn is_error(&self, msg: &Value) -> bool {
        // Hyperliquid 错误格式: {"channel": "error", "data": ...}
        msg.get("channel")
            .and_then(|c| c.as_str())
            .map(|c| c == "error")
            .unwrap_or(false)
    }

    /// 提取错误信息
    fn extract_error(&self, msg: &Value) -> Option<String> {
        let data = msg.get("data")?;
        let message = data
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        Some(message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_to_hyperliquid_symbol() {
        assert_eq!(
            HyperliquidSubscriptionBuilder::to_hyperliquid_symbol("BTC/USDT"),
            "BTC"
        );
        assert_eq!(
            HyperliquidSubscriptionBuilder::to_hyperliquid_symbol("ETH/USDC"),
            "ETH"
        );
    }

    #[test]
    fn test_build_subscribe_orderbook() {
        let builder = HyperliquidSubscriptionBuilder;
        let channel = SubscriptionChannel::orderbook("BTC/USDT");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["method"], "subscribe");
        assert_eq!(msg["subscription"]["type"], "l2Book");
        assert_eq!(msg["subscription"]["coin"], "BTC");
    }

    #[test]
    fn test_build_subscribe_trades() {
        let builder = HyperliquidSubscriptionBuilder;
        let channel = SubscriptionChannel::trades("ETH/USDT");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["subscription"]["type"], "trades");
        assert_eq!(msg["subscription"]["coin"], "ETH");
    }

    #[test]
    fn test_build_subscribe_all_mids() {
        let builder = HyperliquidSubscriptionBuilder;
        let channel = SubscriptionChannel::ticker(""); // allMids 不需要 symbol

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["subscription"]["type"], "allMids");
    }

    #[test]
    fn test_build_ping() {
        let builder = HyperliquidSubscriptionBuilder;

        let msg = builder.build_ping().unwrap();
        assert_eq!(msg["method"], "ping");
    }

    #[test]
    fn test_is_error() {
        let builder = HyperliquidSubscriptionBuilder;

        let msg = json!({
            "channel": "error",
            "data": {"message": "Invalid subscription"}
        });

        assert!(builder.is_error(&msg));
    }

    #[test]
    fn test_extract_channel_ticker_returns_allmids() {
        let builder = HyperliquidSubscriptionBuilder;

        // Ticker with symbol should return just "allMids" (no coin suffix)
        let result = builder.extract_channel_from_subscription(
            &ChannelType::Ticker,
            "ETH/USDC:USDC",
            &HashMap::new(),
        );
        assert_eq!(result, "allMids");

        // Tickers should also return just "allMids"
        let result = builder.extract_channel_from_subscription(
            &ChannelType::Tickers,
            "BTC/USDC:USDC",
            &HashMap::new(),
        );
        assert_eq!(result, "allMids");
    }

    #[test]
    fn test_extract_channel_trades_returns_symbol() {
        let builder = HyperliquidSubscriptionBuilder;

        // Trades with symbol should return "trades:COIN"
        let result = builder.extract_channel_from_subscription(
            &ChannelType::Trades,
            "ETH/USDC:USDC",
            &HashMap::new(),
        );
        assert_eq!(result, "trades:ETH");
    }

    #[test]
    fn test_extract_channel_trades_direct_array() {
        let builder = HyperliquidSubscriptionBuilder;

        // Actual Hyperliquid trades message: data is direct WsTrade[] array
        let msg = json!({
            "channel": "trades",
            "data": [
                {
                    "coin": "ETH",
                    "side": "A",
                    "px": "2000.5",
                    "sz": "1.5",
                    "time": 1700000000000i64
                }
            ]
        });

        let result = builder.extract_channel(&msg);
        assert_eq!(result, Some("trades:ETH".to_string()));
    }

    #[test]
    fn test_extract_channel_orderbook_object() {
        let builder = HyperliquidSubscriptionBuilder;

        // l2Book message: data is object with coin field
        let msg = json!({
            "channel": "l2Book",
            "data": {
                "coin": "BTC",
                "time": 1700000000000i64,
                "levels": [[], []]
            }
        });

        let result = builder.extract_channel(&msg);
        assert_eq!(result, Some("l2Book:BTC".to_string()));
    }
}

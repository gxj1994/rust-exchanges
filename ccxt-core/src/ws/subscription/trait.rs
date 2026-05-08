//! SubscriptionBuilder Trait 定义
//!
//! 用于构建交易所特定的订阅消息格式

use crate::error::Result;
use crate::network::ws_client::subscription::SubscriptionInfo;
use serde_json::Value;

use super::types::{MarketType, SubscriptionChannel};

/// 订阅构建器 Trait
///
/// 每个交易所实现自己的订阅消息格式。
///
/// # 实现示例
///
/// ```rust,ignore
/// use ccxt_core::ws::{SubscriptionBuilder, SubscriptionChannel};
///
/// pub struct OkxSubscriptionBuilder;
///
/// impl SubscriptionBuilder for OkxSubscriptionBuilder {
///     fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
///         let args: Vec<Value> = channels.iter().map(|ch| {
///             json!({
///                 "channel": "tickers",
///                 "instId": &ch.symbol
///             })
///         }).collect();
///
///         Ok(json!({ "op": "subscribe", "args": args }))
///     }
///
///     fn extract_channel(&self, msg: &Value) -> Option<String> {
///         let arg = msg.get("arg")?;
///         let channel = arg.get("channel")?.as_str()?;
///         let inst_id = arg.get("instId")?.as_str()?;
///         Some(format!("{}:{}", channel, inst_id))
///     }
/// }
/// ```
pub trait SubscriptionBuilder: Clone + Send + Sync + 'static {
    /// 构建订阅消息
    ///
    /// # 参数
    ///
    /// - `channels`: 要订阅的频道列表
    ///
    /// # 返回
    ///
    /// 返回交易所特定格式的订阅消息 JSON
    fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value>;

    /// 构建取消订阅消息
    ///
    /// # 参数
    ///
    /// - `channels`: 要取消订阅的频道列表
    ///
    /// # 返回
    ///
    /// 返回交易所特定格式的取消订阅消息 JSON
    fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value>;

    /// 从消息中提取频道标识
    ///
    /// 用于路由消息到正确的订阅者。
    ///
    /// # 参数
    ///
    /// - `msg`: 收到的消息
    ///
    /// # 返回
    ///
    /// 返回频道标识符，如果无法提取则返回 None
    fn extract_channel(&self, msg: &Value) -> Option<String>;

    /// 从消息中提取频道标识（带市场类型上下文）
    ///
    /// 与 `extract_channel` 功能相同，但额外接收市场类型上下文。
    /// 交易所可以利用此信息在 channel key 中嵌入市场类型，
    /// 实现更健壮的现货/合约消息路由（参考 Bitget 的 instType 前缀设计）。
    ///
    /// # 默认实现
    ///
    /// 默认委托给 `extract_channel`，向后兼容现有交易所。
    ///
    /// # 参数
    ///
    /// - `msg`: 收到的消息
    /// - `market_type`: 此连接对应的市场类型（从 WsContext 获取）
    fn extract_channel_with_context(
        &self,
        msg: &Value,
        _market_type: Option<MarketType>,
    ) -> Option<String> {
        self.extract_channel(msg)
    }

    /// 从订阅参数生成预期的频道标识
    ///
    /// 用于订阅时注册到订阅管理器的 key，必须与 `extract_channel` 返回的格式一致。
    ///
    /// # 参数
    ///
    /// - `channel_type`: 频道类型
    /// - `symbol`: 交易对符号
    /// - `params`: 额外参数（如 K线周期）
    ///
    /// # 返回
    ///
    /// 返回预期的频道标识符
    fn extract_channel_from_subscription(
        &self,
        channel_type: &crate::ws::subscription::ChannelType,
        symbol: &str,
        params: &std::collections::HashMap<String, Value>,
    ) -> String;

    /// 从 SubscriptionInfo 重建 SubscriptionChannel
    ///
    /// **核心方法**：用于重连时恢复订阅。
    ///
    /// 每个交易所自己解析自己的格式（例如 Binance 的 "spot:btcusdt@ticker"，
    /// Bybit 的 "tickers.BTCUSDT" 等），Core 层不需要知道任何交易所特定格式。
    ///
    /// # 参数
    ///
    /// - `info`: 订阅信息（包含 channel, symbol, params）
    ///
    /// # 返回
    ///
    /// 返回重建的 SubscriptionChannel
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // Binance 实现
    /// fn rebuild_subscription_channel(&self, info: &SubscriptionInfo) -> Result<SubscriptionChannel> {
    ///     // info.channel = "spot:btcusdt@ticker"
    ///     // info.symbol = Some("btcusdt@ticker")
    ///     
    ///     // 从 symbol 中提取
    ///     if let Some(at_pos) = info.symbol.find('@') {
    ///         let binance_symbol = &info.symbol[..at_pos]; // "btcusdt"
    ///         let stream_type = &info.symbol[at_pos+1..]; // "ticker"
    ///         
    ///         Ok(SubscriptionChannel {
    ///             channel_type: ChannelType::Ticker,
    ///             symbol: binance_symbol.to_uppercase(),
    ///             params: info.params.clone().unwrap_or_default(),
    ///             market_type: MarketType::Spot,
    ///             is_private: false,
    ///         })
    ///     } else {
    ///         // 标准格式处理
    ///         // ...
    ///     }
    /// }
    /// ```
    fn rebuild_subscription_channel(&self, info: &SubscriptionInfo) -> Result<SubscriptionChannel>;

    /// 构建心跳消息（可选）
    ///
    /// 用于保持连接活跃
    fn build_ping(&self) -> Option<Value> {
        None
    }

    /// 构建心跳响应消息（可选）
    ///
    /// 用于响应服务器的心跳请求
    fn build_pong(&self, _msg: &Value) -> Option<Value> {
        None
    }

    /// 检查消息是否为心跳消息
    fn is_ping(&self, _msg: &Value) -> bool {
        false
    }

    /// 检查消息是否为订阅确认
    fn is_subscription_confirm(&self, _msg: &Value) -> bool {
        false
    }

    /// 检查消息是否为错误消息
    fn is_error(&self, _msg: &Value) -> bool {
        false
    }

    /// 从错误消息中提取错误信息
    fn extract_error(&self, _msg: &Value) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestSubscriptionBuilder;

    impl SubscriptionBuilder for TestSubscriptionBuilder {
        fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
            Ok(serde_json::json!({
                "op": "subscribe",
                "channels": channels.iter().map(|c| &c.symbol).collect::<Vec<_>>()
            }))
        }

        fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
            Ok(serde_json::json!({
                "op": "unsubscribe",
                "channels": channels.iter().map(|c| &c.symbol).collect::<Vec<_>>()
            }))
        }

        fn extract_channel(&self, msg: &Value) -> Option<String> {
            msg.get("channel")?.as_str().map(|s| s.to_string())
        }

        fn extract_channel_from_subscription(
            &self,
            channel_type: &crate::ws::subscription::ChannelType,
            symbol: &str,
            _params: &std::collections::HashMap<String, Value>,
        ) -> String {
            if symbol.is_empty() {
                channel_type.to_string()
            } else {
                format!("{}:{}", channel_type, symbol)
            }
        }

        fn rebuild_subscription_channel(
            &self,
            info: &SubscriptionInfo,
        ) -> Result<SubscriptionChannel> {
            // 简单实现：从 symbol 中提取
            Ok(SubscriptionChannel {
                channel_type: crate::ws::subscription::ChannelType::Ticker,
                symbol: info.symbol.clone().unwrap_or_default(),
                params: info.params.clone(),
                market_type: None,
                is_private: false,
            })
        }
    }

    #[test]
    fn test_subscription_builder() {
        let builder = TestSubscriptionBuilder;
        let channels = vec![
            SubscriptionChannel::ticker("BTC/USDT"),
            SubscriptionChannel::orderbook("ETH/USDT"),
        ];

        let msg = builder.build_subscribe(&channels).unwrap();
        assert_eq!(msg["op"], "subscribe");

        let unsub = builder.build_unsubscribe(&channels).unwrap();
        assert_eq!(unsub["op"], "unsubscribe");
    }

    #[test]
    fn test_extract_channel() {
        let builder = TestSubscriptionBuilder;
        let msg = serde_json::json!({"channel": "ticker", "data": {}});
        assert_eq!(builder.extract_channel(&msg), Some("ticker".to_string()));
    }
}

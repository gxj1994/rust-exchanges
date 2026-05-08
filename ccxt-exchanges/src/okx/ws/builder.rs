//! OKX 订阅构建器

use ccxt_core::error::Result;
use ccxt_core::ws::{ChannelType, SubscriptionBuilder, SubscriptionChannel};
use serde_json::{Value, json};
use std::collections::HashMap;

/// OKX WebSocket 订阅构建器
///
/// 将通用订阅请求转换为 OKX V5 API 格式
#[derive(Debug, Clone, Copy)]
pub struct OkxSubscriptionBuilder;

impl OkxSubscriptionBuilder {
    /// 将统一符号格式转换为 OKX 格式
    ///
    /// OKX V5 API instId 格式：
    /// - 现货: BTC/USDT → BTC-USDT
    /// - 永续合约: BTC/USDT:USDT → BTC-USDT-SWAP
    /// - 交割合约: BTC/USDT:USDT-230707 → BTC-USDT-230707
    fn to_okx_symbol(symbol: &str) -> String {
        // 先去掉 '/'
        let without_slash = symbol.replace('/', "-");

        // 如果有 ':' (合约)，需要特殊处理
        if let Some(colon_pos) = without_slash.find(':') {
            let before_colon = &without_slash[..colon_pos];
            let after_colon = &without_slash[colon_pos + 1..];

            // 检查是否有交割日期 (如 USDT-230707)
            if let Some(dash_pos) = after_colon.find('-') {
                // 交割合约: BTC-USDT:USDT-230707 → BTC-USDT-230707
                format!("{}-{}", before_colon, &after_colon[dash_pos + 1..])
            } else {
                // 永续合约: BTC-USDT:USDT → BTC-USDT-SWAP
                format!("{}-SWAP", before_colon)
            }
        } else {
            // 现货: BTC-USDT
            without_slash
        }
    }

    /// 获取 OKX 频道名称
    fn get_channel_name(channel_type: ChannelType, params: &HashMap<String, Value>) -> String {
        match channel_type {
            ChannelType::Ticker => "tickers".to_string(),
            ChannelType::Tickers => "tickers".to_string(),
            ChannelType::OrderBook => {
                // 支持不同深度: books5, books, books50-l2
                params
                    .get("depth")
                    .and_then(|d| d.as_u64())
                    .map(|d| match d {
                        d if d <= 5 => "books5".to_string(),
                        d if d <= 50 => "books50-l2".to_string(),
                        _ => "books".to_string(),
                    })
                    .unwrap_or_else(|| "books5".to_string())
            }
            ChannelType::Trades => "trades".to_string(),
            ChannelType::Kline => {
                // K线频道格式: candle{interval}
                let interval = params
                    .get("interval")
                    .and_then(|i| i.as_str())
                    .unwrap_or("1m");
                format!("candle{}", interval)
            }
            ChannelType::Balance => "account".to_string(),
            ChannelType::Orders => "orders".to_string(),
            ChannelType::MyTrades => "orders".to_string(), // 成交通知来自 orders 频道
            ChannelType::UserEvents => "account".to_string(),
            ChannelType::MarkPrice => "mark-price".to_string(),
            ChannelType::BidsAsks => "tickers".to_string(),
            ChannelType::Positions => "positions".to_string(),
            ChannelType::Custom => params
                .get("channel")
                .and_then(|c| c.as_str())
                .unwrap_or("tickers")
                .to_string(),
        }
    }

    /// 构建 OKX 订阅参数
    fn build_arg(channel: &SubscriptionChannel) -> Value {
        let channel_name = Self::get_channel_name(channel.channel_type, &channel.params);

        let mut arg = serde_json::Map::new();
        arg.insert("channel".to_string(), json!(channel_name));

        // 添加 instId（交易对）
        if !channel.symbol.is_empty() {
            let okx_symbol = Self::to_okx_symbol(&channel.symbol);
            arg.insert("instId".to_string(), json!(okx_symbol));
        }

        // 私有频道可能需要 instType
        if channel.channel_type == ChannelType::Orders {
            let inst_type = channel
                .params
                .get("instType")
                .cloned()
                .unwrap_or_else(|| json!("ANY"));
            arg.insert("instType".to_string(), inst_type);
        }

        json!(arg)
    }
}

impl SubscriptionBuilder for OkxSubscriptionBuilder {
    /// 构建订阅消息
    ///
    /// # OKX 订阅格式
    ///
    /// ```json
    /// {
    ///     "op": "subscribe",
    ///     "args": [
    ///         {"channel": "tickers", "instId": "BTC-USDT"},
    ///         {"channel": "trades", "instId": "ETH-USDT"}
    ///     ]
    /// }
    /// ```
    fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let args: Vec<Value> = channels.iter().map(Self::build_arg).collect();

        Ok(json!({
            "op": "subscribe",
            "args": args
        }))
    }

    /// 构建取消订阅消息
    ///
    /// # OKX 取消订阅格式
    ///
    /// ```json
    /// {
    ///     "op": "unsubscribe",
    ///     "args": [
    ///         {"channel": "tickers", "instId": "BTC-USDT"}
    ///     ]
    /// }
    /// ```
    fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let args: Vec<Value> = channels.iter().map(Self::build_arg).collect();

        Ok(json!({
            "op": "unsubscribe",
            "args": args
        }))
    }

    /// 从消息中提取频道标识
    ///
    /// OKX 消息格式:
    /// ```json
    /// {
    ///     "arg": {"channel": "tickers", "instId": "BTC-USDT"},
    ///     "data": [...]
    /// }
    /// ```
    ///
    /// 订阅确认消息 (event: "subscribe") 不应该被路由到数据流
    fn extract_channel(&self, msg: &Value) -> Option<String> {
        // 跳过订阅确认/取消确认消息
        if msg.get("event").is_some() {
            return None;
        }

        let arg = msg.get("arg")?;
        let channel = arg.get("channel")?.as_str()?;

        // 对于 K线，频道名包含 interval (如 candle1m)
        // 我们需要返回完整的频道标识
        if channel.starts_with("candle") {
            let inst_id = arg.get("instId")?.as_str()?;
            return Some(format!("{}:{}", channel, inst_id));
        }

        // 对于其他频道，返回 channel:instId
        let inst_id = arg.get("instId").and_then(|i| i.as_str()).unwrap_or("");

        if inst_id.is_empty() {
            Some(channel.to_string())
        } else {
            Some(format!("{}:{}", channel, inst_id))
        }
    }

    /// 从订阅参数生成预期的频道标识
    ///
    /// 必须与 `extract_channel` 返回的格式一致
    fn extract_channel_from_subscription(
        &self,
        channel_type: &ChannelType,
        symbol: &str,
        params: &HashMap<String, Value>,
    ) -> String {
        let channel_name = Self::get_channel_name(*channel_type, params);

        if symbol.is_empty() {
            channel_name
        } else {
            let okx_symbol = Self::to_okx_symbol(symbol);
            format!("{}:{}", channel_name, okx_symbol)
        }
    }

    fn rebuild_subscription_channel(
        &self,
        info: &ccxt_core::network::ws_client::subscription::SubscriptionInfo,
    ) -> ccxt_core::error::Result<ccxt_core::ws::subscription::SubscriptionChannel> {
        use ccxt_core::ws::subscription::{ChannelType, SubscriptionChannel};

        // OKX 格式：info.channel 已经是标准格式 "ticker", "trades" 等
        // info.symbol 是 "BTC/USDT" 等统一格式
        let channel_type = match info.channel.as_str() {
            "tickers" | "ticker" => ChannelType::Ticker,
            "trades" => ChannelType::Trades,
            "books" | "books5" | "bbo-tbt" => ChannelType::OrderBook,
            "candle" | "candle1m" | "candle5m" | "candle15m" | "candle1H" => ChannelType::Kline,
            "account" => ChannelType::Balance,
            "orders" => ChannelType::Orders,
            _ => {
                tracing::warn!(
                    channel = %info.channel,
                    "Unknown OKX channel type, defaulting to Ticker"
                );
                ChannelType::Ticker
            }
        };

        Ok(SubscriptionChannel {
            channel_type,
            symbol: info.symbol.clone().unwrap_or_default(),
            params: info.params.clone(),
            market_type: None, // OKX 从 symbol 中可以判断
            is_private: info.channel == "account" || info.channel == "orders",
        })
    }

    /// 构建认证消息
    ///
    /// # 心跳消息
    ///
    /// OKX 使用 "ping" 字符串作为心跳
    fn build_ping(&self) -> Option<Value> {
        Some(json!("ping"))
    }

    /// 构建心跳响应
    ///
    /// OKX 需要 "pong" 响应
    fn build_pong(&self, _msg: &Value) -> Option<Value> {
        Some(json!("pong"))
    }

    /// 检查是否为心跳消息
    fn is_ping(&self, msg: &Value) -> bool {
        msg.as_str() == Some("ping")
    }

    /// 检查是否为订阅确认
    fn is_subscription_confirm(&self, msg: &Value) -> bool {
        msg.get("event")
            .and_then(|e| e.as_str())
            .map(|e| e == "subscribe" || e == "unsubscribe")
            .unwrap_or(false)
    }

    /// 检查是否为错误消息
    fn is_error(&self, msg: &Value) -> bool {
        msg.get("event")
            .and_then(|e| e.as_str())
            .map(|e| e == "error" || e == "login")
            .unwrap_or(false)
            && msg.get("code").is_some()
    }

    /// 提取错误信息
    fn extract_error(&self, msg: &Value) -> Option<String> {
        let code = msg.get("code")?.as_str().unwrap_or("unknown");
        let message = msg
            .get("msg")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        Some(format!("OKX error (code: {}): {}", code, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_okx_symbol() {
        assert_eq!(
            OkxSubscriptionBuilder::to_okx_symbol("BTC/USDT"),
            "BTC-USDT"
        );
        assert_eq!(OkxSubscriptionBuilder::to_okx_symbol("ETH/BTC"), "ETH-BTC");
    }

    #[test]
    fn test_build_subscribe_ticker() {
        let builder = OkxSubscriptionBuilder;
        let channel = SubscriptionChannel::ticker("BTC/USDT");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["op"], "subscribe");
        assert_eq!(msg["args"][0]["channel"], "tickers");
        assert_eq!(msg["args"][0]["instId"], "BTC-USDT");
    }

    #[test]
    fn test_build_subscribe_orderbook() {
        let builder = OkxSubscriptionBuilder;
        let channel = SubscriptionChannel::orderbook("BTC/USDT");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["args"][0]["channel"], "books5");
    }

    #[test]
    fn test_build_subscribe_kline() {
        let builder = OkxSubscriptionBuilder;
        let channel = SubscriptionChannel::kline("BTC/USDT", "1H");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["args"][0]["channel"], "candle1H");
        assert_eq!(msg["args"][0]["instId"], "BTC-USDT");
    }

    #[test]
    fn test_build_subscribe_balance() {
        let builder = OkxSubscriptionBuilder;
        let channel = SubscriptionChannel::balance();

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["args"][0]["channel"], "account");
    }

    #[test]
    fn test_extract_channel() {
        let builder = OkxSubscriptionBuilder;

        let msg = json!({
            "arg": {"channel": "tickers", "instId": "BTC-USDT"},
            "data": [{}]
        });

        let channel_key = builder.extract_channel(&msg);
        assert_eq!(channel_key, Some("tickers:BTC-USDT".to_string()));
    }

    #[test]
    fn test_extract_channel_kline() {
        let builder = OkxSubscriptionBuilder;

        let msg = json!({
            "arg": {"channel": "candle1H", "instId": "BTC-USDT"},
            "data": [[]]
        });

        let channel_key = builder.extract_channel(&msg);
        assert_eq!(channel_key, Some("candle1H:BTC-USDT".to_string()));
    }

    #[test]
    fn test_is_ping() {
        let builder = OkxSubscriptionBuilder;

        let msg = json!("ping");
        assert!(builder.is_ping(&msg));

        let msg = json!({"arg": {"channel": "tickers"}});
        assert!(!builder.is_ping(&msg));
    }

    #[test]
    fn test_is_error() {
        let builder = OkxSubscriptionBuilder;

        let msg = json!({
            "event": "error",
            "code": "50001",
            "msg": "Invalid parameter"
        });

        assert!(builder.is_error(&msg));
    }

    #[test]
    fn test_extract_error() {
        let builder = OkxSubscriptionBuilder;

        let msg = json!({
            "event": "error",
            "code": "50001",
            "msg": "Invalid parameter"
        });

        let error = builder.extract_error(&msg);
        assert!(error.is_some());
        assert!(error.unwrap().contains("50001"));
    }

    #[test]
    fn test_build_unsubscribe() {
        let builder = OkxSubscriptionBuilder;
        let channel = SubscriptionChannel::ticker("BTC/USDT");

        let msg = builder.build_unsubscribe(&[channel]).unwrap();

        assert_eq!(msg["op"], "unsubscribe");
    }
}

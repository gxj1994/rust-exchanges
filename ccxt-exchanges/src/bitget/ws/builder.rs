//! Bitget 订阅构建器

use crate::bitget::core::symbol::BitgetSymbolConverter;
use ccxt_core::error::Result;
use ccxt_core::ws::{ChannelType, SubscriptionBuilder, SubscriptionChannel};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Bitget WebSocket 订阅构建器
///
/// 将通用订阅请求转换为 Bitget V2 API 格式
#[derive(Debug, Clone, Copy)]
pub struct BitgetSubscriptionBuilder;

impl BitgetSubscriptionBuilder {
    /// 获取 Bitget instType
    fn get_inst_type(symbol: &str) -> &'static str {
        BitgetSymbolConverter::ws_inst_type(symbol)
    }

    /// 将统一符号格式转换为 Bitget 格式
    fn to_bitget_symbol(symbol: &str) -> String {
        BitgetSymbolConverter::unified_to_exchange(symbol)
    }

    /// 获取 Bitget 频道名称 (V3 UTA)
    fn get_channel_name(channel_type: ChannelType, params: &HashMap<String, Value>) -> String {
        match channel_type {
            ChannelType::Ticker => "ticker".to_string(),
            ChannelType::Tickers => "ticker".to_string(),
            ChannelType::OrderBook => params
                .get("depth")
                .and_then(|d| d.as_u64())
                .map(|d| format!("books{}", d))
                .unwrap_or_else(|| "books5".to_string()),
            ChannelType::Trades => "publicTrade".to_string(), // V3: trade → publicTrade
            ChannelType::Kline => "kline".to_string(), // V3: candle{interval} → kline (interval独立参数)
            ChannelType::Balance => "account".to_string(),
            ChannelType::Orders => "order".to_string(), // V3: orders → order
            ChannelType::MyTrades => "order".to_string(),
            ChannelType::Custom => params
                .get("channel")
                .and_then(|c| c.as_str())
                .unwrap_or("ticker")
                .to_string(),
            _ => "ticker".to_string(),
        }
    }

    /// 从订阅参数生成预期的频道标识
    ///
    /// 必须与 `extract_channel` 返回的格式一致
    #[allow(unused)]
    fn extract_channel_from_subscription(
        channel_type: &ChannelType,
        symbol: &str,
        params: &HashMap<String, Value>,
    ) -> String {
        let inst_type = Self::get_inst_type(symbol);
        let channel_name = Self::get_channel_name(*channel_type, params);

        if symbol.is_empty() {
            format!("{}:{}", inst_type, channel_name)
        } else {
            let bitget_symbol = Self::to_bitget_symbol(symbol);
            format!("{}:{}:{}", inst_type, channel_name, bitget_symbol)
        }
    }
}

impl SubscriptionBuilder for BitgetSubscriptionBuilder {
    /// 构建订阅消息
    ///
    /// # Bitget V3 UTA 订阅格式
    ///
    /// ```json
    /// {
    ///     "op": "subscribe",
    ///     "args": [
    ///         {
    ///             "instType": "spot",
    ///             "topic": "ticker",
    ///             "symbol": "BTCUSDT"
    ///         }
    ///     ]
    /// }
    /// ```
    fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let args: Vec<Value> = channels
            .iter()
            .map(|ch| {
                let channel_name = Self::get_channel_name(ch.channel_type, &ch.params);
                let inst_type = Self::get_inst_type(&ch.symbol);
                let inst_id = if ch.symbol.is_empty() {
                    "".to_string()
                } else {
                    Self::to_bitget_symbol(&ch.symbol)
                };

                let mut arg = serde_json::Map::new();
                arg.insert("instType".to_string(), json!(inst_type));
                arg.insert("topic".to_string(), json!(channel_name)); // V3: channel → topic
                if !inst_id.is_empty() {
                    arg.insert("symbol".to_string(), json!(inst_id)); // V3: instId → symbol
                }

                // Kline需要额外的interval参数 (V3格式)
                if ch.channel_type == ChannelType::Kline {
                    if let Some(interval) = ch.params.get("interval") {
                        arg.insert("interval".to_string(), interval.clone());
                    }
                }

                json!(arg)
            })
            .collect();

        let msg = json!({
            "op": "subscribe",
            "args": args
        });

        Ok(msg)
    }

    /// 构建取消订阅消息
    fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let args: Vec<Value> = channels
            .iter()
            .map(|ch| {
                let channel_name = Self::get_channel_name(ch.channel_type, &ch.params);
                let inst_type = Self::get_inst_type(&ch.symbol);
                let inst_id = if ch.symbol.is_empty() {
                    "".to_string()
                } else {
                    Self::to_bitget_symbol(&ch.symbol)
                };

                let mut arg = serde_json::Map::new();
                arg.insert("instType".to_string(), json!(inst_type));
                arg.insert("topic".to_string(), json!(channel_name)); // V3: channel → topic
                if !inst_id.is_empty() {
                    arg.insert("symbol".to_string(), json!(inst_id)); // V3: instId → symbol
                }

                // Kline需要额外的interval参数 (V3格式)
                if ch.channel_type == ChannelType::Kline {
                    if let Some(interval) = ch.params.get("interval") {
                        arg.insert("interval".to_string(), interval.clone());
                    }
                }

                json!(arg)
            })
            .collect();

        Ok(json!({
            "op": "unsubscribe",
            "args": args
        }))
    }

    /// 从消息中提取频道标识
    ///
    /// 订阅确认消息 (event: "subscribe") 不应该被路由到数据流
    /// 只有数据消息 (有 arg 和 data 字段) 才应该被提取
    fn extract_channel(&self, msg: &Value) -> Option<String> {
        // 跳过订阅确认/取消确认消息
        if msg.get("event").is_some() {
            return None;
        }

        let arg = msg.get("arg")?;
        let inst_type = arg
            .get("instType")
            .and_then(|t| t.as_str())
            .unwrap_or("spot"); // V3: include instType
        let channel = arg.get("topic").and_then(|c| c.as_str())?; // V3: channel → topic
        let inst_id = arg.get("symbol").and_then(|i| i.as_str()).unwrap_or(""); // V3: instId → symbol

        // Include instType in channel key to distinguish spot/swap/futures
        if inst_id.is_empty() {
            Some(format!("{}:{}", inst_type, channel))
        } else {
            Some(format!("{}:{}:{}", inst_type, channel, inst_id))
        }
    }

    /// 从订阅参数生成预期的频道标识
    fn extract_channel_from_subscription(
        &self,
        channel_type: &ChannelType,
        symbol: &str,
        params: &HashMap<String, Value>,
    ) -> String {
        let inst_type = Self::get_inst_type(symbol);
        let channel_name = Self::get_channel_name(*channel_type, params);

        if symbol.is_empty() {
            format!("{}:{}", inst_type, channel_name)
        } else {
            let bitget_symbol = Self::to_bitget_symbol(symbol);
            format!("{}:{}:{}", inst_type, channel_name, bitget_symbol)
        }
    }

    fn rebuild_subscription_channel(
        &self,
        info: &ccxt_core::network::ws_client::subscription::SubscriptionInfo,
    ) -> ccxt_core::error::Result<ccxt_core::ws::subscription::SubscriptionChannel> {
        use ccxt_core::ws::subscription::{ChannelType, SubscriptionChannel};

        // Bitget 格式：
        // - info.channel = "spot" 或 "swap" (market type)
        // - info.symbol = "ticker:BTCUSDT" 或 "books5:BTCUSDT" (channel:symbol)
        // 需要从 symbol 中提取频道类型
        if let Some(ref sym) = info.symbol {
            if let Some(colon_pos) = sym.find(':') {
                let channel_name = &sym[..colon_pos]; // "ticker", "books5", "publicTrade", etc.
                let bitget_symbol = &sym[colon_pos + 1..]; // "BTCUSDT"

                let channel_type = match channel_name {
                    "ticker" => ChannelType::Ticker,
                    "publicTrade" | "trade" | "trades" => ChannelType::Trades,
                    "books" | "books5" | "book" | "books15" => ChannelType::OrderBook,
                    "kline" | "candle" | "candle1m" | "candle5m" => ChannelType::Kline,
                    "account" | "balance" => ChannelType::Balance,
                    "order" | "orders" => ChannelType::Orders,
                    _ => {
                        tracing::warn!(
                            channel_name = %channel_name,
                            channel = %info.channel,
                            "Unknown Bitget channel name, defaulting to Ticker"
                        );
                        ChannelType::Ticker
                    }
                };

                return Ok(SubscriptionChannel {
                    channel_type,
                    symbol: bitget_symbol.to_string(),
                    params: info.params.clone(),
                    market_type: if info.channel == "swap" {
                        Some(ccxt_core::ws::subscription::MarketType::Swap)
                    } else {
                        Some(ccxt_core::ws::subscription::MarketType::Spot)
                    },
                    is_private: channel_name == "account" || channel_name == "order",
                });
            }
        }

        // Fallback: 如果无法解析，使用默认值
        tracing::warn!(
            channel = %info.channel,
            symbol = ?info.symbol,
            "Failed to parse Bitget subscription info, using defaults"
        );

        Ok(SubscriptionChannel {
            channel_type: ChannelType::Ticker,
            symbol: info.symbol.clone().unwrap_or_default(),
            params: info.params.clone(),
            market_type: Some(ccxt_core::ws::subscription::MarketType::Spot),
            is_private: false,
        })
    }

    /// 构建心跳消息
    fn build_ping(&self) -> Option<Value> {
        Some(json!("ping"))
    }

    /// 构建心跳响应
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
            .map(|e| e == "error")
            .unwrap_or(false)
    }

    /// 提取错误信息
    fn extract_error(&self, msg: &Value) -> Option<String> {
        let code = msg
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or("unknown");
        let message = msg
            .get("msg")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        Some(format!("Bitget error (code: {}): {}", code, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_subscribe_ticker() {
        let builder = BitgetSubscriptionBuilder;
        let channel = SubscriptionChannel::ticker("BTC/USDT");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["op"], "subscribe");
        assert_eq!(msg["args"][0]["topic"], "ticker"); // V3: channel → topic
        assert_eq!(msg["args"][0]["symbol"], "BTCUSDT"); // V3: instId → symbol
        assert_eq!(msg["args"][0]["instType"], "spot"); // V3: lowercase
    }

    #[test]
    fn test_build_subscribe_orderbook() {
        let builder = BitgetSubscriptionBuilder;
        let channel = SubscriptionChannel::orderbook("BTC/USDT");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["args"][0]["topic"], "books5"); // V3: channel → topic
    }

    #[test]
    fn test_is_ping() {
        let builder = BitgetSubscriptionBuilder;

        let msg = json!("ping");
        assert!(builder.is_ping(&msg));

        let msg = json!({"op": "subscribe"});
        assert!(!builder.is_ping(&msg));
    }

    #[test]
    fn test_is_error() {
        let builder = BitgetSubscriptionBuilder;

        let msg = json!({
            "event": "error",
            "code": "50001",
            "msg": "Invalid parameter"
        });

        assert!(builder.is_error(&msg));
    }
}

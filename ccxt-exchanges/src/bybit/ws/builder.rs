//! Bybit 订阅构建器

use ccxt_core::error::Result;
use ccxt_core::ws::subscription::MarketType;
use ccxt_core::ws::{ChannelType, SubscriptionBuilder, SubscriptionChannel};
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::bybit::to_bybit_interval;

/// Bybit WebSocket 订阅构建器
///
/// 将通用订阅请求转换为 Bybit V5 API 格式
#[derive(Debug, Clone, Copy)]
pub struct BybitSubscriptionBuilder;

impl BybitSubscriptionBuilder {
    /// 将统一符号格式转换为 Bybit 格式
    ///
    /// Bybit V5 API symbol 格式：
    /// - 现货: BTC/USDT → BTCUSDT
    /// - U本位永续: BTC/USDT:USDT → BTCUSDT (去掉结算币种)
    /// - U本位交割: BTC/USDT:USDT-26DEC25 → BTC-26DEC25
    fn to_bybit_symbol(symbol: &str) -> String {
        // 先去掉 '/'
        let without_slash = symbol.replace('/', "");

        // 如果有 ':' (合约)，需要特殊处理
        if let Some(colon_pos) = without_slash.find(':') {
            let before_colon = &without_slash[..colon_pos];
            let after_colon = &without_slash[colon_pos + 1..];

            // 检查是否有交割日期 (如 USDT-26DEC25)
            if let Some(dash_pos) = after_colon.find('-') {
                // 交割合约: BTCUSDT:USDT-26DEC25 → BTC-26DEC25
                // 提取 base currency 的第一个字符
                if let Some(_first_char) = before_colon.chars().next() {
                    let base =
                        before_colon.trim_end_matches(after_colon.split('-').next().unwrap_or(""));
                    format!("{}-{}", base, &after_colon[dash_pos + 1..])
                } else {
                    before_colon.to_string()
                }
            } else {
                // 永续合约: BTCUSDT:USDT → BTCUSDT
                before_colon.to_string()
            }
        } else {
            // 现货: BTCUSDT
            without_slash
        }
    }

    /// 获取 Bybit topic 名称
    fn get_topic(channel_type: ChannelType, params: &HashMap<String, Value>) -> String {
        match channel_type {
            ChannelType::Ticker => "tickers".to_string(),
            ChannelType::Tickers => "tickers".to_string(),
            ChannelType::BidsAsks => "orderbook.1".to_string(), // 1档最优买卖价
            ChannelType::OrderBook => params
                .get("depth")
                .and_then(|d| d.as_u64())
                .map(|d| format!("orderbook.{}", d))
                .unwrap_or_else(|| "orderbook.50".to_string()),
            ChannelType::Trades => "publicTrade".to_string(),
            ChannelType::Kline => {
                // Normalize interval: ccxt unified format ("1m") -> Bybit format ("1")
                let interval = params
                    .get("interval")
                    .and_then(|i| i.as_str())
                    .and_then(to_bybit_interval)
                    .unwrap_or("1");
                format!("kline.{}", interval)
            }
            ChannelType::Balance => "wallet".to_string(),
            ChannelType::Orders => "order".to_string(),
            ChannelType::MyTrades => "execution".to_string(),
            ChannelType::Custom => params
                .get("channel")
                .and_then(|c| c.as_str())
                .unwrap_or("tickers")
                .to_string(),
            _ => "tickers".to_string(),
        }
    }
}

impl SubscriptionBuilder for BybitSubscriptionBuilder {
    /// 构建订阅消息
    ///
    /// # Bybit 订阅格式
    ///
    /// ```json
    /// {
    ///     "op": "subscribe",
    ///     "args": ["tickers.BTCUSDT"]
    /// }
    /// ```
    fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let args: Vec<String> = channels
            .iter()
            .map(|ch| {
                let topic = Self::get_topic(ch.channel_type, &ch.params);
                if ch.symbol.is_empty() {
                    topic
                } else {
                    let symbol = Self::to_bybit_symbol(&ch.symbol);
                    format!("{}.{}", topic, symbol)
                }
            })
            .collect();

        Ok(json!({
            "op": "subscribe",
            "args": args
        }))
    }

    /// 构建取消订阅消息
    fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let args: Vec<String> = channels
            .iter()
            .map(|ch| {
                let topic = Self::get_topic(ch.channel_type, &ch.params);
                if ch.symbol.is_empty() {
                    topic
                } else {
                    let symbol = Self::to_bybit_symbol(&ch.symbol);
                    format!("{}.{}", topic, symbol)
                }
            })
            .collect();

        Ok(json!({
            "op": "unsubscribe",
            "args": args
        }))
    }

    /// 从消息中提取频道标识
    fn extract_channel(&self, msg: &Value) -> Option<String> {
        msg.get("topic")?.as_str().map(|s| s.to_string())
    }

    /// 从消息中提取频道标识（带市场类型上下文）
    ///
    /// 在 channel key 中嵌入 market_type 前缀，参考 Bitget 的 instType 前缀设计。
    /// 即使将来 Bybit 合并 URL，现货/合约消息也不会串扰。
    /// 格式: `spot:tickers.BTCUSDT` 或 `swap:tickers.BTCUSDT`
    fn extract_channel_with_context(
        &self,
        msg: &Value,
        market_type: Option<MarketType>,
    ) -> Option<String> {
        let topic = msg.get("topic")?.as_str()?;
        match market_type {
            Some(MarketType::Spot) => Some(format!("spot:{}", topic)),
            Some(MarketType::Swap) => Some(format!("swap:{}", topic)),
            Some(MarketType::Future) => Some(format!("future:{}", topic)),
            Some(MarketType::Option) => Some(format!("option:{}", topic)),
            None => Some(topic.to_string()),
        }
    }

    /// 从订阅参数生成预期的频道标识
    ///
    /// 格式: `{market}:{topic}.{symbol}` (如 `spot:tickers.BTCUSDT` vs `swap:tickers.BTCUSDT`)
    fn extract_channel_from_subscription(
        &self,
        channel_type: &ChannelType,
        symbol: &str,
        params: &HashMap<String, Value>,
    ) -> String {
        let topic = Self::get_topic(*channel_type, params);
        let channel_key = if symbol.is_empty() {
            topic
        } else {
            let bybit_symbol = Self::to_bybit_symbol(symbol);
            format!("{}.{}", topic, bybit_symbol)
        };

        // 嵌入市场类型前缀，实现现货/合约 channel key 区分
        // 参考 Bitget 的 instType 前缀设计
        let market = if symbol.contains(':') {
            "swap"
        } else if symbol.is_empty() {
            ""
        } else {
            "spot"
        };

        if market.is_empty() {
            channel_key
        } else {
            format!("{}:{}", market, channel_key)
        }
    }

    fn rebuild_subscription_channel(
        &self,
        info: &ccxt_core::network::ws_client::subscription::SubscriptionInfo,
    ) -> ccxt_core::error::Result<ccxt_core::ws::subscription::SubscriptionChannel> {
        use ccxt_core::ws::subscription::{ChannelType, SubscriptionChannel};

        // info.symbol 格式: "orderbook.50.BTCUSDT" 或 "tickers.BTCUSDT"
        // 需要从 symbol 中提取频道类型和原始 symbol
        if let Some(ref sym) = info.symbol {
            let parts: Vec<&str> = sym.split('.').collect();

            if parts.len() >= 2 {
                let topic = parts[0]; // "orderbook", "tickers", "trade", etc.

                // 判断是否有深度参数（OrderBook 特有）
                let (depth, bybit_symbol) = if parts.len() >= 3 && topic == "orderbook" {
                    // "orderbook.50.BTCUSDT" - parts = ["orderbook", "50", "BTCUSDT"]
                    let depth_str = parts[1];
                    let symbol = parts[2..].join("."); // 处理 symbol 中包含点号的情况
                    (depth_str.parse::<u64>().ok(), symbol)
                } else {
                    // "tickers.BTCUSDT" - parts = ["tickers", "BTCUSDT"]
                    (None, parts[1..].join("."))
                };

                let mut params = info.params.clone();

                let channel_type = match topic {
                    "tickers" | "ticker" => ChannelType::Ticker,
                    "trade" | "trades" | "publicTrade" => ChannelType::Trades,
                    "orderbook" | "depth" => ChannelType::OrderBook,
                    "kline" | "candle" => {
                        // Kline: 提取 interval 参数
                        // topic 格式: "kline" 或 "kline.5" (Bybit 格式)
                        // 需要从原始 channel key 中提取
                        if parts.len() >= 2 && topic == "kline" {
                            let bybit_interval = parts[1];
                            // 将 Bybit interval 转换为 ccxt 格式
                            // Bybit: "1", "3", "5", "15", "30", "60", "120", "240", "360", "720", "D", "W", "M"
                            // ccxt: "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "12h", "1d", "1w", "1M"
                            let ccxt_interval = match bybit_interval {
                                "1" => "1m",
                                "3" => "3m",
                                "5" => "5m",
                                "15" => "15m",
                                "30" => "30m",
                                "60" => "1h",
                                "120" => "2h",
                                "240" => "4h",
                                "360" => "6h",
                                "720" => "12h",
                                "D" => "1d",
                                "W" => "1w",
                                "M" => "1M",
                                _ => bybit_interval,
                            };
                            params.insert("interval".to_string(), serde_json::json!(ccxt_interval));
                        }
                        ChannelType::Kline
                    }
                    // 私有频道
                    "wallet" => ChannelType::Balance,
                    "order" | "orders" => ChannelType::Orders,
                    "execution" | "executions" => ChannelType::MyTrades,
                    "position" | "positions" => ChannelType::Positions,
                    _ => {
                        tracing::warn!(
                            topic = %topic,
                            channel = %info.channel,
                            "Unknown Bybit topic, defaulting to Ticker"
                        );
                        ChannelType::Ticker
                    }
                };

                if let Some(d) = depth {
                    params.insert("depth".to_string(), serde_json::json!(d));
                }

                // 判断是否为私有频道
                let is_private = matches!(
                    channel_type,
                    ChannelType::Balance
                        | ChannelType::Orders
                        | ChannelType::MyTrades
                        | ChannelType::Positions
                );

                return Ok(SubscriptionChannel {
                    channel_type,
                    symbol: bybit_symbol,
                    params,
                    market_type: if info.channel.starts_with("swap:") {
                        Some(ccxt_core::ws::subscription::MarketType::Swap)
                    } else {
                        Some(ccxt_core::ws::subscription::MarketType::Spot)
                    },
                    is_private,
                });
            }
        }

        // Fallback: 如果无法解析，使用默认值
        tracing::warn!(
            channel = %info.channel,
            symbol = ?info.symbol,
            "Failed to parse Bybit subscription info, using defaults"
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
        Some(json!({"op": "ping"}))
    }

    /// 构建心跳响应
    fn build_pong(&self, _msg: &Value) -> Option<Value> {
        Some(json!({"op": "pong"}))
    }

    /// 检查是否为心跳消息
    fn is_ping(&self, msg: &Value) -> bool {
        msg.get("op")
            .and_then(|o| o.as_str())
            .map(|o| o == "ping")
            .unwrap_or(false)
    }

    /// 检查是否为订阅确认
    fn is_subscription_confirm(&self, msg: &Value) -> bool {
        msg.get("op")
            .and_then(|o| o.as_str())
            .map(|o| o == "subscribe" || o == "unsubscribe")
            .unwrap_or(false)
            && msg.get("success").is_some()
    }

    /// 检查是否为错误消息
    fn is_error(&self, msg: &Value) -> bool {
        msg.get("success")
            .and_then(|s| s.as_bool())
            .map(|s| !s)
            .unwrap_or(false)
            && msg.get("ret_msg").is_some()
    }

    /// 提取错误信息
    fn extract_error(&self, msg: &Value) -> Option<String> {
        let code = msg.get("ret_code").and_then(|c| c.as_i64()).unwrap_or(-1);
        let message = msg
            .get("ret_msg")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        Some(format!("Bybit error (code: {}): {}", code, message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_to_bybit_symbol() {
        // 现货
        assert_eq!(
            BybitSubscriptionBuilder::to_bybit_symbol("BTC/USDT"),
            "BTCUSDT"
        );
        // U本位永续合约 - 去掉结算币种
        assert_eq!(
            BybitSubscriptionBuilder::to_bybit_symbol("BTC/USDT:USDT"),
            "BTCUSDT"
        );
        // U本位交割合约 - 保留日期
        assert_eq!(
            BybitSubscriptionBuilder::to_bybit_symbol("BTC/USDT:USDT-26DEC25"),
            "BTC-26DEC25"
        );
    }

    #[test]
    fn test_build_subscribe_ticker() {
        let builder = BybitSubscriptionBuilder;
        let channel = SubscriptionChannel::ticker("BTC/USDT");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["op"], "subscribe");
        assert_eq!(msg["args"][0], "tickers.BTCUSDT");
    }

    #[test]
    fn test_build_subscribe_orderbook() {
        let builder = BybitSubscriptionBuilder;
        let channel = SubscriptionChannel::orderbook("BTC/USDT");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["args"][0], "orderbook.50.BTCUSDT");
    }

    #[test]
    fn test_build_subscribe_kline() {
        let builder = BybitSubscriptionBuilder;
        let channel = SubscriptionChannel::kline("BTC/USDT", "1");

        let msg = builder.build_subscribe(&[channel]).unwrap();

        assert_eq!(msg["args"][0], "kline.1.BTCUSDT");
    }

    #[test]
    fn test_is_ping() {
        let builder = BybitSubscriptionBuilder;

        let msg = json!({"op": "ping"});
        assert!(builder.is_ping(&msg));

        let msg = json!({"op": "subscribe", "args": ["tickers.BTCUSDT"]});
        assert!(!builder.is_ping(&msg));
    }

    #[test]
    fn test_is_error() {
        let builder = BybitSubscriptionBuilder;

        let msg = json!({
            "success": false,
            "ret_code": 10001,
            "ret_msg": "Invalid parameter"
        });

        assert!(builder.is_error(&msg));
    }
}

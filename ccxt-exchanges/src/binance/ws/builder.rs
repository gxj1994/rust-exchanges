//! Binance WebSocket 订阅构建器
//!
//! 将通用订阅请求转换为 Binance WebSocket API 格式。
//!
//! # OrderBook 订阅策略
//!
//! 默认使用**快照流** (`@depth{5,10,20}@100ms`)：
//! - 每次推送完整的 N 档深度
//! - 直接替换本地订单簿，无需增量管理
//! - 简单可靠，适合大多数场景
//!
//! 如需**增量流** (`@depth@100ms`)：
//! - 设置 `depth=0`
//! - 需要自行处理 REST API 快照同步
//! - 需要处理增量序号连续性
//!
//! # 示例
//!
//! ```rust,ignore
//! // 快照流（默认）
//! let channel = SubscriptionChannel::order_book("BTC/USDT");
//! // 结果: btcusdt@depth20@100ms
//!
//! // 自定义深度
//! let channel = SubscriptionChannel::order_book("BTC/USDT")
//!     .with_param("depth", json!(10));
//! // 结果: btcusdt@depth10@100ms
//!
//! // 增量流（高级用法）
//! let channel = SubscriptionChannel::order_book("BTC/USDT")
//!     .with_param("depth", json!(0));
//! // 结果: btcusdt@depth@100ms
//! ```

use crate::binance::core::symbol::BinanceSymbolConverter;
use ccxt_core::error::Result;
use ccxt_core::ws::subscription::MarketType;
use ccxt_core::ws::{ChannelType, SubscriptionBuilder, SubscriptionChannel};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Binance WebSocket 订阅构建器
///
/// 将通用订阅请求转换为 Binance WebSocket API 格式
#[derive(Debug, Clone, Copy)]
pub struct BinanceSubscriptionBuilder;

impl BinanceSubscriptionBuilder {
    /// 将统一符号格式转换为 Binance 格式
    fn to_binance_symbol(symbol: &str) -> String {
        BinanceSymbolConverter::unified_to_exchange(symbol)
    }

    /// 获取 Binance stream 名称
    fn get_stream_name(
        channel_type: ChannelType,
        symbol: &str,
        params: &HashMap<String, Value>,
    ) -> String {
        let symbol = Self::to_binance_symbol(symbol);

        match channel_type {
            ChannelType::Ticker => format!("{}@ticker", symbol.to_lowercase()),
            ChannelType::Tickers => format!("{}@ticker", symbol.to_lowercase()),
            ChannelType::OrderBook => {
                // 默认使用快照流 (@depth{N}@100ms)
                // - 快照流：每次推送完整的 N 档深度，直接替换本地订单簿
                // - 增量流：需要 REST API 快照同步，复杂度高
                //
                // 可选参数：
                // - depth: 5, 10, 20（默认 20）
                // - speed: "100ms", "1000ms"（默认 "100ms"）
                //
                // 如果需要增量流，设置 depth=0 并自行处理快照同步
                let depth = params.get("depth").and_then(|d| d.as_u64()).unwrap_or(20);
                let speed = params
                    .get("speed")
                    .and_then(|s| s.as_str())
                    .unwrap_or("100ms");

                if depth == 0 {
                    // 增量流：需要快照同步
                    format!("{}@depth@{}", symbol.to_lowercase(), speed)
                } else {
                    // 快照流：直接使用
                    format!("{}@depth{}@{}", symbol.to_lowercase(), depth, speed)
                }
            }
            ChannelType::Trades => format!("{}@aggTrade", symbol.to_lowercase()),
            ChannelType::Kline => {
                let interval = params
                    .get("interval")
                    .and_then(|i| i.as_str())
                    .unwrap_or("1m");
                format!("{}@kline_{}", symbol.to_lowercase(), interval)
            }
            ChannelType::MarkPrice => {
                // 标记价格流，支持 1s 或 3s 更新频率
                // 默认使用 3s（不带后缀），因为消息中没有频率信息
                // 这样可以确保 extract_channel 提取的 key 能匹配
                let freq = params.get("freq").and_then(|f| f.as_str()).unwrap_or("3s");
                if freq == "1s" {
                    format!("{}@markPrice@1s", symbol.to_lowercase())
                } else {
                    // 3s 更新频率，不带后缀
                    format!("{}@markPrice", symbol.to_lowercase())
                }
            }
            ChannelType::BidsAsks => {
                // 最优买卖价 (bookTicker)
                format!("{}@bookTicker", symbol.to_lowercase())
            }
            ChannelType::Balance
            | ChannelType::Orders
            | ChannelType::MyTrades
            | ChannelType::Positions => {
                // 用户数据流需要 listenKey
                "userData".to_string()
            }
            ChannelType::Custom => params
                .get("stream")
                .and_then(|c| c.as_str())
                .unwrap_or(&format!("{}@ticker", symbol.to_lowercase()))
                .to_string(),
            _ => format!("{}@ticker", symbol.to_lowercase()),
        }
    }
}

impl SubscriptionBuilder for BinanceSubscriptionBuilder {
    /// 构建订阅消息
    ///
    /// # Binance 订阅格式
    ///
    /// ```json
    /// {
    ///     "method": "SUBSCRIBE",
    ///     "params": ["btcusdt@ticker"],
    ///     "id": 1
    /// }
    /// ```
    fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let params: Vec<String> = channels
            .iter()
            .map(|ch| {
                if ch.symbol.is_empty() {
                    // 对于 userData，返回固定名称
                    if matches!(
                        ch.channel_type,
                        ChannelType::Balance
                            | ChannelType::Orders
                            | ChannelType::MyTrades
                            | ChannelType::Positions
                    ) {
                        "userData".to_string()
                    } else {
                        ch.params
                            .get("stream")
                            .and_then(|s| s.as_str())
                            .unwrap_or("btcusdt@ticker")
                            .to_string()
                    }
                } else {
                    Self::get_stream_name(ch.channel_type, &ch.symbol, &ch.params)
                }
            })
            .collect();

        Ok(json!({
            "method": "SUBSCRIBE",
            "params": params,
            "id": 1
        }))
    }

    /// 构建取消订阅消息
    fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let params: Vec<String> = channels
            .iter()
            .map(|ch| {
                if ch.symbol.is_empty() {
                    if matches!(
                        ch.channel_type,
                        ChannelType::Balance
                            | ChannelType::Orders
                            | ChannelType::MyTrades
                            | ChannelType::Positions
                    ) {
                        "userData".to_string()
                    } else {
                        ch.params
                            .get("stream")
                            .and_then(|s| s.as_str())
                            .unwrap_or("btcusdt@ticker")
                            .to_string()
                    }
                } else {
                    Self::get_stream_name(ch.channel_type, &ch.symbol, &ch.params)
                }
            })
            .collect();

        Ok(json!({
            "method": "UNSUBSCRIBE",
            "params": params,
            "id": 1
        }))
    }

    /// 从消息中提取频道标识
    fn extract_channel(&self, msg: &Value) -> Option<String> {
        // Binance 使用 stream 字段或从事件类型和 symbol 组合
        if let Some(stream) = msg.get("stream").and_then(|s| s.as_str()) {
            return Some(stream.to_string());
        }

        // 尝试从事件类型和 symbol 组合
        if let (Some(event), Some(symbol)) = (
            msg.get("e").and_then(|e| e.as_str()),
            msg.get("s").and_then(|s| s.as_str()),
        ) {
            // 对于 kline 事件，需要从 k.i 字段提取时间间隔
            if event == "kline" {
                if let Some(interval) = msg
                    .get("k")
                    .and_then(|k| k.get("i"))
                    .and_then(|i| i.as_str())
                {
                    return Some(format!("{}@kline_{}", symbol.to_lowercase(), interval));
                }
            }

            // 将 Binance 的事件类型映射到统一的频道类型
            let channel_type = match event {
                "24hrTicker" | "24hrMiniTicker" => "ticker",
                "depthUpdate" | "depth" => "depth",
                "trade" | "aggTrade" => "aggTrade",
                "kline" => "kline", // 如果没有 interval，使用通用 key
                "markPriceUpdate" => {
                    // markPrice 需要检查是否有 @1s 后缀
                    // 订阅时可能是 btcusdt@markPrice 或 btcusdt@markPrice@1s
                    // 消息中没有频率信息，使用默认的 markPrice（不带@1s）
                    "markPrice"
                }
                "bookTicker" => "bookTicker",
                _ => event,
            };

            return Some(format!("{}@{}", symbol.to_lowercase(), channel_type));
        }

        // 对于 OrderBook 快照流（没有 e 和 s 字段），检查是否有 bids/asks
        // 这种消息在 single stream 模式下不包含 symbol 信息
        if msg.get("bids").is_some() || msg.get("asks").is_some() {
            // 返回简化的 depth key，与订阅时使用的 key 匹配
            // 注意：这假设当前只有一个 OrderBook 订阅
            // 如果有多个，需要更复杂的逻辑来匹配
            return Some("depth".to_string());
        }

        // bookTicker 消息没有 e 字段，但有 b (bid) 和 a (ask) 字段
        // 格式: { "u": 2912757203, "s": "BTCUSDT", "b": "18950.17", "B": "45.8740", "a": "18952.08", "A": "10.5016" }
        if msg.get("b").is_some() && msg.get("a").is_some() {
            if let Some(symbol) = msg.get("s").and_then(|s| s.as_str()) {
                return Some(format!("{}@bookTicker", symbol.to_lowercase()));
            }
        }

        None
    }

    /// 从消息中提取频道标识（带市场类型上下文）
    ///
    /// 在 channel key 中嵌入 market_type 前缀，实现现货/合约消息路由区分。
    /// 格式: `spot:btcusdt@ticker` 或 `swap:btcusdt@ticker`
    fn extract_channel_with_context(
        &self,
        msg: &Value,
        market_type: Option<MarketType>,
    ) -> Option<String> {
        let channel = self.extract_channel(msg)?;
        match market_type {
            Some(MarketType::Spot) => Some(format!("spot:{}", channel)),
            Some(MarketType::Swap) => Some(format!("swap:{}", channel)),
            Some(MarketType::Future) => Some(format!("future:{}", channel)),
            Some(MarketType::Option) => Some(format!("option:{}", channel)),
            None => Some(channel),
        }
    }

    /// 从订阅参数生成预期的频道标识
    fn extract_channel_from_subscription(
        &self,
        channel_type: &ChannelType,
        symbol: &str,
        params: &HashMap<String, Value>,
    ) -> String {
        let channel_key = if symbol.is_empty() {
            // 对于 userData 等私有频道，区分子类型
            if matches!(channel_type, ChannelType::Balance) {
                "userData:balance".to_string()
            } else if matches!(channel_type, ChannelType::Orders) {
                "userData:orders".to_string()
            } else if matches!(channel_type, ChannelType::MyTrades) {
                "userData:trades".to_string()
            } else if matches!(channel_type, ChannelType::Positions) {
                "userData:positions".to_string()
            } else {
                params
                    .get("stream")
                    .and_then(|s| s.as_str())
                    .unwrap_or("btcusdt@ticker")
                    .to_string()
            }
        } else {
            // 对于所有频道类型，使用完整 stream name 以保留参数信息
            Self::get_stream_name(*channel_type, symbol, params)
        };

        // 嵌入市场类型前缀，实现现货/合约 channel key 区分
        // 参考 Bybit 的设计，区分 spot 和 swap 的相同 symbol
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

        // info.symbol 格式: "btcusdt@ticker" 或 "btcusdt@depth"
        // 需要从 symbol 中提取频道类型和原始 symbol
        if let Some(ref sym) = info.symbol {
            if let Some(at_pos) = sym.find('@') {
                let binance_symbol = &sym[..at_pos]; // "btcusdt"
                let stream_type = &sym[at_pos + 1..]; // "ticker", "depth", etc.

                // 转换为统一格式 (BTCUSDT -> BTC/USDT)
                let unified_symbol = binance_symbol.to_uppercase();

                let channel_type = match stream_type {
                    "ticker" => ChannelType::Ticker,
                    "trade" | "trades" | "aggTrade" => ChannelType::Trades,
                    "depth" | "depth@100ms" | "depth@50ms" | "depth@0ms" => {
                        // OrderBook: 提取 depth 和 speed 参数
                        let mut params = info.params.clone();

                        // 解析 depth20@100ms 或 depth@100ms
                        if stream_type.starts_with("depth") {
                            let rest = &stream_type[5..]; // "20@100ms" 或 "@100ms" 或 空
                            if let Some(at_pos) = rest.find('@') {
                                let depth_str = &rest[..at_pos];
                                if !depth_str.is_empty() {
                                    if let Ok(d) = depth_str.parse::<u64>() {
                                        params.insert("depth".to_string(), serde_json::json!(d));
                                    }
                                }
                                let speed = &rest[at_pos + 1..];
                                params.insert("speed".to_string(), serde_json::json!(speed));
                            } else if !rest.is_empty() {
                                // 只有 depth 没有 speed
                                if let Ok(d) = rest.parse::<u64>() {
                                    params.insert("depth".to_string(), serde_json::json!(d));
                                }
                            }
                        }

                        // 参数完整性校验
                        if !params.contains_key("depth") {
                            tracing::debug!(
                                stream_type = %stream_type,
                                "OrderBook depth parameter not extracted, using default"
                            );
                        }

                        return Ok(SubscriptionChannel {
                            channel_type: ChannelType::OrderBook,
                            symbol: unified_symbol,
                            params,
                            market_type: if info.channel.starts_with("swap:") {
                                Some(ccxt_core::ws::subscription::MarketType::Swap)
                            } else {
                                Some(ccxt_core::ws::subscription::MarketType::Spot)
                            },
                            is_private: false,
                        });
                    }
                    "kline" | "kline_1m" | "kline_5m" | "kline_15m" | "kline_1h" | "kline_4h"
                    | "kline_1d" => {
                        // Kline: 提取 interval 参数
                        let mut params = info.params.clone();

                        if let Some(underscore_pos) = stream_type.find('_') {
                            let interval = &stream_type[underscore_pos + 1..];
                            params.insert("interval".to_string(), serde_json::json!(interval));
                        } else {
                            params.insert("interval".to_string(), serde_json::json!("1m"));
                        }

                        // 参数完整性校验
                        if !params.contains_key("interval") {
                            tracing::error!(
                                stream_type = %stream_type,
                                "Kline interval parameter extraction failed!"
                            );
                        }

                        return Ok(SubscriptionChannel {
                            channel_type: ChannelType::Kline,
                            symbol: unified_symbol,
                            params,
                            market_type: if info.channel.starts_with("swap:") {
                                Some(ccxt_core::ws::subscription::MarketType::Swap)
                            } else {
                                Some(ccxt_core::ws::subscription::MarketType::Spot)
                            },
                            is_private: false,
                        });
                    }
                    "markPrice" | "markPrice@1s" => {
                        // MarkPrice: 提取频率参数
                        let mut params = info.params.clone();

                        if stream_type.contains("@1s") {
                            params.insert("freq".to_string(), serde_json::json!("1s"));
                        } else {
                            params.insert("freq".to_string(), serde_json::json!("3s"));
                        }

                        return Ok(SubscriptionChannel {
                            channel_type: ChannelType::MarkPrice,
                            symbol: unified_symbol,
                            params,
                            market_type: if info.channel.starts_with("swap:") {
                                Some(ccxt_core::ws::subscription::MarketType::Swap)
                            } else {
                                Some(ccxt_core::ws::subscription::MarketType::Spot)
                            },
                            is_private: false,
                        });
                    }
                    _ => {
                        tracing::warn!(
                            stream_type = %stream_type,
                            channel = %info.channel,
                            "Unknown Binance stream type, defaulting to Ticker"
                        );
                        ChannelType::Ticker
                    }
                };

                return Ok(SubscriptionChannel {
                    channel_type,
                    symbol: unified_symbol,
                    params: info.params.clone(),
                    market_type: if info.channel.starts_with("swap:") {
                        Some(ccxt_core::ws::subscription::MarketType::Swap)
                    } else {
                        Some(ccxt_core::ws::subscription::MarketType::Spot)
                    },
                    is_private: false,
                });
            }
        }

        // 处理 userData 私有频道
        if info.channel.contains("userData") {
            let sub_type = if let Some(_colon_pos) = info.channel.find(':') {
                info.channel.split(':').nth(1).unwrap_or("balance")
            } else {
                "balance"
            };

            let channel_type = match sub_type {
                "balance" => ChannelType::Balance,
                "orders" => ChannelType::Orders,
                "trades" | "myTrades" => ChannelType::MyTrades,
                "positions" => ChannelType::Positions,
                _ => {
                    tracing::warn!(sub_type = %sub_type, "Unknown userData sub type, defaulting to Balance");
                    ChannelType::Balance
                }
            };

            return Ok(SubscriptionChannel {
                channel_type,
                symbol: String::new(),
                params: info.params.clone(),
                market_type: if info.channel.starts_with("swap:") {
                    Some(ccxt_core::ws::subscription::MarketType::Swap)
                } else {
                    Some(ccxt_core::ws::subscription::MarketType::Spot)
                },
                is_private: true, // ✅ userData 是私有频道
            });
        }

        // Fallback: 如果无法解析，使用默认值
        tracing::warn!(
            channel = %info.channel,
            symbol = ?info.symbol,
            "Failed to parse Binance subscription info, using defaults"
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
    /// Binance 使用 WebSocket 原生 ping/pong，不需要应用层心跳
    fn build_ping(&self) -> Option<Value> {
        // Binance 使用 WebSocket 层面的 ping/pong
        // 不需要应用层心跳
        None
    }

    /// 构建心跳响应
    ///
    /// Binance 现货服务器每 20 秒发送应用层 PING（JSON 格式，包含 id 字段）
    /// 客户端必须回复相同的 payload
    /// 官方文档: https://binance-docs.github.io/apidocs/spot/en/#websocket-market-streams
    fn build_pong(&self, msg: &Value) -> Option<Value> {
        // 回复相同的 payload（Binance 要求 PONG 与 PING 的 payload 一致）
        Some(msg.clone())
    }

    /// 检查是否为心跳消息
    ///
    /// Binance 现货的应用层 PING 格式: {"id": <number>}
    /// 这是一个简单的 JSON 对象，只包含 id 字段（数字）
    fn is_ping(&self, msg: &Value) -> bool {
        // 检测只包含 id 字段（数字）的 JSON 对象
        if let Some(obj) = msg.as_object() {
            // Binance PING 消息只有 1 个字段：id（数字）
            obj.len() == 1 && obj.contains_key("id") && obj["id"].is_number()
        } else {
            false
        }
    }

    /// 检查是否为订阅确认
    fn is_subscription_confirm(&self, msg: &Value) -> bool {
        msg.get("result").map(|r| r.is_null()).unwrap_or(false) && msg.get("id").is_some()
    }

    /// 检查是否为错误消息
    fn is_error(&self, msg: &Value) -> bool {
        msg.get("error").is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_subscribe_ticker() {
        let builder = BinanceSubscriptionBuilder;

        let channels = vec![SubscriptionChannel {
            symbol: "BTC/USDT".to_string(),
            channel_type: ChannelType::Ticker,
            params: HashMap::new(),
            market_type: None,
            is_private: false,
        }];

        let msg = builder.build_subscribe(&channels).unwrap();
        assert_eq!(msg["method"], "SUBSCRIBE");
        assert!(
            msg["params"]
                .as_array()
                .unwrap()
                .contains(&json!("btcusdt@ticker"))
        );
    }

    #[test]
    fn test_build_subscribe_orderbook() {
        let builder = BinanceSubscriptionBuilder;

        let channels = vec![SubscriptionChannel {
            symbol: "BTC/USDT".to_string(),
            channel_type: ChannelType::OrderBook,
            params: HashMap::new(),
            market_type: None,
            is_private: false,
        }];

        let msg = builder.build_subscribe(&channels).unwrap();
        assert_eq!(msg["method"], "SUBSCRIBE");
    }

    #[test]
    fn test_to_binance_symbol() {
        assert_eq!(
            BinanceSubscriptionBuilder::to_binance_symbol("BTC/USDT"),
            "BTCUSDT"
        );
        assert_eq!(
            BinanceSubscriptionBuilder::to_binance_symbol("ETH/USDT:USDT"),
            "ETHUSDT"
        );
    }
}

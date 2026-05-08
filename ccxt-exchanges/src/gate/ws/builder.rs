//! Gate.io WebSocket subscription builder
//!
//! Implements the SubscriptionBuilder trait for Gate.io WebSocket protocol.

use ccxt_core::error::Result;
use ccxt_core::ws::ChannelType;
use ccxt_core::ws::subscription::{SubscriptionBuilder, SubscriptionChannel};
use ccxt_core::{Error, SubscriptionInfo};
use serde_json::{Value, json};

/// Gate.io WebSocket subscription builder
#[derive(Clone)]
pub struct GateSubscriptionBuilder;

impl SubscriptionBuilder for GateSubscriptionBuilder {
    fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let mut subscriptions = Vec::new();

        for channel in channels {
            let subscription = match channel.channel_type {
                ccxt_core::ws::subscription::ChannelType::Ticker => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    // Contract: BTC/USDT:USDT -> contains ':'
                    // Spot: BTC/USDT -> no ':'
                    if symbol.contains(':') {
                        // Futures contract ticker
                        // Contract symbol format: BTC_USDT (remove /QUOTE:SETTLE part)
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");

                        json!({
                            "channel": "futures.tickers",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        // Spot ticker
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.tickers",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::Trades => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures contract trades
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");
                        json!({
                            "channel": "futures.trades",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        // Spot trades
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.trades",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::OrderBook => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures contract orderbook
                        // 期货 OrderBook payload 只需要合约名称，不需要 depth 和 speed
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");
                        json!({
                            "channel": "futures.order_book",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        // Spot orderbook - use spot.order_book (full snapshot)
                        // NOT spot.order_book_update (requires initial snapshot sync)
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.order_book",
                            "event": "subscribe",
                            "payload": [gate_symbol, "5", "100ms"]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::BidsAsks => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures contract book_ticker
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");
                        json!({
                            "channel": "futures.book_ticker",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        // Spot book_ticker
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.book_ticker",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::Kline => {
                    let symbol = channel.symbol.as_str();
                    let interval = channel
                        .params
                        .get("interval")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1m");

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures contract candlesticks
                        // Official format: payload: ["interval", "contract"]
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");
                        let sub_msg = json!({
                            "channel": "futures.candlesticks",
                            "event": "subscribe",
                            "payload": [interval, gate_symbol]
                        });
                        sub_msg
                    } else {
                        // Spot candlesticks
                        // Official format: payload: ["interval", "symbol"]
                        // Example: ["1m", "BTC_USDT"]
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.candlesticks",
                            "event": "subscribe",
                            "payload": [interval, gate_symbol]
                        })
                    }
                }
                // Private channels
                ccxt_core::ws::subscription::ChannelType::Balance => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures balance
                        json!({
                            "channel": "futures.balances",
                            "event": "subscribe",
                            "payload": []
                        })
                    } else if symbol.is_empty() {
                        // Spot balance (all)
                        json!({
                            "channel": "spot.balances",
                            "event": "subscribe",
                            "payload": []
                        })
                    } else {
                        // Spot balance (specific currency pair)
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.balances",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::Orders => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures orders
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");
                        json!({
                            "channel": "futures.orders",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        // Spot orders
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.orders",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::MyTrades => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures user trades
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");
                        json!({
                            "channel": "futures.usertrades",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        // Spot user trades
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.usertrades",
                            "event": "subscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                _ => {
                    return Err(Error::not_implemented(format!(
                        "Channel type {:?} not supported",
                        channel.channel_type
                    )));
                }
            };
            subscriptions.push(subscription);
        }

        // Gate.io API requires "time" in every message (Unix timestamp in seconds)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Gate.io supports multiple subscriptions in one message
        if subscriptions.len() == 1 {
            let mut msg = subscriptions.remove(0);
            if let Some(obj) = msg.as_object_mut() {
                obj.insert("time".to_string(), serde_json::json!(now));
            }
            Ok(msg)
        } else {
            let arr: Vec<Value> = subscriptions
                .into_iter()
                .map(|mut msg| {
                    if let Some(obj) = msg.as_object_mut() {
                        obj.insert("time".to_string(), serde_json::json!(now));
                    }
                    msg
                })
                .collect();
            Ok(Value::Array(arr))
        }
    }

    fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let mut subscriptions = Vec::new();

        for channel in channels {
            let subscription = match channel.channel_type {
                ccxt_core::ws::subscription::ChannelType::Ticker => {
                    let symbol = channel.symbol.as_str();
                    let gate_symbol = symbol.replace('/', "_");

                    if symbol.contains(':') {
                        json!({
                            "channel": "futures.tickers",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        json!({
                            "channel": "spot.tickers",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::Trades => {
                    let symbol = channel.symbol.as_str();
                    let gate_symbol = symbol.replace('/', "_");

                    if symbol.contains(':') {
                        json!({
                            "channel": "futures.trades",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        json!({
                            "channel": "spot.trades",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::OrderBook => {
                    let symbol = channel.symbol.as_str();
                    let gate_symbol = symbol.replace('/', "_");

                    if symbol.contains(':') {
                        json!({
                            "channel": "futures.order_book",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        json!({
                            "channel": "spot.order_book",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::BidsAsks => {
                    let symbol = channel.symbol.as_str();
                    let gate_symbol = symbol.replace('/', "_");

                    if symbol.contains(':') {
                        json!({
                            "channel": "futures.book_ticker",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        json!({
                            "channel": "spot.book_ticker",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::Kline => {
                    let symbol = channel.symbol.as_str();
                    let gate_symbol = symbol.replace('/', "_");
                    let interval = channel
                        .params
                        .get("interval")
                        .and_then(|v| v.as_str())
                        .unwrap_or("1m");

                    if symbol.contains(':') {
                        json!({
                            "channel": "futures.candlesticks",
                            "event": "unsubscribe",
                            "payload": [interval, gate_symbol]
                        })
                    } else {
                        json!({
                            "channel": "spot.candlesticks",
                            "event": "unsubscribe",
                            "payload": [interval, gate_symbol]
                        })
                    }
                }
                // Private channels
                ccxt_core::ws::subscription::ChannelType::Balance => {
                    let symbol = channel.symbol.as_str();
                    if symbol.is_empty() {
                        json!({
                            "channel": "spot.balances",
                            "event": "unsubscribe",
                            "payload": []
                        })
                    } else {
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.balances",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::Orders => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures orders
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");
                        json!({
                            "channel": "futures.orders",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        // Spot orders
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.orders",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                ccxt_core::ws::subscription::ChannelType::MyTrades => {
                    let symbol = channel.symbol.as_str();

                    // Detect market type from symbol format
                    if symbol.contains(':') {
                        // Futures user trades
                        let gate_symbol =
                            symbol.split(':').next().unwrap_or(symbol).replace('/', "_");
                        json!({
                            "channel": "futures.usertrades",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    } else {
                        // Spot user trades
                        let gate_symbol = symbol.replace('/', "_");
                        json!({
                            "channel": "spot.usertrades",
                            "event": "unsubscribe",
                            "payload": [gate_symbol]
                        })
                    }
                }
                _ => {
                    return Err(Error::not_implemented(format!(
                        "Channel type {:?} not supported",
                        channel.channel_type
                    )));
                }
            };
            subscriptions.push(subscription);
        }

        if subscriptions.len() == 1 {
            let mut msg = subscriptions.remove(0);
            if let Some(obj) = msg.as_object_mut() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                obj.insert("time".to_string(), serde_json::json!(now));
            }
            Ok(msg)
        } else {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let arr: Vec<Value> = subscriptions
                .into_iter()
                .map(|mut msg| {
                    if let Some(obj) = msg.as_object_mut() {
                        obj.insert("time".to_string(), serde_json::json!(now));
                    }
                    msg
                })
                .collect();
            Ok(Value::Array(arr))
        }
    }

    fn extract_channel(&self, msg: &Value) -> Option<String> {
        let channel = msg["channel"].as_str()?;
        let result = &msg["result"];

        // Extract symbol from result
        let symbol = if let Some(currency_pair) = result["currency_pair"].as_str() {
            // Spot: BTC_USDT -> BTC/USDT
            currency_pair.replace('_', "/")
        } else if let Some(contract) = result["contract"].as_str() {
            // Futures: BTC_USDT -> BTC/USDT:USDT (need to detect from channel)
            let base_quote = contract.replace('_', "/");
            // For futures channels, append :USDT (default settle)
            if channel.starts_with("futures.") {
                format!("{}:USDT", base_quote)
            } else {
                base_quote
            }
        } else if let Some(s) = result["s"].as_str() {
            // OrderBook channel: "BTC_USDT" (spot) or "BTC_USDT" (futures)
            let symbol_str = s.replace('_', "/");
            if channel.starts_with("futures.") {
                format!("{}:USDT", symbol_str)
            } else {
                symbol_str
            }
        } else if let Some(n) = result["n"].as_str() {
            // Candlesticks channel: "1m_BTC_USDT" or "1s_BTC_USDT"
            // Extract symbol after the interval prefix
            if let Some(underscore_pos) = n.find('_') {
                let symbol_part = &n[underscore_pos + 1..];
                let symbol_str = symbol_part.replace('_', "/");
                if channel.starts_with("futures.") {
                    format!("{}:USDT", symbol_str)
                } else {
                    symbol_str
                }
            } else {
                n.to_string()
            }
        } else if let Some(arr) = result.as_array() {
            // Result is an array (e.g., futures tickers, orders, trades)
            if let Some(first) = arr.first() {
                if let Some(contract) = first["contract"].as_str() {
                    let base_quote = contract.replace('_', "/");
                    if channel.starts_with("futures.") {
                        format!("{}:USDT", base_quote)
                    } else {
                        base_quote
                    }
                } else if let Some(currency_pair) = first["currency_pair"].as_str() {
                    currency_pair.replace('_', "/")
                } else if let Some(order) = first.get("order") {
                    // Private order channel
                    if let Some(contract) = order["contract"].as_str() {
                        let base_quote = contract.replace('_', "/");
                        format!("{}:USDT", base_quote)
                    } else if let Some(currency_pair) = order["currency_pair"].as_str() {
                        currency_pair.replace('_', "/")
                    } else {
                        return Some(channel.to_string());
                    }
                } else if let Some(trade) = first.get("trade") {
                    // Private trade channel
                    if let Some(contract) = trade["contract"].as_str() {
                        let base_quote = contract.replace('_', "/");
                        format!("{}:USDT", base_quote)
                    } else if let Some(currency_pair) = trade["currency_pair"].as_str() {
                        currency_pair.replace('_', "/")
                    } else {
                        return Some(channel.to_string());
                    }
                } else if let Some(n) = first["n"].as_str() {
                    // Candlesticks array format (futures): "1s_BTC_USDT"
                    if let Some(underscore_pos) = n.find('_') {
                        let symbol_part = &n[underscore_pos + 1..];
                        let symbol_str = symbol_part.replace('_', "/");
                        if channel.starts_with("futures.") {
                            format!("{}:USDT", symbol_str)
                        } else {
                            symbol_str
                        }
                    } else {
                        n.to_string()
                    }
                } else {
                    return Some(channel.to_string());
                }
            } else {
                return Some(channel.to_string());
            }
        } else {
            return Some(channel.to_string());
        };

        Some(format!("{}:{}", channel, symbol))
    }

    fn extract_channel_from_subscription(
        &self,
        channel_type: &ccxt_core::ws::subscription::ChannelType,
        symbol: &str,
        _params: &std::collections::HashMap<String, Value>,
    ) -> String {
        let gate_channel: String = match channel_type {
            ccxt_core::ws::subscription::ChannelType::Ticker => {
                if symbol.contains(':') {
                    "futures.tickers".to_string()
                } else {
                    "spot.tickers".to_string()
                }
            }
            ccxt_core::ws::subscription::ChannelType::Trades => {
                if symbol.contains(':') {
                    "futures.trades".to_string()
                } else {
                    "spot.trades".to_string()
                }
            }
            ccxt_core::ws::subscription::ChannelType::OrderBook => {
                // Gate 服务器返回的 channel 不包含 level 参数，只是 "spot.order_book"
                if symbol.contains(':') {
                    "futures.order_book".to_string()
                } else {
                    "spot.order_book".to_string()
                }
            }
            ccxt_core::ws::subscription::ChannelType::BidsAsks => {
                if symbol.contains(':') {
                    "futures.book_ticker".to_string()
                } else {
                    "spot.book_ticker".to_string()
                }
            }
            ccxt_core::ws::subscription::ChannelType::Kline => {
                // Gate 服务器返回的 channel 不包含 interval 参数，只是 "spot.candlesticks"
                if symbol.contains(':') {
                    "futures.candlesticks".to_string()
                } else {
                    "spot.candlesticks".to_string()
                }
            }
            _ => return channel_type.to_string(),
        };

        if symbol.is_empty() {
            gate_channel.to_string()
        } else {
            format!("{}:{}", gate_channel, symbol)
        }
    }

    fn rebuild_subscription_channel(
        &self,
        info: &SubscriptionInfo,
    ) -> ccxt_core::error::Result<SubscriptionChannel> {
        // Gate 格式：info.channel 是 "spot.order_book.L5:BTC_USDT" 或 "spot.candlesticks.I5m:BTC_USDT"
        // 需要从 channel 中提取频道类型和参数

        let (channel_type, params) =
            if info.channel.contains(":ticker") || info.channel.contains(".tickers") {
                (ChannelType::Ticker, info.params.clone())
            } else if info.channel.contains(":trades") || info.channel.contains(".trades") {
                (ChannelType::Trades, info.params.clone())
            } else if info.channel.contains(":orderbook") || info.channel.contains(".order_book") {
                // 提取 level 参数
                let mut params = info.params.clone();

                // 从 channel 中提取 level: "spot.order_book.L5" 或 "futures.order_book.L20"
                if let Some(l_pos) = info.channel.find(".L") {
                    let after_l = &info.channel[l_pos + 2..]; // "5:BTC_USDT"
                    if let Some(colon_pos) = after_l.find(':') {
                        let level_str = &after_l[..colon_pos]; // "5"
                        if let Ok(level) = level_str.parse::<u64>() {
                            params.insert("level".to_string(), serde_json::json!(level));
                        }
                    }
                }

                (ChannelType::OrderBook, params)
            } else if info.channel.contains(":candlesticks")
                || info.channel.contains(".kline")
                || info.channel.contains(".candlesticks")
            {
                // 提取 interval 参数
                let mut params = info.params.clone();

                // 从 channel 中提取 interval: "spot.candlesticks.I5m" 或 "futures.candlesticks.I1h"
                if let Some(i_pos) = info.channel.find(".I") {
                    let after_i = &info.channel[i_pos + 2..]; // "5m:BTC_USDT"
                    if let Some(colon_pos) = after_i.find(':') {
                        let interval = &after_i[..colon_pos].to_string(); // "5m"
                        params.insert("interval".to_string(), serde_json::json!(interval));
                    }
                }

                (ChannelType::Kline, params)
            } else if info.channel.contains(".balances") || info.channel.contains(":balances") {
                // 私有频道：余额
                (ChannelType::Balance, info.params.clone())
            } else if info.channel.contains(".orders") || info.channel.contains(":orders") {
                // 私有频道：订单
                (ChannelType::Orders, info.params.clone())
            } else if info.channel.contains(".usertrades") || info.channel.contains(":usertrades") {
                // 私有频道：成交
                (ChannelType::MyTrades, info.params.clone())
            } else {
                tracing::warn!(
                    channel = %info.channel,
                    "Unknown Gate channel type, defaulting to Ticker"
                );
                (ChannelType::Ticker, info.params.clone())
            };

        // 从 symbol 或 channel 中提取交易对
        let symbol = info.symbol.clone().unwrap_or_default();

        // 判断市场类型
        let market_type =
            if info.channel.starts_with("futures:") || info.channel.starts_with("swap:") {
                Some(ccxt_core::ws::subscription::MarketType::Swap)
            } else {
                Some(ccxt_core::ws::subscription::MarketType::Spot)
            };

        // 判断是否为私有频道
        let is_private = matches!(
            channel_type,
            ChannelType::Balance
                | ChannelType::Orders
                | ChannelType::MyTrades
                | ChannelType::Positions
        );

        Ok(SubscriptionChannel {
            channel_type,
            symbol,
            params,
            market_type,
            is_private,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ticker_subscribe() {
        let builder = GateSubscriptionBuilder;
        let channel = SubscriptionChannel::ticker("BTC/USDT");
        let channels = vec![channel];
        let msg = builder.build_subscribe(&channels).unwrap();

        assert_eq!(msg["channel"], "spot.tickers");
        assert_eq!(msg["event"], "subscribe");
        assert_eq!(msg["payload"][0], "BTC_USDT");
    }

    #[test]
    fn test_build_trade_subscribe() {
        let builder = GateSubscriptionBuilder;
        let channel = SubscriptionChannel::trades("BTC/USDT");
        let channels = vec![channel];
        let msg = builder.build_subscribe(&channels).unwrap();

        assert_eq!(msg["channel"], "spot.trades");
        assert_eq!(msg["event"], "subscribe");
    }

    #[test]
    fn test_extract_channel() {
        let builder = GateSubscriptionBuilder;
        let message = serde_json::json!({
            "time": 1606292218,
            "channel": "spot.tickers",
            "event": "update",
            "result": {
                "currency_pair": "BTC_USDT"
            }
        });

        let channel = builder.extract_channel(&message);
        assert!(channel.is_some());
        assert_eq!(channel.unwrap(), "spot.tickers:BTC/USDT");
    }
}

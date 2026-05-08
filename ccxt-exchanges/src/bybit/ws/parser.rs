//! Bybit WebSocket 消息解析器
//!
//! 通过复用 `parser::ws` 模块的解析函数实现 StreamParser trait。

use ccxt_core::error::{Error, Result};
use ccxt_core::types::{Balance, BidAsk, Order, OrderBook, Ticker, Trade};
use ccxt_core::ws::{Parseable, ParsedMessage, StreamParser, StreamParserExt};
use serde_json::Value;

// 复用 parser/ws 模块的解析函数
use crate::bybit::parser::ws::{
    extract_data, parse_balance, parse_bids_asks, parse_execution, parse_ohlcv, parse_order,
    parse_order_from_data, parse_orderbook, parse_ticker, parse_trades,
};

/// Bybit WebSocket 消息解析器
#[derive(Debug, Clone, Copy)]
pub struct BybitStreamParser;

impl StreamParser for BybitStreamParser {
    /// 解析消息
    fn parse(&self, msg: &Value) -> Result<ParsedMessage> {
        // 检查是否为心跳
        if msg.get("op").and_then(|o| o.as_str()) == Some("pong") {
            return Ok(ParsedMessage::Heartbeat);
        }

        // 检查是否为操作响应
        if let Some(op) = msg.get("op").and_then(|o| o.as_str()) {
            return match op {
                "auth" => {
                    let success = msg
                        .get("success")
                        .and_then(|s| s.as_bool())
                        .unwrap_or(false);
                    if success {
                        Ok(ParsedMessage::AuthSuccess)
                    } else {
                        let message = msg
                            .get("ret_msg")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Authentication failed");
                        Ok(ParsedMessage::Error {
                            code: None,
                            message: message.to_string(),
                        })
                    }
                }
                "subscribe" | "unsubscribe" => {
                    let success = msg
                        .get("success")
                        .and_then(|s| s.as_bool())
                        .unwrap_or(false);
                    if success {
                        let topic = msg
                            .get("request")
                            .and_then(|r| r.get("args"))
                            .and_then(|a| a.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        Ok(ParsedMessage::SubscriptionConfirm {
                            channel: topic.to_string(),
                        })
                    } else {
                        let message = msg
                            .get("ret_msg")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Subscription failed");
                        Ok(ParsedMessage::Error {
                            code: None,
                            message: message.to_string(),
                        })
                    }
                }
                _ => Ok(ParsedMessage::Unknown(msg.clone())),
            };
        }

        // 检查是否有 topic 和 data (数据消息)
        if let Some(topic) = msg.get("topic").and_then(|t| t.as_str()) {
            return if topic.starts_with("tickers") {
                let ticker = parse_ticker(msg)?;
                Ok(ParsedMessage::Ticker(ticker))
            } else if topic.starts_with("orderbook.1") {
                // orderbook.1 是最优买卖价 (bids_asks)
                let ba = parse_bids_asks(msg)?;
                Ok(ParsedMessage::BidsAsks(ba))
            } else if topic.starts_with("orderbook") {
                let ob = parse_orderbook(msg)?;
                Ok(ParsedMessage::OrderBook(ob))
            } else if topic.starts_with("publicTrade") {
                let trades = parse_trades(msg)?;
                Ok(ParsedMessage::Trades(trades))
            } else if topic.starts_with("kline") {
                let ohlcv = parse_ohlcv(msg)?;
                Ok(ParsedMessage::Ohlcv(ohlcv))
            } else if topic == "wallet" {
                let balance = parse_balance(msg)?;
                Ok(ParsedMessage::Balance(balance))
            } else if topic == "order" {
                let data = extract_data(msg)?;
                if let Some(first) = data.as_array().and_then(|arr| arr.first()) {
                    let order = parse_order_from_data(first)?;
                    Ok(ParsedMessage::Order(order))
                } else {
                    Ok(ParsedMessage::Unknown(msg.clone()))
                }
            } else if topic == "execution" {
                let trades = parse_execution(msg)?;
                Ok(ParsedMessage::Trades(trades))
            } else {
                Ok(ParsedMessage::Unknown(msg.clone()))
            };
        }

        Ok(ParsedMessage::Unknown(msg.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::Symbol;
    use serde_json::json;

    #[test]
    fn test_parse_heartbeat() {
        let parser = BybitStreamParser;

        let msg = json!({"op": "pong"});
        let ws_msg = parser.parse(&msg).unwrap();
        assert!(ws_msg.is_heartbeat());
    }

    #[test]
    fn test_to_unified_symbol() {
        // 使用 parser::ws 模块的函数
        assert_eq!(
            crate::bybit::parser::ws::to_unified_symbol("BTCUSDT", None),
            "BTC/USDT"
        );
        assert_eq!(
            crate::bybit::parser::ws::to_unified_symbol("ETHUSDC", None),
            "ETH/USDC"
        );
    }

    #[test]
    fn test_parse_ticker() {
        let msg = json!({
            "topic": "tickers.BTCUSDT",
            "data": {
                "lastPrice": "50000.00",
                "bid1Price": "49999.00",
                "ask1Price": "50001.00",
                "highPrice24h": "51000.00",
                "lowPrice24h": "49000.00",
                "volume24h": "1000.5",
                "ts": 1700000000000u64
            }
        });

        let ticker = parse_ticker(&msg).unwrap();
        assert_eq!(ticker.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert!(ticker.last.is_some());
    }

    #[test]
    fn test_parse_error() {
        let parser = BybitStreamParser;

        let msg = json!({
            "op": "subscribe",
            "success": false,
            "ret_code": 10001,
            "ret_msg": "Invalid parameter"
        });

        let ws_msg = parser.parse(&msg).unwrap();
        assert!(matches!(ws_msg, ParsedMessage::Error { .. }));
    }
}

// ============================================================================
// StreamParserExt 实现（泛型解析）
// ============================================================================

impl StreamParserExt for BybitStreamParser {
    fn parse_as<T: Parseable>(&self, msg: &Value) -> Result<T> {
        match T::TYPE_NAME {
            "ticker" => {
                let ticker: Ticker = parse_ticker(msg)?;
                // SAFETY: T is guaranteed to be Ticker by the match above.
                // We use transmute_copy + forget to transfer ownership without dropping.
                let result = unsafe { std::mem::transmute_copy(&ticker) };
                std::mem::forget(ticker);
                Ok(result)
            }
            "bid_ask" => {
                let bidask: BidAsk = parse_bids_asks(msg)?;
                let result = unsafe { std::mem::transmute_copy(&bidask) };
                std::mem::forget(bidask);
                Ok(result)
            }
            "orderbook" => {
                let orderbook: OrderBook = parse_orderbook(msg)?;
                let result = unsafe { std::mem::transmute_copy(&orderbook) };
                std::mem::forget(orderbook);
                Ok(result)
            }
            "trade" => {
                let trades: Vec<Trade> = parse_trades(msg)?;
                let result = unsafe { std::mem::transmute_copy(&trades) };
                std::mem::forget(trades);
                Ok(result)
            }
            "ohlcv" => {
                let ohlcv = parse_ohlcv(msg)?;
                let result = unsafe { std::mem::transmute_copy(&ohlcv) };
                std::mem::forget(ohlcv);
                Ok(result)
            }
            "balance" => {
                let balance: Balance = parse_balance(msg)?;
                let result = unsafe { std::mem::transmute_copy(&balance) };
                std::mem::forget(balance);
                Ok(result)
            }
            "order" => {
                let order: Order = parse_order(msg)?;
                let result = unsafe { std::mem::transmute_copy(&order) };
                std::mem::forget(order);
                Ok(result)
            }
            _ => Err(Error::invalid_request(format!(
                "Bybit parser does not support type: {}",
                T::TYPE_NAME
            ))),
        }
    }
}

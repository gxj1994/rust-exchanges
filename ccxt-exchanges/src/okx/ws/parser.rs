//! OKX WebSocket 消息解析器

use ccxt_core::error::Result;
use ccxt_core::types::Trade;
use ccxt_core::ws::{Parseable, ParsedMessage, StreamParser, StreamParserExt};
use serde_json::Value;

// 使用 parser/ws/ 模块中的解析函数
use crate::okx::parser::ws::{
    extract_data, parse_balance, parse_ohlcv, parse_order, parse_order_from_data, parse_orderbook,
    parse_ticker, parse_trades,
};

/// OKX WebSocket 消息解析器
#[derive(Debug, Clone, Copy)]
pub struct OkxStreamParser;

impl StreamParser for OkxStreamParser {
    /// 解析消息
    fn parse(&self, msg: &Value) -> Result<ParsedMessage> {
        // 检查是否为心跳
        if msg.as_str() == Some("ping") || msg.as_str() == Some("pong") {
            return Ok(ParsedMessage::Heartbeat);
        }

        // 检查事件类型
        if let Some(event) = msg.get("event").and_then(|e| e.as_str()) {
            return match event {
                "login" => {
                    let code = msg.get("code").and_then(|c| c.as_str()).unwrap_or("1");
                    if code == "0" {
                        Ok(ParsedMessage::AuthSuccess)
                    } else {
                        let msg_text = msg
                            .get("msg")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Authentication failed");
                        Ok(ParsedMessage::Error {
                            code: code.parse().ok(),
                            message: msg_text.to_string(),
                        })
                    }
                }
                "subscribe" | "unsubscribe" => {
                    let channel = msg
                        .get("arg")
                        .and_then(|a| a.get("channel"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    Ok(ParsedMessage::SubscriptionConfirm { channel })
                }
                "error" => {
                    let code = msg.get("code").and_then(|c| c.as_str());
                    let message = msg
                        .get("msg")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error");
                    Ok(ParsedMessage::Error {
                        code: code.and_then(|c| c.parse().ok()),
                        message: message.to_string(),
                    })
                }
                _ => Ok(ParsedMessage::Unknown(msg.clone())),
            };
        }

        // 检查是否有 arg 和 data (数据消息)
        if let Some(arg) = msg.get("arg") {
            let channel = arg.get("channel").and_then(|c| c.as_str());

            return match channel {
                Some("tickers") => {
                    let ticker = parse_ticker(msg)?;
                    Ok(ParsedMessage::Ticker(ticker))
                }
                Some(ch) if ch.starts_with("books") => {
                    let ob = parse_orderbook(msg)?;
                    Ok(ParsedMessage::OrderBook(ob))
                }
                Some("trades") => {
                    let trades = parse_trades(msg)?;
                    Ok(ParsedMessage::Trades(trades))
                }
                Some(ch) if ch.starts_with("candle") => {
                    let ohlcv = parse_ohlcv(msg)?;
                    Ok(ParsedMessage::Ohlcv(ohlcv))
                }
                Some("account") => {
                    let balance = parse_balance(msg)?;
                    Ok(ParsedMessage::Balance(balance))
                }
                Some("orders") => {
                    let data = extract_data(msg)?;
                    if let Some(first) = data.first() {
                        let order = parse_order_from_data(first)?;
                        Ok(ParsedMessage::Order(order))
                    } else {
                        Ok(ParsedMessage::Unknown(msg.clone()))
                    }
                }
                _ => Ok(ParsedMessage::Unknown(msg.clone())),
            };
        }

        Ok(ParsedMessage::Unknown(msg.clone()))
    }
}

impl StreamParserExt for OkxStreamParser {
    /// 泛型解析方法
    fn parse_as<T: Parseable>(&self, msg: &Value) -> ccxt_core::error::Result<T> {
        use ccxt_core::error::Error;

        match T::TYPE_NAME {
            "ticker" => {
                let ticker = parse_ticker(msg)?;
                // SAFETY: T is guaranteed to be Ticker by the match above.
                // We use transmute_copy + forget to transfer ownership without dropping.
                let result = unsafe { std::mem::transmute_copy(&ticker) };
                std::mem::forget(ticker);
                Ok(result)
            }
            "orderbook" => {
                let orderbook = parse_orderbook(msg)?;
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
                let balance = parse_balance(msg)?;
                let result = unsafe { std::mem::transmute_copy(&balance) };
                std::mem::forget(balance);
                Ok(result)
            }
            "order" => {
                let order = parse_order(msg)?;
                let result = unsafe { std::mem::transmute_copy(&order) };
                std::mem::forget(order);
                Ok(result)
            }
            _ => Err(Error::invalid_request(format!(
                "OKX parser does not support type: {}",
                T::TYPE_NAME
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::Symbol;
    use serde_json::json;

    #[test]
    fn test_parse_ticker() {
        let msg = json!({
            "arg": {"channel": "tickers", "instId": "BTC-USDT"},
            "data": [{
                "instId": "BTC-USDT",
                "last": "50000.00",
                "high24h": "51000.00",
                "low24h": "49000.00",
                "bidPx": "49999.00",
                "askPx": "50001.00",
                "vol24h": "1000.5",
                "ts": "1700000000000"
            }]
        });

        let ticker = parse_ticker(&msg).unwrap();
        assert_eq!(ticker.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert!(ticker.last.is_some());
    }

    #[test]
    fn test_parse_trades() {
        let msg = json!({
            "arg": {"channel": "trades", "instId": "BTC-USDT"},
            "data": [{
                "instId": "BTC-USDT",
                "tradeId": "123456789",
                "px": "50000.00",
                "sz": "0.5",
                "side": "buy",
                "ts": "1700000000000"
            }]
        });

        let trades = parse_trades(&msg).unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].symbol, Symbol::new_unchecked("BTC/USDT"));
    }

    #[test]
    fn test_parse_ohlcv() {
        let msg = json!({
            "arg": {"channel": "candle1H", "instId": "BTC-USDT"},
            "data": [[
                "1700000000000",
                "50000.5",
                "50100.0",
                "49900.0",
                "50050.0",
                "123.456",
                "6000000.0",
                "6000000.0",
                "true"
            ]]
        });

        let ohlcvs = parse_ohlcv(&msg).unwrap();
        assert!(!ohlcvs.is_empty());
        assert_eq!(ohlcvs[0].timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_heartbeat() {
        let parser = OkxStreamParser;

        let msg = json!("ping");
        let ws_msg = parser.parse(&msg).unwrap();
        assert!(ws_msg.is_heartbeat());

        let msg = json!("pong");
        let ws_msg = parser.parse(&msg).unwrap();
        assert!(ws_msg.is_heartbeat());
    }
}

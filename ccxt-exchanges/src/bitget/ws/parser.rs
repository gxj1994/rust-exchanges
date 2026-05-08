//! Bitget WebSocket 消息解析器
//!
//! 通过复用 `parser::ws` 模块的解析函数实现 StreamParser trait。

use ccxt_core::error::{Error, Result};
use ccxt_core::types::{Balance, Order, OrderBook, Ticker, Trade};
use ccxt_core::ws::{Parseable, ParsedMessage, StreamParser, StreamParserExt};
use serde_json::Value;

// 复用 parser/ws 模块的解析函数
use crate::bitget::parser::ws::{
    extract_data, parse_balance, parse_ohlcv, parse_order, parse_order_from_data, parse_orderbook,
    parse_ticker, parse_trades,
};

/// Bitget WebSocket 消息解析器
#[derive(Debug, Clone, Copy)]
pub struct BitgetStreamParser;

impl StreamParser for BitgetStreamParser {
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
                    println!("[BitgetParser] Subscription confirm: {}", channel);
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
            let channel = arg.get("topic").and_then(|c| c.as_str()); // V3: channel → topic

            return match channel {
                Some("ticker") => {
                    let ticker = parse_ticker(msg)?;
                    Ok(ParsedMessage::Ticker(ticker))
                }
                Some(ch) if ch.starts_with("books") => {
                    let ob = parse_orderbook(msg)?;
                    Ok(ParsedMessage::OrderBook(ob))
                }
                Some("publicTrade") => {
                    // V3: trade → publicTrade
                    let trades = parse_trades(msg)?;
                    Ok(ParsedMessage::Trades(trades))
                }
                Some("kline") => {
                    // V3: candle{interval} → kline
                    let ohlcv = parse_ohlcv(msg)?;
                    Ok(ParsedMessage::Ohlcv(ohlcv))
                }
                Some("account") => {
                    let balance = parse_balance(msg)?;
                    Ok(ParsedMessage::Balance(balance))
                }
                Some("order") => {
                    // V3: orders → order
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

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::Symbol;
    use serde_json::json;

    #[test]
    fn test_parse_heartbeat() {
        let parser = BitgetStreamParser;

        let msg = json!("ping");
        let ws_msg = parser.parse(&msg).unwrap();
        assert!(ws_msg.is_heartbeat());

        let msg = json!("pong");
        let ws_msg = parser.parse(&msg).unwrap();
        assert!(ws_msg.is_heartbeat());
    }

    #[test]
    fn test_parse_error() {
        let parser = BitgetStreamParser;

        let msg = json!({
            "event": "error",
            "code": "50001",
            "msg": "Invalid parameter"
        });

        let ws_msg = parser.parse(&msg).unwrap();
        assert!(matches!(ws_msg, ParsedMessage::Error { .. }));
    }

    #[test]
    fn test_to_unified_symbol() {
        // 使用 parser::ws 模块的函数
        assert_eq!(
            crate::bitget::parser::ws::to_unified_symbol("BTCUSDT", None),
            "BTC/USDT"
        );
        assert_eq!(
            crate::bitget::parser::ws::to_unified_symbol("ETHUSDC", None),
            "ETH/USDC"
        );
    }

    #[test]
    fn test_parse_ticker() {
        // V3 UTA格式：symbol在arg中，data中只有价格数据
        let msg = json!({
            "arg": {"topic": "ticker", "symbol": "BTCUSDT"},
            "data": [{
                "lastPrice": "50000.00",
                "bid1Price": "49999.00",
                "ask1Price": "50001.00",
                "high24h": "51000.00",
                "low24h": "49000.00",
                "volume24h": "1000.5",
                "ts": "1700000000000"
            }]
        });

        let ticker = parse_ticker(&msg).unwrap();
        assert_eq!(ticker.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert!(ticker.last.is_some());
    }
}

// ============================================================================
// StreamParserExt 实现（泛型解析）
// ============================================================================

impl StreamParserExt for BitgetStreamParser {
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
                "Bitget parser does not support type: {}",
                T::TYPE_NAME
            ))),
        }
    }
}

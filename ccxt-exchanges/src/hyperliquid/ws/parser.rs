//! Hyperliquid WebSocket 消息解析器
//!
//! 将 WebSocket 消息解析为标准化的 CCXT 格式结构。

use ccxt_core::error::{Error, Result};
use ccxt_core::types::BidAsk;
use ccxt_core::ws::{Parseable, ParsedMessage, StreamParser, StreamParserExt};
use serde_json::Value;

// 使用 parser/ws/ 模块中的解析函数
use crate::hyperliquid::parser::ws::{
    parse_all_mids, parse_balance as parse_ws_balance, parse_bids_asks as parse_ws_bids_asks,
    parse_ohlcv as parse_ws_ohlcv, parse_order_from_data, parse_order_update,
    parse_orderbook as parse_ws_orderbook, parse_trades, parse_user_fills,
};

/// Hyperliquid WebSocket 消息解析器
#[derive(Debug, Clone, Copy)]
pub struct HyperliquidStreamParser;

impl HyperliquidStreamParser {
    /// 从消息中提取数据
    fn extract_data(msg: &Value) -> Result<&Value> {
        msg.get("data")
            .ok_or_else(|| Error::invalid_request("Missing data in message"))
    }
}

impl StreamParser for HyperliquidStreamParser {
    /// 解析消息
    fn parse(&self, msg: &Value) -> Result<ParsedMessage> {
        // 检查是否为心跳
        if msg.get("method").and_then(|m| m.as_str()) == Some("pong") {
            return Ok(ParsedMessage::Heartbeat);
        }

        // 检查频道类型
        let channel = msg.get("channel").and_then(|c| c.as_str());

        match channel {
            Some("error") => {
                let message = msg
                    .get("data")
                    .and_then(|d| d.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                Ok(ParsedMessage::Error {
                    code: None,
                    message: message.to_string(),
                })
            }
            Some("subscriptionResponse") => {
                let sub_channel = msg
                    .get("data")
                    .and_then(|d| d.get("channel")?.as_str())
                    .unwrap_or("");
                Ok(ParsedMessage::SubscriptionConfirm {
                    channel: sub_channel.to_string(),
                })
            }
            Some("allMids") => {
                let data = Self::extract_data(msg)?;
                let ticker = parse_all_mids(data)?;
                Ok(ParsedMessage::Ticker(ticker))
            }
            Some("l2Book") => {
                let data = Self::extract_data(msg)?;
                let ob = parse_ws_orderbook(data)?;
                Ok(ParsedMessage::OrderBook(ob))
            }
            Some("trades") => {
                let data = Self::extract_data(msg)?;
                let trades = parse_trades(data)?;
                Ok(ParsedMessage::Trades(trades))
            }
            Some("userEvents") => {
                // 用户事件，解析余额和订单
                let data = Self::extract_data(msg);
                if let Ok(data) = data {
                    // 检查是否有账户更新
                    if data.get("balances").is_some() {
                        let balance = parse_ws_balance(data)?;
                        return Ok(ParsedMessage::Balance(balance));
                    }
                    if data.get("orders").is_some() {
                        if let Some(orders) = data.get("orders").and_then(|o| o.as_array()) {
                            if let Some(first) = orders.first() {
                                let order = parse_order_from_data(first)?;
                                return Ok(ParsedMessage::Order(order));
                            }
                        }
                    }
                }
                Ok(ParsedMessage::Unknown(msg.clone()))
            }
            Some("userFills") => {
                // 用户成交
                let data = Self::extract_data(msg)?;
                let trades = parse_user_fills(data)?;
                Ok(ParsedMessage::Trades(trades))
            }
            Some("orderUpdates") => {
                // 订单更新
                let data = Self::extract_data(msg)?;
                let order = parse_order_update(data)?;
                Ok(ParsedMessage::Order(order))
            }
            Some("candle") => {
                // K线数据
                let data = Self::extract_data(msg)?;
                let ohlcv = parse_ws_ohlcv(data)?;
                Ok(ParsedMessage::Ohlcv(ohlcv))
            }
            Some("bbo") => {
                // Best Bid/Offer (book_ticker) 数据
                let bidask = parse_ws_bids_asks(msg)?;
                Ok(ParsedMessage::BidsAsks(bidask))
            }
            _ => Ok(ParsedMessage::Unknown(msg.clone())),
        }
    }
}

// ============================================================================
// 新设计：StreamParserExt 实现（泛型解析）
// ============================================================================

impl StreamParserExt for HyperliquidStreamParser {
    /// 泛型解析方法
    fn parse_as<T: Parseable>(&self, msg: &Value) -> ccxt_core::error::Result<T> {
        use ccxt_core::error::Error;

        match T::TYPE_NAME {
            "ticker" => {
                let data = Self::extract_data(msg)?;
                let ticker = parse_all_mids(data)?;
                // SAFETY: T is guaranteed to be Ticker by the match above.
                // We use transmute_copy + forget to transfer ownership without dropping.
                let result = unsafe { std::mem::transmute_copy(&ticker) };
                std::mem::forget(ticker);
                Ok(result)
            }
            "orderbook" => {
                let data = Self::extract_data(msg)?;
                let orderbook = parse_ws_orderbook(data)?;
                let result = unsafe { std::mem::transmute_copy(&orderbook) };
                std::mem::forget(orderbook);
                Ok(result)
            }
            "trade" => {
                let data = Self::extract_data(msg)?;
                let trades = parse_trades(data)?;
                let result = unsafe { std::mem::transmute_copy(&trades) };
                std::mem::forget(trades);
                Ok(result)
            }
            "ohlcv" => {
                let data = Self::extract_data(msg)?;
                let ohlcv = parse_ws_ohlcv(data)?;
                let result = unsafe { std::mem::transmute_copy(&ohlcv) };
                std::mem::forget(ohlcv);
                Ok(result)
            }
            "bid_ask" => {
                let bidask: BidAsk = parse_ws_bids_asks(msg)?;
                let result = unsafe { std::mem::transmute_copy(&bidask) };
                std::mem::forget(bidask);
                Ok(result)
            }
            "balance" => {
                let data = Self::extract_data(msg)?;
                let balance = parse_ws_balance(data)?;
                let result = unsafe { std::mem::transmute_copy(&balance) };
                std::mem::forget(balance);
                Ok(result)
            }
            "order" => {
                let data = Self::extract_data(msg)?;
                let order = parse_order_from_data(data)?;
                let result = unsafe { std::mem::transmute_copy(&order) };
                std::mem::forget(order);
                Ok(result)
            }
            _ => Err(Error::invalid_request(format!(
                "Hyperliquid parser does not support type: {}",
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
    fn test_parse_heartbeat() {
        let parser = HyperliquidStreamParser;

        let msg = json!({"method": "pong"});
        let ws_msg = parser.parse(&msg).unwrap();
        assert!(ws_msg.is_heartbeat());
    }

    #[test]
    fn test_parse_error() {
        let parser = HyperliquidStreamParser;

        let msg = json!({
            "channel": "error",
            "data": {"message": "Invalid subscription"}
        });

        let ws_msg = parser.parse(&msg).unwrap();
        assert!(matches!(ws_msg, ParsedMessage::Error { .. }));
    }

    #[test]
    fn test_parse_orderbook() {
        let parser = HyperliquidStreamParser;

        let msg = json!({
            "channel": "l2Book",
            "data": {
                "coin": "BTC",
                "time": 1700000000000i64,
                "levels": [
                    [{"px": "50000.0", "sz": "1.5"}, {"px": "49999.0", "sz": "2.0"}],
                    [{"px": "50001.0", "sz": "1.0"}, {"px": "50002.0", "sz": "0.5"}]
                ]
            }
        });

        let ws_msg = parser.parse(&msg).unwrap();
        if let ParsedMessage::OrderBook(ob) = ws_msg {
            // HyperLiquid永续合约使用BTC/USDC:USDC格式
            assert_eq!(ob.symbol, Symbol::new_unchecked("BTC/USDC:USDC"));
        } else {
            panic!("Expected OrderBook");
        }
    }

    #[test]
    fn test_parse_trades_legacy_format() {
        let parser = HyperliquidStreamParser;

        // Legacy format: data.trades is nested array
        let msg = json!({
            "channel": "trades",
            "data": {
                "coin": "BTC",
                "trades": [
                    {
                        "px": "50000.0",
                        "sz": "0.5",
                        "side": "B",
                        "time": 1700000000000i64
                    }
                ]
            }
        });

        let ws_msg = parser.parse(&msg).unwrap();
        if let ParsedMessage::Trades(trades) = ws_msg {
            assert_eq!(trades.len(), 1);
            assert_eq!(trades[0].symbol, Symbol::new_unchecked("BTC/USDC:USDC"));
        } else {
            panic!("Expected Trades");
        }
    }

    #[test]
    fn test_parse_trades_direct_array() {
        let parser = HyperliquidStreamParser;

        // Actual Hyperliquid format: data is direct WsTrade[] array
        let msg = json!({
            "channel": "trades",
            "data": [
                {
                    "coin": "ETH",
                    "side": "A",
                    "px": "2000.5",
                    "sz": "1.5",
                    "hash": "0xabc",
                    "tid": 12345,
                    "time": 1700000000000i64
                }
            ]
        });

        let ws_msg = parser.parse(&msg).unwrap();
        if let ParsedMessage::Trades(trades) = ws_msg {
            assert_eq!(trades.len(), 1);
            assert_eq!(trades[0].symbol, Symbol::new_unchecked("ETH/USDC:USDC"));
        } else {
            panic!("Expected Trades");
        }
    }
}

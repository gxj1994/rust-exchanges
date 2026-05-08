//! Binance WebSocket v2 消息解析器
//!
//! 实现 ccxt_core::ws::StreamParser trait，用于解析 Binance WebSocket 消息。

use ccxt_core::error::{Error, Result};
use ccxt_core::types::{OrderBookDelta, Symbol};
use ccxt_core::ws::{
    OrderBookDeltaParser, Parseable, ParsedMessage, StreamParser, StreamParserExt,
};
use rust_decimal::Decimal;
use serde_json::Value;

// 使用 parser/ws/ 模块中的解析函数
use crate::binance::parser::ws::{
    parse_account_trade, parse_account_trade_futures, parse_balance, parse_balance_futures,
    parse_bids_asks, parse_mark_price, parse_ohlcv, parse_order, parse_order_futures,
    parse_orderbook, parse_orderbook_side_ws, parse_positions, parse_ticker, parse_trades,
    to_unified_symbol,
};

/// Binance WebSocket 消息解析器
#[derive(Debug, Clone, Copy)]
pub struct BinanceStreamParser;

impl BinanceStreamParser {
    /// 解析十进制值
    fn parse_decimal(data: &Value, field: &str) -> Option<Decimal> {
        data.get(field)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    }
}

impl StreamParser for BinanceStreamParser {
    /// 解析消息
    fn parse(&self, msg: &Value) -> Result<ParsedMessage> {
        // 检查是否为心跳 (Binance 使用 ping/pong 帧，但有时也有 result 消息)
        if msg.get("result").is_some() && msg.get("id").is_some() {
            // 这是订阅响应
            let result = msg.get("result").and_then(|r| r.as_null()).is_some();
            if result {
                return Ok(ParsedMessage::SubscriptionConfirm {
                    channel: msg
                        .get("id")
                        .and_then(|i| i.as_u64())
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                });
            }
        }

        // 检查错误响应
        if let Some(error) = msg.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).map(|c| c as i32);
            let message = error
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            return Ok(ParsedMessage::Error { code, message });
        }

        // 检查事件类型
        let event = msg.get("e").and_then(|e| e.as_str());

        match event {
            // 24hr Ticker
            Some("24hrTicker") | Some("24hrMiniTicker") => {
                let ticker = parse_ticker(msg)?;
                Ok(ParsedMessage::Ticker(ticker))
            }
            // 深度更新
            Some("depthUpdate") | Some("depth") => {
                let ob = parse_orderbook(msg)?;
                Ok(ParsedMessage::OrderBook(ob))
            }
            // 成交
            Some("trade") | Some("aggTrade") => {
                let trades = parse_trades(msg)?;
                Ok(ParsedMessage::Trades(trades))
            }
            // K线
            Some("kline") => {
                let ohlcv = parse_ohlcv(msg)?;
                Ok(ParsedMessage::Ohlcv(ohlcv))
            }
            // 标记价格
            Some("markPriceUpdate") => {
                let mp = parse_mark_price(msg)?;
                Ok(ParsedMessage::MarkPrice(mp))
            }
            // 书籍行情 (Book Ticker) - 最优买卖价
            Some("bookTicker") => {
                let ba = parse_bids_asks(msg)?;
                Ok(ParsedMessage::BidsAsks(ba))
            }
            // 账户更新
            Some("outboundAccountPosition") | Some("balanceUpdate") => {
                let balance = parse_balance(msg)?;
                Ok(ParsedMessage::Balance(balance))
            }
            // 订单更新（现货）
            Some("executionReport") => {
                // 检查是否是交易成交事件
                let filled_qty = Self::parse_decimal(msg, "l").unwrap_or_default();
                if filled_qty > Decimal::ZERO {
                    let trade = parse_account_trade(msg)?;
                    Ok(ParsedMessage::Trades(vec![trade]))
                } else {
                    let order = parse_order(msg)?;
                    Ok(ParsedMessage::Order(order))
                }
            }
            // 期货订单/交易更新
            Some("ORDER_TRADE_UPDATE") => {
                if let Some(order_data) = msg.get("o") {
                    let exec_type = order_data.get("x").and_then(|x| x.as_str()).unwrap_or("");
                    if exec_type == "TRADE" {
                        let trade = parse_account_trade_futures(msg)?;
                        Ok(ParsedMessage::Trades(vec![trade]))
                    } else {
                        let order = parse_order_futures(msg)?;
                        Ok(ParsedMessage::Order(order))
                    }
                } else {
                    Ok(ParsedMessage::Unknown(msg.clone()))
                }
            }
            // 期货账户更新
            Some("ACCOUNT_UPDATE") => {
                if let Some(account_data) = msg.get("a") {
                    if account_data.get("P").and_then(|p| p.as_array()).is_some() {
                        let positions = parse_positions(msg)?;
                        Ok(ParsedMessage::Positions(positions))
                    } else {
                        let balance = parse_balance_futures(msg)?;
                        Ok(ParsedMessage::Balance(balance))
                    }
                } else {
                    let balance = parse_balance_futures(msg)?;
                    Ok(ParsedMessage::Balance(balance))
                }
            }
            // listenKey 过期
            Some("listenKeyExpired") => Ok(ParsedMessage::Error {
                code: Some(-1),
                message: "listenKey expired".to_string(),
            }),
            _ => Ok(ParsedMessage::Unknown(msg.clone())),
        }
    }
}

/// Binance OrderBook 增量解析器实现
impl OrderBookDeltaParser for BinanceStreamParser {
    fn parse_delta(&self, msg: &Value, symbol: &Symbol) -> Result<OrderBookDelta> {
        let first_update_id = msg
            .get("U")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| Error::invalid_request("Missing U (first_update_id)"))?;

        let final_update_id = msg
            .get("u")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| Error::invalid_request("Missing u (final_update_id)"))?;

        let prev_final_update_id = msg.get("pu").and_then(|v| v.as_i64());

        let timestamp = msg
            .get("E")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        let bids = parse_orderbook_side_ws(msg.get("b").unwrap_or(&Value::Null));
        let asks = parse_orderbook_side_ws(msg.get("a").unwrap_or(&Value::Null));

        Ok(OrderBookDelta {
            symbol: symbol.clone(),
            first_update_id,
            final_update_id,
            prev_final_update_id,
            timestamp,
            bids,
            asks,
        })
    }

    fn is_delta(&self, msg: &Value) -> bool {
        msg.get("e").and_then(|v| v.as_str()) == Some("depthUpdate")
            && msg.get("U").is_some()
            && msg.get("u").is_some()
    }

    fn is_snapshot(&self, msg: &Value) -> bool {
        msg.get("e").and_then(|v| v.as_str()) == Some("depthUpdate")
            && msg.get("U").is_none()
            && msg.get("lastUpdateId").is_some()
    }

    fn extract_symbol(&self, msg: &Value) -> Option<String> {
        msg.get("s").and_then(|v| v.as_str())?;
        Some(to_unified_symbol(msg))
    }

    fn extract_nonce(&self, msg: &Value) -> Option<i64> {
        msg.get("u")
            .and_then(|v| v.as_i64())
            .or_else(|| msg.get("lastUpdateId").and_then(|v| v.as_i64()))
    }
}

// ============================================================================
// 新设计：StreamParserExt 实现（泛型解析）
// ============================================================================

impl StreamParserExt for BinanceStreamParser {
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
                let trades = parse_trades(msg)?;
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
            "mark_price" => {
                let mp = parse_mark_price(msg)?;
                let result = unsafe { std::mem::transmute_copy(&mp) };
                std::mem::forget(mp);
                Ok(result)
            }
            "bid_ask" => {
                let ba = parse_bids_asks(msg)?;
                let result = unsafe { std::mem::transmute_copy(&ba) };
                std::mem::forget(ba);
                Ok(result)
            }
            "position" => {
                let positions = parse_positions(msg)?;
                let result = unsafe { std::mem::transmute_copy(&positions) };
                std::mem::forget(positions);
                Ok(result)
            }
            _ => Err(Error::invalid_request(format!(
                "Binance parser does not support type: {}",
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
            "e": "24hrTicker",
            "s": "BTCUSDT",
            "c": "50000.00",
            "o": "49000.00",
            "h": "51000.00",
            "l": "48000.00",
            "v": "1000.00",
            "E": 1700000000000i64
        });

        let ticker = parse_ticker(&msg).unwrap();
        assert_eq!(ticker.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert!(ticker.last.is_some());
    }

    #[test]
    fn test_parse_ohlcv() {
        let msg = json!({
            "e": "kline",
            "k": {
                "t": 1700000000000i64,
                "o": "50000.00",
                "h": "51000.00",
                "l": "49000.00",
                "c": "50500.00",
                "v": "100.00"
            }
        });

        let ohlcvs = parse_ohlcv(&msg).unwrap();
        assert_eq!(ohlcvs.len(), 1);
        assert_eq!(ohlcvs[0].timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_trades() {
        let msg = json!({
            "e": "trade",
            "s": "BTCUSDT",
            "p": "50000.00",
            "q": "0.5",
            "m": true,
            "E": 1700000000000i64
        });

        let trades = parse_trades(&msg).unwrap();
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].symbol, Symbol::new_unchecked("BTC/USDT"));
    }
}

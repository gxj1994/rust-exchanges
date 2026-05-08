//! Gate.io WebSocket stream parser
//!
//! Implements the StreamParser trait for Gate.io WebSocket messages.

use ccxt_core::error::Result;
use ccxt_core::types::BidAsk;
use ccxt_core::ws::parser::{
    ExchangeMessage, Parseable, ParsedMessage, StreamParser, StreamParserExt,
};
use serde_json::Value;

/// Gate.io WebSocket stream parser
#[derive(Clone)]
pub struct GateStreamParser;

impl StreamParser for GateStreamParser {
    fn parse(&self, msg: &Value) -> Result<ParsedMessage> {
        let event = msg["event"].as_str().unwrap_or("");

        // Handle errors (check before heartbeat to avoid false positives)
        // Gate errors can have "error" or "message" field
        if let Some(error_msg) = msg["error"].as_str().or_else(|| msg["message"].as_str()) {
            return Ok(ParsedMessage::Error {
                code: None,
                message: error_msg.to_string(),
            });
        }

        // Handle pong (heartbeat response)
        // Only if no error field and no channel field
        if event.is_empty() && msg.get("time").is_some() && msg.get("channel").is_none() {
            return Ok(ParsedMessage::Heartbeat);
        }

        // Handle subscription confirmations
        if event == "subscribe" || event == "unsubscribe" {
            return Ok(ParsedMessage::SubscriptionConfirm {
                channel: msg["channel"].as_str().unwrap_or("").to_string(),
            });
        }

        // Handle data updates
        if event == "update" {
            let channel = msg["channel"].as_str().unwrap_or("").to_string();
            let result = &msg["result"];

            if result.is_null() {
                return Ok(ParsedMessage::ExchangeSpecific(ExchangeMessage {
                    exchange_id: "gate",
                    channel,
                    data: Value::Null,
                }));
            }

            // Route book_ticker channels to BidsAsks
            if channel == "spot.book_ticker" || channel == "futures.book_ticker" {
                use crate::gate::parser;
                let bidask = parser::parse_bids_asks(msg)?;
                return Ok(ParsedMessage::BidsAsks(bidask));
            }

            // Return as exchange-specific message (can be further parsed by consumers)
            Ok(ParsedMessage::ExchangeSpecific(ExchangeMessage {
                exchange_id: "gate",
                channel,
                data: result.clone(),
            }))
        } else {
            // Unknown event
            Ok(ParsedMessage::Unknown(msg.clone()))
        }
    }
}

impl StreamParserExt for GateStreamParser {
    fn parse_as<T: Parseable>(&self, msg: &Value) -> Result<T> {
        use crate::gate::parser;
        use ccxt_core::error::Error;

        match T::TYPE_NAME {
            "ticker" => {
                // Extract result from Gate message format
                let data = if msg.get("result").is_some() {
                    &msg["result"]
                } else {
                    msg
                };

                // Handle array result (futures tickers)
                let ticker = if let Some(arr) = data.as_array() {
                    if let Some(first) = arr.first() {
                        parser::parse_ws_ticker(first, None)?
                    } else {
                        return Err(Error::invalid_request("Empty ticker array"));
                    }
                } else {
                    parser::parse_ws_ticker(data, None)?
                };

                let result = unsafe { std::mem::transmute_copy(&ticker) };
                std::mem::forget(ticker);
                Ok(result)
            }
            "bid_ask" => {
                let bidask: BidAsk = parser::parse_bids_asks(msg)?;
                let result = unsafe { std::mem::transmute_copy(&bidask) };
                std::mem::forget(bidask);
                Ok(result)
            }
            "orderbook" => {
                // Extract result from Gate message format
                let data = if msg.get("result").is_some() {
                    &msg["result"]
                } else {
                    msg
                };

                // Use WS-specific parser that auto-detects spot/futures format
                let orderbook = parser::parse_ws_orderbook(data)?;
                let result = unsafe { std::mem::transmute_copy(&orderbook) };
                std::mem::forget(orderbook);
                Ok(result)
            }
            "trade" => {
                // Extract result from Gate message format
                let data = if msg.get("result").is_some() {
                    &msg["result"]
                } else {
                    msg
                };
                // Gate sends trades as array in result
                let trades_data = if data.is_array() {
                    data.clone()
                } else {
                    serde_json::json!([data])
                };
                // Use WS-specific parser that handles both spot and futures formats
                let mut trades = Vec::new();
                if let Some(arr) = trades_data.as_array() {
                    for trade_data in arr {
                        trades.push(parser::parse_ws_trade(trade_data, None)?);
                    }
                }
                let result = unsafe { std::mem::transmute_copy(&trades) };
                std::mem::forget(trades);
                Ok(result)
            }
            "ohlcv" => {
                // Extract result from Gate message format
                let data = if msg.get("result").is_some() {
                    &msg["result"]
                } else {
                    msg
                };

                // Gate WS sends candlesticks as object (spot) or array (futures)
                // Use WS-specific parser that handles both formats
                let ohlcv = parser::parse_ws_ohlcv(data)?;
                let ohlcvs = vec![ohlcv];
                let result = unsafe { std::mem::transmute_copy(&ohlcvs) };
                std::mem::forget(ohlcvs);
                Ok(result)
            }
            _ => Err(Error::invalid_request(format!(
                "Gate parser does not support type: {}",
                T::TYPE_NAME
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ticker_update() {
        let parser = GateStreamParser;
        let message = serde_json::json!({
            "time": 1606292218,
            "channel": "spot.tickers",
            "event": "update",
            "result": {
                "currency_pair": "BTC_USDT",
                "last": "50000.0"
            }
        });

        let result = parser.parse(&message).unwrap();
        match result {
            ParsedMessage::ExchangeSpecific(msg) => {
                assert_eq!(msg.channel, "spot.tickers");
                assert_eq!(msg.data["currency_pair"], "BTC_USDT");
            }
            _ => panic!("Expected ExchangeSpecific message"),
        }
    }

    #[test]
    fn test_parse_subscribe_ack() {
        let parser = GateStreamParser;
        let message = serde_json::json!({
            "time": 1606292218,
            "channel": "spot.tickers",
            "event": "subscribe",
            "result": null
        });

        let result = parser.parse(&message).unwrap();
        match result {
            ParsedMessage::SubscriptionConfirm { channel } => {
                assert_eq!(channel, "spot.tickers");
            }
            _ => panic!("Expected SubscriptionConfirm message"),
        }
    }

    #[test]
    fn test_parse_heartbeat() {
        let parser = GateStreamParser;
        let message = serde_json::json!({
            "time": 1606292218
        });

        let result = parser.parse(&message).unwrap();
        assert!(matches!(result, ParsedMessage::Heartbeat));
    }

    #[test]
    fn test_parse_error() {
        let parser = GateStreamParser;
        let message = serde_json::json!({
            "time": 1606292218,
            "channel": "spot.tickers",
            "event": "update",
            "error": "Invalid symbol"
        });

        let result = parser.parse(&message).unwrap();
        match result {
            ParsedMessage::Error { message, .. } => {
                assert_eq!(message, "Invalid symbol");
            }
            _ => panic!("Expected Error message"),
        }
    }
}

//! WebSocket Trade data parser for Bybit.

use ccxt_core::{
    error::{Error, Result},
    types::{
        Fee, OrderSide, Symbol, TakerOrMaker, Trade,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use super::to_unified_symbol;

/// Parse trades from WebSocket publicTrade message.
pub fn parse_trades(msg: &Value) -> Result<Vec<Trade>> {
    let data = msg
        .get("data")
        .ok_or_else(|| Error::invalid_request("Missing data in message"))?;

    let symbol_str = msg
        .get("topic")
        .and_then(|t| t.as_str())
        .and_then(|t| t.strip_prefix("publicTrade."))
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(symbol_str, Some(msg)));

    let trades: Vec<Trade> = data
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let trade_id = item
                        .get("i")
                        .and_then(|i| i.as_str())
                        .map(ToString::to_string);

                    let side = match item.get("S").and_then(|s| s.as_str()) {
                        Some("Buy") => OrderSide::Buy,
                        Some("Sell") => OrderSide::Sell,
                        _ => OrderSide::Buy,
                    };

                    let price = item
                        .get("p")
                        .and_then(|p| p.as_str())
                        .and_then(|s| Decimal::from_str(s).ok())
                        .map(Price::new)?;

                    let amount = item
                        .get("v")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Decimal::from_str(s).ok())
                        .map(Amount::new)?;

                    let timestamp = item.get("T").and_then(|t| t.as_u64()).unwrap_or(0) as i64;

                    Some(Trade {
                        id: trade_id,
                        order: None,
                        symbol: symbol.clone(),
                        trade_type: None,
                        side,
                        taker_or_maker: None,
                        price,
                        amount,
                        cost: None,
                        fee: None,
                        timestamp,
                        datetime: None,
                        info: HashMap::new(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(trades)
}

/// Parse execution records (my trades) from WebSocket message.
pub fn parse_execution(msg: &Value) -> Result<Vec<Trade>> {
    let data = msg
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::invalid_request("Missing data in execution message"))?;

    let trades: Vec<Trade> = data
        .iter()
        .filter_map(|item| {
            let symbol_str = item
                .get("symbol")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let symbol = Symbol::new_unchecked(to_unified_symbol(symbol_str, Some(msg)));

            let trade_id = item
                .get("execId")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);

            let order_id = item
                .get("orderId")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);

            let side = match item.get("side").and_then(|v| v.as_str()) {
                Some("Sell") => OrderSide::Sell,
                _ => OrderSide::Buy,
            };

            let price = item
                .get("execPrice")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .map(Price::new)?;

            let amount = item
                .get("execQty")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .map(Amount::new)?;

            let timestamp = item
                .get("execTime")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            let fee = item
                .get("execFee")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .map(|cost| {
                    let currency = item
                        .get("feeCurrency")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    Fee {
                        cost,
                        currency,
                        rate: None,
                    }
                });

            let is_maker = item
                .get("isMaker")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            Some(Trade {
                id: trade_id,
                order: order_id,
                symbol,
                trade_type: None,
                side,
                taker_or_maker: Some(if is_maker {
                    TakerOrMaker::Maker
                } else {
                    TakerOrMaker::Taker
                }),
                price,
                amount,
                cost: None,
                fee,
                timestamp,
                datetime: None,
                info: HashMap::new(),
            })
        })
        .collect();

    Ok(trades)
}

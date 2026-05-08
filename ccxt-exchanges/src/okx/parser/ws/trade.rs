//! WebSocket Trade data parser for OKX.

use ccxt_core::{
    error::{Error, Result},
    types::{
        OrderSide, Symbol, Trade,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use super::{extract_data, to_unified_symbol};

/// Parse trades from WebSocket message.
pub fn parse_trades(msg: &Value) -> Result<Vec<Trade>> {
    let data = extract_data(msg)?;

    let arg = msg
        .get("arg")
        .ok_or_else(|| Error::invalid_request("Missing arg in trades message"))?;
    let inst_id = arg
        .get("instId")
        .and_then(|i| i.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(inst_id));

    let trades: Vec<Trade> = data
        .iter()
        .filter_map(|item| {
            let trade_id = item
                .get("tradeId")
                .and_then(|t| t.as_str())
                .map(ToString::to_string);

            let side = match item.get("side").and_then(|s| s.as_str()) {
                Some("buy") => OrderSide::Buy,
                Some("sell") => OrderSide::Sell,
                _ => OrderSide::Buy,
            };

            let price = item
                .get("px")
                .and_then(|p| p.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .map(Price::new)?;

            let amount = item
                .get("sz")
                .and_then(|s| s.as_str())
                .and_then(|s| Decimal::from_str(s).ok())
                .map(Amount::new)?;

            let timestamp = item
                .get("ts")
                .and_then(|t| t.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

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
        .collect();

    Ok(trades)
}

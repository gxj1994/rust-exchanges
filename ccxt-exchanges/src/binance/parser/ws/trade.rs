//! WebSocket Trade data parser for Binance.
//!
//! Parses trade, aggTrade, and ORDER_TRADE_UPDATE events.

use ccxt_core::{
    error::{Error, Result},
    types::{
        Fee, OrderSide, Symbol, TakerOrMaker, Trade,
        financial::{Amount, Cost, Price},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;

use super::{parse_decimal, to_unified_symbol, to_unified_symbol_with_mt};

/// Parse trades from WebSocket trade/aggTrade message.
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a vector of CCXT [`Trade`] structures.
pub fn parse_trades(msg: &Value) -> Result<Vec<Trade>> {
    let symbol = Symbol::new_unchecked(to_unified_symbol(msg));

    let timestamp = msg.get("E").and_then(|t| t.as_i64()).unwrap_or(0);

    let price = parse_decimal(msg, "p")
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));
    let amount = parse_decimal(msg, "q")
        .map(Amount::new)
        .unwrap_or_else(|| Amount::new(Decimal::ZERO));

    let side = msg
        .get("m")
        .and_then(|m| m.as_bool())
        .map(|is_buyer_maker| {
            if is_buyer_maker {
                OrderSide::Sell
            } else {
                OrderSide::Buy
            }
        })
        .unwrap_or(OrderSide::Buy);

    let id = msg.get("t").and_then(|t| t.as_u64()).map(|i| i.to_string());

    let trade = Trade {
        id,
        order: None,
        symbol,
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
    };

    Ok(vec![trade])
}

/// Parse user trade from executionReport (spot).
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
pub fn parse_account_trade(msg: &Value) -> Result<Trade> {
    let symbol = Symbol::new_unchecked(to_unified_symbol(msg));

    let id = msg.get("t").and_then(|t| t.as_i64()).map(|v| v.to_string());

    let order_id = msg.get("i").and_then(|i| i.as_u64()).map(|v| v.to_string());

    let timestamp = msg
        .get("T")
        .and_then(|t| t.as_i64())
        .unwrap_or_else(|| msg.get("E").and_then(|t| t.as_i64()).unwrap_or(0));

    let price = parse_decimal(msg, "L").unwrap_or_default();
    let amount = parse_decimal(msg, "l").unwrap_or_default();
    let cost = parse_decimal(msg, "Y").unwrap_or_else(|| price * amount);

    let side = match msg.get("S").and_then(|s| s.as_str()) {
        Some("BUY") => OrderSide::Buy,
        Some("SELL") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let taker_or_maker = msg.get("m").and_then(|m| m.as_bool()).map(|is_maker| {
        if is_maker {
            TakerOrMaker::Maker
        } else {
            TakerOrMaker::Taker
        }
    });

    // Parse fee
    let fee = parse_decimal(msg, "n").and_then(|fee_cost| {
        let fee_currency = msg.get("N").and_then(|n| n.as_str()).unwrap_or("");
        Some(Fee {
            cost: fee_cost,
            currency: fee_currency.to_string(),
            rate: None,
        })
    });

    Ok(Trade {
        id,
        order: order_id,
        symbol,
        trade_type: None,
        side,
        taker_or_maker,
        price: Price::new(price),
        amount: Amount::new(amount),
        cost: Some(Cost::new(cost)),
        fee,
        timestamp,
        datetime: None,
        info: HashMap::new(),
    })
}

/// Parse user trade from ORDER_TRADE_UPDATE (futures).
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Trade`] structure.
pub fn parse_account_trade_futures(msg: &Value) -> Result<Trade> {
    let order_data = msg
        .get("o")
        .ok_or_else(|| Error::invalid_request("Missing order data 'o' in ORDER_TRADE_UPDATE"))?;

    // Only handle TRADE type
    let exec_type = order_data.get("x").and_then(|x| x.as_str()).unwrap_or("");
    if exec_type != "TRADE" {
        return Err(Error::invalid_request(format!(
            "ORDER_TRADE_UPDATE execution type is '{}', not 'TRADE'",
            exec_type
        )));
    }

    let symbol_str = order_data
        .get("s")
        .and_then(|s| s.as_str())
        .unwrap_or_default();
    let mt = msg.get("_ccxt_mt").and_then(|v| v.as_str());
    let symbol = Symbol::new_unchecked(to_unified_symbol_with_mt(symbol_str, mt));

    let id = order_data
        .get("t")
        .and_then(|t| t.as_i64())
        .map(|v| v.to_string());

    let order_id = order_data
        .get("i")
        .and_then(|i| i.as_u64())
        .map(|v| v.to_string());

    let timestamp = order_data
        .get("T")
        .and_then(|t| t.as_i64())
        .unwrap_or_else(|| msg.get("T").and_then(|t| t.as_i64()).unwrap_or(0));

    let price = parse_decimal(order_data, "L").unwrap_or_default();
    let amount = parse_decimal(order_data, "l").unwrap_or_default();
    let cost = parse_decimal(order_data, "Y").unwrap_or_else(|| price * amount);

    let side = match order_data.get("S").and_then(|s| s.as_str()) {
        Some("BUY") => OrderSide::Buy,
        Some("SELL") => OrderSide::Sell,
        _ => OrderSide::Buy,
    };

    let taker_or_maker = order_data
        .get("m")
        .and_then(|m| m.as_bool())
        .map(|is_maker| {
            if is_maker {
                TakerOrMaker::Maker
            } else {
                TakerOrMaker::Taker
            }
        });

    let fee = parse_decimal(order_data, "n").and_then(|fee_cost| {
        let fee_currency = order_data.get("N").and_then(|n| n.as_str()).unwrap_or("");
        Some(Fee {
            cost: fee_cost,
            currency: fee_currency.to_string(),
            rate: None,
        })
    });

    Ok(Trade {
        id,
        order: order_id,
        symbol,
        trade_type: None,
        side,
        taker_or_maker,
        price: Price::new(price),
        amount: Amount::new(amount),
        cost: Some(Cost::new(cost)),
        fee,
        timestamp,
        datetime: None,
        info: HashMap::new(),
    })
}

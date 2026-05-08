//! WebSocket Trade data parser for HyperLiquid.
//!
//! Parses trades and userFills channel messages.

use ccxt_core::{
    error::Result,
    types::{
        OrderSide, Symbol, Trade,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use crate::hyperliquid::core::symbol::HyperliquidSymbolConverter;

/// Parse trades from WebSocket trades message.
///
/// Handles both data formats:
/// - Data format: `WsTrade[]` (direct array of trades as per Hyperliquid docs)
/// - Legacy format: `{ coin, trades: [...] }` (for backward compatibility)
///
/// # Arguments
///
/// * `data` - The `data` field extracted from WebSocket message
///
/// # Returns
///
/// Returns a vector of CCXT [`Trade`] structures.
pub fn parse_trades(data: &Value) -> Result<Vec<Trade>> {
    // Helper to parse a single trade item
    fn parse_trade_item(item: &Value, default_symbol: &Symbol) -> Option<Trade> {
        let coin = item
            .get("coin")
            .and_then(|c| c.as_str())
            .unwrap_or_default();
        let symbol = if coin.is_empty() {
            default_symbol.clone()
        } else {
            Symbol::new_unchecked(HyperliquidSymbolConverter::exchange_to_unified_inferred(
                coin,
            ))
        };

        let price = item.get("px")?.as_str()?;
        let size = item.get("sz")?.as_str()?;
        let side = item.get("side")?.as_str()?;

        let price = Decimal::from_str(price).ok()?;
        let amount = Decimal::from_str(size).ok()?;

        let timestamp = item.get("time")?.as_i64()?;

        Some(Trade {
            id: item
                .get("tid")
                .and_then(|t| t.as_u64())
                .map(|i| i.to_string())
                .or_else(|| {
                    item.get("hash")
                        .and_then(|h| h.as_str())
                        .map(|s| s.to_string())
                }),
            order: None,
            symbol,
            trade_type: None,
            side: if side == "B" {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            },
            taker_or_maker: None,
            price: Price::new(price),
            amount: Amount::new(amount),
            cost: None,
            fee: None,
            timestamp,
            datetime: None,
            info: HashMap::new(),
        })
    }

    let default_coin = data
        .get("coin")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    let default_symbol = Symbol::new_unchecked(
        HyperliquidSymbolConverter::exchange_to_unified_inferred(default_coin),
    );

    // Try legacy format first: data.trades is an array
    let legacy_trades: Option<Vec<Trade>> =
        data.get("trades").and_then(|t| t.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|item| parse_trade_item(item, &default_symbol))
                .collect()
        });

    if let Some(trades) = legacy_trades {
        if !trades.is_empty() {
            return Ok(trades);
        }
    }

    // Actual Hyperliquid format: data is a direct WsTrade[] array
    if let Some(arr) = data.as_array() {
        let trades: Vec<Trade> = arr
            .iter()
            .filter_map(|item| parse_trade_item(item, &default_symbol))
            .collect();
        return Ok(trades);
    }

    Ok(Vec::new())
}

/// Parse user fills from WebSocket userFills message.
///
/// # Arguments
///
/// * `data` - The `data` field extracted from WebSocket message
///
/// # Returns
///
/// Returns a vector of CCXT [`Trade`] structures.
pub fn parse_user_fills(data: &Value) -> Result<Vec<Trade>> {
    let fills = data
        .get("fills")
        .and_then(|f| f.as_array())
        .map(|arr| arr.to_vec())
        .unwrap_or_default();

    let trades: Vec<Trade> = fills
        .iter()
        .filter_map(|fill| {
            let coin = fill
                .get("coin")
                .and_then(|c| c.as_str())
                .unwrap_or_default();
            let symbol = Symbol::new_unchecked(
                HyperliquidSymbolConverter::exchange_to_unified_inferred(coin),
            );

            let price = fill.get("px")?.as_str()?;
            let amount = fill.get("sz")?.as_str()?;
            let side = fill.get("side")?.as_str()?;

            let price = Decimal::from_str(price).ok()?;
            let amount = Decimal::from_str(amount).ok()?;

            let timestamp = fill.get("time").and_then(|t| t.as_i64()).unwrap_or(0);

            Some(Trade {
                id: fill
                    .get("oid")
                    .and_then(|o| o.as_u64())
                    .map(|i| i.to_string()),
                order: None,
                symbol,
                trade_type: None,
                side: if side == "B" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                taker_or_maker: None,
                price: Price::new(price),
                amount: Amount::new(amount),
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

//! WebSocket Position data parser for Binance.
//!
//! Parses ACCOUNT_UPDATE position data.

use ccxt_core::{error::Result, types::Position};
use serde_json::Value;

use super::to_unified_symbol_with_mt;

/// Parse positions from WebSocket ACCOUNT_UPDATE message.
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a vector of CCXT [`Position`] structures.
pub fn parse_positions(msg: &Value) -> Result<Vec<Position>> {
    let mut positions = Vec::new();

    // ACCOUNT_UPDATE message format:
    // { "e": "ACCOUNT_UPDATE", "E": ..., "a": { "B": [...], "P": [...] } }
    if let Some(account_data) = msg.get("a") {
        if let Some(positions_arr) = account_data.get("P").and_then(|p| p.as_array()) {
            for pos_data in positions_arr {
                let symbol_str = pos_data
                    .get("s")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();
                let mt = msg.get("_ccxt_mt").and_then(|v| v.as_str());
                let symbol = to_unified_symbol_with_mt(symbol_str, mt);

                let position_amount_str =
                    pos_data.get("pa").and_then(|pa| pa.as_str()).unwrap_or("0");
                let position_amount: f64 = position_amount_str.parse().unwrap_or(0.0);

                let position_side = pos_data
                    .get("ps")
                    .and_then(|ps| ps.as_str())
                    .unwrap_or("BOTH")
                    .to_uppercase();

                let (side, hedged) = if position_side == "BOTH" {
                    let actual_side = if position_amount < 0.0 {
                        "short"
                    } else {
                        "long"
                    };
                    (actual_side.to_string(), false)
                } else {
                    (position_side.to_lowercase(), true)
                };

                let entry_price = pos_data
                    .get("ep")
                    .and_then(|ep| ep.as_str())
                    .and_then(|s| s.parse::<f64>().ok());
                let unrealized_pnl = pos_data
                    .get("up")
                    .and_then(|up| up.as_str())
                    .and_then(|s| s.parse::<f64>().ok());
                let realized_pnl = pos_data
                    .get("cr")
                    .and_then(|cr| cr.as_str())
                    .and_then(|s| s.parse::<f64>().ok());
                let margin_mode = pos_data
                    .get("mt")
                    .and_then(|mt| mt.as_str())
                    .map(|s| s.to_string());
                let initial_margin = pos_data
                    .get("iw")
                    .and_then(|iw| iw.as_str())
                    .and_then(|s| s.parse::<f64>().ok());

                let timestamp = msg.get("E").and_then(|t| t.as_i64());

                positions.push(Position {
                    info: pos_data.clone(),
                    id: None,
                    symbol,
                    side: Some(side),
                    position_side: None,
                    dual_side_position: None,
                    contracts: Some(position_amount.abs()),
                    contract_size: None,
                    entry_price,
                    mark_price: None,
                    notional: None,
                    leverage: None,
                    collateral: initial_margin,
                    initial_margin,
                    initial_margin_percentage: None,
                    maintenance_margin: None,
                    maintenance_margin_percentage: None,
                    unrealized_pnl,
                    realized_pnl,
                    liquidation_price: None,
                    margin_ratio: None,
                    margin_mode,
                    hedged: Some(hedged),
                    percentage: None,
                    timestamp,
                    datetime: None,
                });
            }
        }
    }

    Ok(positions)
}

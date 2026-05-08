//! Position parser for OKX.

use ccxt_core::{Result, parser_utils::parse_timestamp, types::account::position::PositionSide};
use serde_json::Value;

use super::funding_rate::parse_f64_field;

/// Parse position data from OKX account positions response.
///
/// OKX position fields:
/// - instId: instrument ID
/// - posSide: position side (long/short/net)
/// - pos: position quantity
/// - avgPx: average entry price
/// - markPx: mark price
/// - upl: unrealized PnL
/// - lever: leverage
/// - liqPx: liquidation price
/// - mgnMode: margin mode (cross/isolated)
/// - imr: initial margin requirement
/// - mmr: maintenance margin requirement
/// - cTime: creation time
/// - uTime: update time
///
/// # Arguments
///
/// * `data` - OKX position data JSON object
/// * `symbol` - Unified symbol string
///
/// # Returns
///
/// Returns a CCXT [`Position`] structure.
pub fn parse_position(data: &Value, symbol: &str) -> Result<ccxt_core::types::Position> {
    let pos_side_str = data["posSide"].as_str().unwrap_or("net");
    let position_side = match pos_side_str.to_lowercase().as_str() {
        "long" => PositionSide::Long,
        "short" => PositionSide::Short,
        _ => PositionSide::Both,
    };

    let pos = parse_f64_field(data, "pos").unwrap_or(0.0);
    let avg_px = parse_f64_field(data, "avgPx");
    let mark_px = parse_f64_field(data, "markPx");
    let upl = parse_f64_field(data, "upl");
    let lever = parse_f64_field(data, "lever");
    let liq_px = parse_f64_field(data, "liqPx");
    let imr = parse_f64_field(data, "imr");
    let mmr = parse_f64_field(data, "mmr");
    let notional_usd = parse_f64_field(data, "notionalUsd");
    let margin = parse_f64_field(data, "margin");
    let realized_pnl = parse_f64_field(data, "realizedPnl");

    let mgn_mode = data["mgnMode"].as_str().unwrap_or("cross");
    let margin_mode = Some(mgn_mode.to_string());

    let timestamp = parse_timestamp(data, "uTime").or_else(|| parse_timestamp(data, "cTime"));
    let datetime = timestamp.and_then(ccxt_core::parser_utils::timestamp_to_datetime);

    // Determine side from position quantity or posSide
    let side = match position_side {
        PositionSide::Long => Some("long".to_string()),
        PositionSide::Short => Some("short".to_string()),
        PositionSide::Both => {
            if pos > 0.0 {
                Some("long".to_string())
            } else if pos < 0.0 {
                Some("short".to_string())
            } else {
                None
            }
        }
    };

    let contracts = Some(pos.abs());

    // Calculate initial margin percentage from leverage
    let initial_margin_percentage = lever.map(|l| if l > 0.0 { 1.0 / l } else { 0.0 });

    // Calculate notional value
    let notional = notional_usd.or(match (avg_px, contracts) {
        (Some(price), Some(qty)) => Some(price * qty),
        _ => None,
    });

    // Calculate percentage PnL
    let percentage = match (upl, margin.or(imr)) {
        (Some(pnl), Some(m)) if m > 0.0 => Some((pnl / m) * 100.0),
        _ => None,
    };

    let hedged = match position_side {
        PositionSide::Both => Some(false),
        _ => Some(true),
    };

    Ok(ccxt_core::types::Position {
        info: data.clone(),
        id: data["posId"].as_str().map(ToString::to_string),
        symbol: symbol.to_string(),
        side,
        position_side: Some(position_side),
        dual_side_position: hedged,
        contracts,
        contract_size: parse_f64_field(data, "ctVal"),
        entry_price: avg_px,
        mark_price: mark_px,
        notional,
        leverage: lever,
        collateral: margin,
        initial_margin: imr,
        initial_margin_percentage,
        maintenance_margin: mmr,
        maintenance_margin_percentage: None,
        unrealized_pnl: upl,
        realized_pnl,
        liquidation_price: liq_px,
        margin_ratio: None,
        margin_mode,
        hedged,
        percentage,
        timestamp,
        datetime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_position_long() {
        let data = json!({
            "instId": "BTC-USDT-SWAP",
            "posId": "12345",
            "posSide": "long",
            "pos": "1.5",
            "avgPx": "50000.00",
            "markPx": "51000.00",
            "upl": "1500.00",
            "lever": "10",
            "liqPx": "45000.00",
            "mgnMode": "cross",
            "imr": "5000.00",
            "mmr": "500.00",
            "notionalUsd": "76500.00",
            "margin": "5000.00",
            "uTime": "1700000000000"
        });

        let position = parse_position(&data, "BTC/USDT:USDT").unwrap();
        assert_eq!(position.symbol, "BTC/USDT:USDT");
        assert_eq!(position.side, Some("long".to_string()));
        assert_eq!(position.contracts, Some(1.5));
        assert_eq!(position.entry_price, Some(50000.00));
        assert_eq!(position.mark_price, Some(51000.00));
        assert_eq!(position.unrealized_pnl, Some(1500.00));
        assert_eq!(position.leverage, Some(10.0));
        assert_eq!(position.liquidation_price, Some(45000.00));
        assert_eq!(position.margin_mode, Some("cross".to_string()));
        assert_eq!(position.initial_margin, Some(5000.00));
        assert_eq!(position.maintenance_margin, Some(500.00));
        assert_eq!(position.hedged, Some(true));
    }

    #[test]
    fn test_parse_position_short() {
        let data = json!({
            "instId": "ETH-USDT-SWAP",
            "posSide": "short",
            "pos": "-10",
            "avgPx": "3000.00",
            "markPx": "2900.00",
            "upl": "1000.00",
            "lever": "5",
            "mgnMode": "isolated",
            "uTime": "1700000000000"
        });

        let position = parse_position(&data, "ETH/USDT:USDT").unwrap();
        assert_eq!(position.side, Some("short".to_string()));
        assert_eq!(position.contracts, Some(10.0));
        assert_eq!(position.margin_mode, Some("isolated".to_string()));
        assert_eq!(position.hedged, Some(true));
    }

    #[test]
    fn test_parse_position_net_mode() {
        let data = json!({
            "instId": "BTC-USDT-SWAP",
            "posSide": "net",
            "pos": "2",
            "avgPx": "50000.00",
            "lever": "10",
            "mgnMode": "cross",
            "uTime": "1700000000000"
        });

        let position = parse_position(&data, "BTC/USDT:USDT").unwrap();
        assert_eq!(position.side, Some("long".to_string()));
        assert_eq!(position.contracts, Some(2.0));
        assert_eq!(position.hedged, Some(false));
    }

    #[test]
    fn test_parse_position_empty_fields() {
        let data = json!({
            "instId": "BTC-USDT-SWAP",
            "posSide": "net",
            "pos": "0",
            "avgPx": "",
            "markPx": "",
            "upl": "",
            "lever": "10",
            "mgnMode": "cross"
        });

        let position = parse_position(&data, "BTC/USDT:USDT").unwrap();
        assert_eq!(position.contracts, Some(0.0));
        assert_eq!(position.entry_price, None);
        assert_eq!(position.mark_price, None);
        assert_eq!(position.unrealized_pnl, None);
    }
}

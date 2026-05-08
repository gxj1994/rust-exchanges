//! Position data parser for Bitget.

use ccxt_core::{
    Result,
    parser_utils::parse_timestamp,
    types::{Position, account::position::PositionSide},
};
use serde_json::Value;

use super::{parse_f64_field, timestamp_to_datetime};

/// Parse position data from Bitget V3 position response.
///
/// # V3 API Fields
/// - symbol: exchange symbol (e.g., "BTCUSDT")
/// - category: product type (USDT-FUTURES/COIN-FUTURES/USDC-FUTURES)
/// - posSide: position side (long/short)
/// - total: total position size (available + frozen)
/// - available: available position size
/// - frozen: frozen position size
/// - avgPrice: average entry price
/// - markPrice: mark price
/// - unrealisedPnl: unrealized PnL
/// - curRealisedPnl: realized PnL
/// - leverage: leverage multiplier
/// - liquidationPrice: liquidation price
/// - marginMode: margin mode (crossed/isolated)
/// - positionBalance: position margin
/// - holdMode: position mode (one_way_mode/hedge_mode)
/// - createdTime/updatedTime: timestamps
pub fn parse_position(data: &Value, symbol: &str) -> Result<Position> {
    // V3 API 使用 posSide 而不是 holdSide
    let hold_side = data["posSide"]
        .as_str()
        .or_else(|| data["holdSide"].as_str())
        .unwrap_or("long");

    let position_side = match hold_side.to_lowercase().as_str() {
        "short" => PositionSide::Short,
        "long" => PositionSide::Long,
        _ => PositionSide::Both,
    };

    let side = match position_side {
        PositionSide::Long => Some("long".to_string()),
        PositionSide::Short => Some("short".to_string()),
        PositionSide::Both => None,
    };

    // V3 API 字段映射
    let total = parse_f64_field(data, "total")
        .or_else(|| parse_f64_field(data, "openDelegateSize"))
        .unwrap_or(0.0);

    let avg_px = parse_f64_field(data, "avgPrice")
        .or_else(|| parse_f64_field(data, "averageOpenPrice"))
        .or_else(|| parse_f64_field(data, "openPriceAvg"));

    let mark_px = parse_f64_field(data, "markPrice");

    let upl = parse_f64_field(data, "unrealisedPnl")
        .or_else(|| parse_f64_field(data, "unrealizedPL"))
        .or_else(|| parse_f64_field(data, "unrealizedPl"));

    let lever = parse_f64_field(data, "leverage");
    let liq_px = parse_f64_field(data, "liquidationPrice");

    // V3 使用 positionBalance 作为保证金
    let margin =
        parse_f64_field(data, "positionBalance").or_else(|| parse_f64_field(data, "margin"));

    // V3 使用 curRealisedPnl
    let realized_pnl = parse_f64_field(data, "curRealisedPnl")
        .or_else(|| parse_f64_field(data, "achievedProfits"));

    let mgn_mode = data["marginMode"].as_str().unwrap_or("crossed");
    let margin_mode = Some(match mgn_mode {
        "isolated" => "isolated".to_string(),
        _ => "cross".to_string(),
    });

    // V3 使用 updatedTime/createdTime
    let timestamp = parse_timestamp(data, "updatedTime")
        .or_else(|| parse_timestamp(data, "createdTime"))
        .or_else(|| parse_timestamp(data, "uTime"))
        .or_else(|| parse_timestamp(data, "cTime"));
    let datetime = timestamp.and_then(timestamp_to_datetime);

    let initial_margin_percentage = lever.map(|l| if l > 0.0 { 1.0 / l } else { 0.0 });

    let notional = match (avg_px, Some(total)) {
        (Some(price), Some(qty)) if qty > 0.0 => Some(price * qty),
        _ => None,
    };

    let percentage = match (upl, margin) {
        (Some(pnl), Some(m)) if m > 0.0 => Some((pnl / m) * 100.0),
        _ => None,
    };

    Ok(Position {
        info: data.clone(),
        id: data["posId"]
            .as_str()
            .or_else(|| data["trackingNo"].as_str())
            .map(ToString::to_string),
        symbol: symbol.to_string(),
        side,
        position_side: Some(position_side),
        dual_side_position: None,
        contracts: Some(total),
        contract_size: parse_f64_field(data, "contractSize"),
        entry_price: avg_px,
        mark_price: mark_px,
        notional,
        leverage: lever,
        collateral: margin,
        initial_margin: margin,
        initial_margin_percentage,
        maintenance_margin: None,
        maintenance_margin_percentage: None,
        unrealized_pnl: upl,
        realized_pnl,
        liquidation_price: liq_px,
        margin_ratio: None,
        margin_mode,
        hedged: None,
        percentage,
        timestamp,
        datetime,
    })
}

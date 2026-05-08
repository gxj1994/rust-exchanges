//! Market data parser for OKX.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{Market, MarketLimits, MarketPrecision, MarketType, MinMax, Symbol},
};
use serde_json::Value;

/// Parse market data from OKX exchange info.
///
/// OKX uses `instId` for instrument ID (e.g., "BTC-USDT").
///
/// # Arguments
///
/// * `data` - OKX market data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure.
pub fn parse_market(data: &Value) -> Result<Market> {
    // OKX uses "instId" for the exchange-specific ID (e.g., "BTC-USDT")
    let id = data["instId"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("instId")))?
        .to_string();

    // Parse instId parts for fallback
    let parts: Vec<&str> = id.split('-').collect();
    let (base_from_id, quote_from_id) = if parts.len() >= 2 {
        (Some(parts[0]), Some(parts[1]))
    } else {
        (None, None)
    };

    // Base and quote currencies
    let base = data["baseCcy"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| base_from_id.map(ToString::to_string))
        .ok_or_else(|| Error::from(ParseError::missing_field("baseCcy")))?;

    let quote = data["quoteCcy"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| quote_from_id.map(ToString::to_string))
        .ok_or_else(|| Error::from(ParseError::missing_field("quoteCcy")))?;

    // Instrument type
    let inst_type = data["instType"].as_str().unwrap_or("SPOT");
    let market_type = match inst_type {
        "SWAP" => MarketType::Swap,
        "FUTURES" => MarketType::Futures,
        "OPTION" => MarketType::Option,
        _ => MarketType::Spot,
    };

    // Market status
    let state = data["state"].as_str().unwrap_or("live");
    let active = state == "live";

    // Parse precision - OKX uses tickSz and lotSz
    let price_precision = parse_decimal(data, "tickSz");
    let amount_precision = parse_decimal(data, "lotSz");

    // Parse limits
    let min_amount = parse_decimal(data, "minSz");
    let max_amount = parse_decimal(data, "maxLmtSz");

    // Contract-specific fields
    let contract = inst_type != "SPOT";
    let linear = if contract {
        Some(data["ctType"].as_str() == Some("linear"))
    } else {
        None
    };
    let inverse = if contract {
        Some(data["ctType"].as_str() == Some("inverse"))
    } else {
        None
    };
    let contract_size = parse_decimal(data, "ctVal");

    // Settlement currency for derivatives
    let settle = data["settleCcy"].as_str().map(ToString::to_string);
    let settle_id = settle.clone();

    // Expiry for futures/options
    let expiry = parse_timestamp(data, "expTime");
    let expiry_datetime = expiry.and_then(ccxt_core::parser_utils::timestamp_to_datetime);

    // Build unified symbol format based on market type:
    // - Spot: BASE/QUOTE (e.g., "BTC/USDT")
    // - Swap: BASE/QUOTE:SETTLE (e.g., "BTC/USDT:USDT")
    // - Futures: BASE/QUOTE:SETTLE-YYMMDD (e.g., "BTC/USDT:USDT-241231")
    let symbol = match market_type {
        MarketType::Spot => format!("{}/{}", base, quote),
        MarketType::Swap => {
            if let Some(ref s) = settle {
                format!("{}/{}:{}", base, quote, s)
            } else {
                // Fallback: use quote as settle for linear
                format!("{}/{}:{}", base, quote, quote)
            }
        }
        MarketType::Futures | MarketType::Option => {
            if let (Some(s), Some(exp_ts)) = (&settle, expiry) {
                // Convert timestamp to YYMMDD format
                if let Some(dt) = chrono::DateTime::from_timestamp_millis(exp_ts) {
                    let year = (dt.format("%y").to_string().parse::<u8>()).unwrap_or(0);
                    let month = (dt.format("%m").to_string().parse::<u8>()).unwrap_or(1);
                    let day = (dt.format("%d").to_string().parse::<u8>()).unwrap_or(1);
                    format!("{}/{}:{}-{:02}{:02}{:02}", base, quote, s, year, month, day)
                } else {
                    format!("{}/{}:{}", base, quote, s)
                }
            } else if let Some(ref s) = settle {
                format!("{}/{}:{}", base, quote, s)
            } else {
                format!("{}/{}", base, quote)
            }
        }
    };

    // Parse the symbol to get structured representation
    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&symbol).ok();

    Ok(Market {
        id,
        symbol: Symbol::new_unchecked(symbol),
        parsed_symbol,
        base: base.clone(),
        quote: quote.clone(),
        settle,
        base_id: Some(base),
        quote_id: Some(quote),
        settle_id,
        market_type,
        active,
        margin: inst_type == "MARGIN",
        contract: Some(contract),
        linear,
        inverse,
        contract_size,
        expiry,
        expiry_datetime,
        strike: parse_decimal(data, "stk"),
        option_type: data["optType"].as_str().map(ToString::to_string),
        precision: MarketPrecision {
            price: price_precision,
            amount: amount_precision,
            base: None,
            quote: None,
        },
        limits: MarketLimits {
            amount: Some(MinMax {
                min: min_amount,
                max: max_amount,
            }),
            price: None,
            cost: None,
            leverage: None,
        },
        maker: parse_decimal(data, "makerFee"),
        taker: parse_decimal(data, "takerFee"),
        percentage: Some(true),
        tier_based: Some(false),
        fee_side: Some("quote".to_string()),
        info: value_to_hashmap(data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_market_swap_empty_base_quote() {
        let data = json!({
            "instId": "BTC-USDT-SWAP",
            "instType": "SWAP",
            "baseCcy": "",
            "quoteCcy": "",
            "settleCcy": "USDT",
            "state": "live",
            "tickSz": "0.1",
            "lotSz": "1",
            "minSz": "1",
            "ctVal": "100"
        });

        // This should now correctly infer base/quote from instId
        let market = parse_market(&data).unwrap();

        // Assertions for FIXED behavior
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert_eq!(market.symbol, Symbol::new_unchecked("BTC/USDT:USDT"));
    }

    #[test]
    fn test_parse_market() {
        let data = json!({
            "instId": "BTC-USDT",
            "instType": "SPOT",
            "baseCcy": "BTC",
            "quoteCcy": "USDT",
            "state": "live",
            "tickSz": "0.01",
            "lotSz": "0.0001",
            "minSz": "0.0001"
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(market.id, "BTC-USDT");
        assert_eq!(market.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
        assert_eq!(market.market_type, MarketType::Spot);
    }
}

//! Market data parser for Bybit.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{Market, MarketLimits, MarketPrecision, MarketType, MinMax, Symbol},
};
use serde_json::Value;

/// Parse market data from Bybit exchange info.
///
/// Bybit uses `symbol` for instrument ID (e.g., "BTCUSDT").
///
/// # Arguments
///
/// * `data` - Bybit market data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure.
pub fn parse_market(data: &Value) -> Result<Market> {
    // Bybit uses "symbol" for the exchange-specific ID (e.g., "BTCUSDT")
    let id = data["symbol"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("symbol")))?
        .to_string();

    // Base and quote currencies
    let base = data["baseCoin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("baseCoin")))?
        .to_string();

    let quote = data["quoteCoin"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("quoteCoin")))?
        .to_string();

    // Contract type - Bybit uses "contractType" for derivatives
    // Option uses "optionsType" (Call/Put) instead of "contractType"
    let contract_type = data["contractType"].as_str();
    let options_type = data["optionsType"].as_str();

    // Determine market type based on contractType and optionsType
    let market_type = if options_type.is_some() {
        // Option market: has "optionsType" field (Call/Put)
        MarketType::Option
    } else {
        match contract_type {
            Some("LinearPerpetual" | "InversePerpetual") => MarketType::Swap,
            Some("LinearFutures" | "InverseFutures") => MarketType::Futures,
            _ => MarketType::Spot,
        }
    };

    // Market status
    let status = data["status"].as_str().unwrap_or("Trading");
    let active = status == "Trading";

    // Parse precision - Bybit uses tickSize and basePrecision
    let price_precision = parse_decimal(data, "priceFilter").or_else(|| {
        data.get("priceFilter")
            .and_then(|pf| parse_decimal(pf, "tickSize"))
    });
    let amount_precision = parse_decimal(data, "lotSizeFilter").or_else(|| {
        data.get("lotSizeFilter")
            .and_then(|lf| parse_decimal(lf, "basePrecision"))
    });

    // Parse limits from lotSizeFilter
    let (min_amount, max_amount) = if let Some(lot_filter) = data.get("lotSizeFilter") {
        (
            parse_decimal(lot_filter, "minOrderQty"),
            parse_decimal(lot_filter, "maxOrderQty"),
        )
    } else {
        (None, None)
    };

    // Contract-specific fields
    let contract = market_type != MarketType::Spot;
    let linear = if contract {
        Some(contract_type == Some("LinearPerpetual") || contract_type == Some("LinearFutures"))
    } else {
        None
    };
    let inverse = if contract {
        Some(contract_type == Some("InversePerpetual") || contract_type == Some("InverseFutures"))
    } else {
        None
    };
    let contract_size = parse_decimal(data, "contractSize");

    // Settlement currency for derivatives
    let settle = data["settleCoin"].as_str().map(ToString::to_string);
    let settle_id = settle.clone();

    // Expiry for futures/options
    let expiry = parse_timestamp(data, "deliveryTime");
    let expiry_datetime = expiry.and_then(ccxt_core::parser_utils::timestamp_to_datetime);

    // Option-specific fields
    let option_type = options_type.map(|s| s.to_string());
    let strike = if market_type == MarketType::Option {
        // For options, extract strike price from symbol if available
        // Bybit option symbol format: BTC-29DEC23-40000-C
        // Try to parse strike from symbol
        id.split('-')
            .nth(2)
            .and_then(|strike_str| parse_decimal(&serde_json::json!(strike_str), ""))
    } else {
        None
    };

    // Build unified symbol format based on market type:
    // - Spot: BASE/QUOTE (e.g., "BTC/USDT")
    // - Swap: BASE/QUOTE:SETTLE (e.g., "BTC/USDT:USDT")
    // - Futures: BASE/QUOTE:SETTLE-YYMMDD (e.g., "BTC/USDT:USDT-241231")
    let symbol = match market_type {
        MarketType::Swap => {
            if let Some(ref s) = settle {
                format!("{}/{}:{}", base, quote, s)
            } else if linear == Some(true) {
                // Linear swaps settle in quote currency
                format!("{}/{}:{}", base, quote, quote)
            } else {
                // Inverse swaps settle in base currency
                format!("{}/{}:{}", base, quote, base)
            }
        }
        MarketType::Futures => {
            let settle_ccy = settle.clone().unwrap_or_else(|| {
                if linear == Some(true) {
                    quote.clone()
                } else {
                    base.clone()
                }
            });
            if let Some(exp_ts) = expiry {
                // Convert timestamp to YYMMDD format
                if let Some(dt) = chrono::DateTime::from_timestamp_millis(exp_ts) {
                    let year = (dt.format("%y").to_string().parse::<u8>()).unwrap_or(0);
                    let month = (dt.format("%m").to_string().parse::<u8>()).unwrap_or(1);
                    let day = (dt.format("%d").to_string().parse::<u8>()).unwrap_or(1);
                    format!(
                        "{}/{}:{}-{:02}{:02}{:02}",
                        base, quote, settle_ccy, year, month, day
                    )
                } else {
                    format!("{}/{}:{}", base, quote, settle_ccy)
                }
            } else {
                format!("{}/{}:{}", base, quote, settle_ccy)
            }
        }
        _ => format!("{}/{}", base, quote), // Spot and Option
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
        margin: contract,
        contract: Some(contract),
        linear,
        inverse,
        contract_size,
        expiry,
        expiry_datetime,
        strike,
        option_type,
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
        maker: parse_decimal(data, "makerFeeRate"),
        taker: parse_decimal(data, "takerFeeRate"),
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
    fn test_parse_market() {
        let data = json!({
            "symbol": "BTCUSDT",
            "baseCoin": "BTC",
            "quoteCoin": "USDT",
            "status": "Trading",
            "lotSizeFilter": {
                "basePrecision": "0.0001",
                "minOrderQty": "0.0001",
                "maxOrderQty": "100"
            },
            "priceFilter": {
                "tickSize": "0.01"
            }
        });

        let market = parse_market(&data).unwrap();
        assert_eq!(market.id, "BTCUSDT");
        assert_eq!(market.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
        assert_eq!(market.market_type, MarketType::Spot);
    }
}

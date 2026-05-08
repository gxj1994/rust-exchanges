//! Market data parser for HyperLiquid.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::value_to_hashmap,
    types::{Market, MarketLimits, MarketPrecision, MarketType, MinMax, Symbol},
};
use rust_decimal::Decimal;

/// Parse market data from HyperLiquid meta response.
///
/// # Arguments
///
/// * `data` - HyperLiquid market data JSON object
/// * `index` - Market index (used as ID)
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure.
pub fn parse_market(data: &serde_json::Value, index: usize) -> Result<Market> {
    let name = data["name"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("name")))?;

    // HyperLiquid uses format like "BTC" for the asset name
    // We convert to unified format: "BTC/USDC:USDC"
    let symbol = format!("{}/USDC:USDC", name);
    let id = index.to_string();

    // Parse size decimals for precision
    let sz_decimals = data["szDecimals"].as_u64().unwrap_or(4) as u32;
    let amount_precision = Decimal::new(1, sz_decimals);

    // Price precision (HyperLiquid uses 5 significant figures typically)
    let price_precision = Decimal::new(1, 5);

    // Parse the symbol to get structured representation
    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&symbol).ok();

    Ok(Market {
        id,
        symbol: Symbol::new_unchecked(&symbol),
        parsed_symbol,
        base: name.to_string(),
        quote: "USDC".to_string(),
        settle: Some("USDC".to_string()),
        base_id: Some(name.to_string()),
        quote_id: Some("USDC".to_string()),
        settle_id: Some("USDC".to_string()),
        market_type: MarketType::Swap,
        active: true,
        margin: true,
        contract: Some(true),
        linear: Some(true),
        inverse: Some(false),
        contract_size: Some(Decimal::ONE),
        expiry: None,
        expiry_datetime: None,
        strike: None,
        option_type: None,
        precision: MarketPrecision {
            price: Some(price_precision),
            amount: Some(amount_precision),
            base: None,
            quote: None,
        },
        limits: MarketLimits {
            amount: Some(MinMax {
                min: Some(amount_precision),
                max: None,
            }),
            price: None,
            cost: Some(MinMax {
                min: Some(Decimal::new(10, 0)), // $10 minimum
                max: None,
            }),
            leverage: Some(MinMax {
                min: Some(Decimal::ONE),
                max: Some(Decimal::new(50, 0)),
            }),
        },
        maker: Some(Decimal::new(2, 4)), // 0.02%
        taker: Some(Decimal::new(5, 4)), // 0.05%
        percentage: Some(true),
        tier_based: Some(true),
        fee_side: Some("quote".to_string()),
        info: value_to_hashmap(data),
    })
}

/// Parse spot market data from HyperLiquid spotMeta response.
///
/// # Arguments
///
/// * `data` - HyperLiquid spot market data JSON object
/// * `index` - Market index (used as ID)
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure for spot markets.
///
/// # Symbol Format
///
/// Spot markets use BASE/QUOTE format (no settle):
/// - `PURR/USDC`
/// - `HYPE/USDC`
///
/// # Asset Index
///
/// According to HyperLiquid API docs:
/// For spot assets, use 10000 + index where index is the corresponding index in spotMeta.universe.
pub fn parse_spot_market(data: &serde_json::Value, index: usize) -> Result<Market> {
    let name = data["name"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("name")))?;

    // Spot market symbol: "PURR/USDC" format
    let symbol = name.to_string();

    // IMPORTANT: HyperLiquid requires 10000 + index for spot assets
    // This is used in order placement and other trading operations
    let asset_index = 10000 + index;
    let id = asset_index.to_string(); // Store as "10000", "10001", etc.

    // Extract base and quote from symbol (e.g., "PURR/USDC" -> base="PURR", quote="USDC")
    let (base, quote) = if symbol.contains('/') {
        let parts: Vec<&str> = symbol.split('/').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (symbol.clone(), "USDC".to_string())
        }
    } else {
        // If no '/' found, check if it's a token pair from universe
        // Some spot markets may have different format
        (symbol.clone(), "USDC".to_string())
    };

    // Parse decimals
    let decimals = data["decimals"].as_u64().unwrap_or(6); // Default 6 decimals for spot
    let amount_precision = Decimal::new(1, decimals as u32);

    // Price precision
    let price_precision = Decimal::new(1, 8); // 8 decimals for spot prices

    // Parse the symbol to get structured representation
    let parsed_symbol = ccxt_core::symbol::SymbolParser::parse(&symbol).ok();

    Ok(Market {
        id,
        symbol: Symbol::new_unchecked(&symbol),
        parsed_symbol,
        base: base.clone(),
        quote: quote.clone(),
        settle: None, // Spot markets don't have settle
        base_id: Some(base),
        quote_id: Some(quote),
        settle_id: None,
        market_type: MarketType::Spot,
        active: true,
        margin: false, // Spot doesn't have margin
        contract: Some(false),
        linear: None,
        inverse: None,
        contract_size: None,
        expiry: None,
        expiry_datetime: None,
        strike: None,
        option_type: None,
        precision: MarketPrecision {
            price: Some(price_precision),
            amount: Some(amount_precision),
            base: None,
            quote: None,
        },
        limits: MarketLimits {
            amount: Some(MinMax {
                min: Some(amount_precision),
                max: None,
            }),
            price: None,
            cost: Some(MinMax {
                min: Some(Decimal::new(10, 0)), // $10 minimum
                max: None,
            }),
            leverage: None, // Spot doesn't have leverage
        },
        maker: Some(Decimal::new(2, 4)), // 0.02%
        taker: Some(Decimal::new(5, 4)), // 0.05%
        percentage: Some(true),
        tier_based: Some(true),
        fee_side: Some("quote".to_string()),
        info: value_to_hashmap(data),
    })
}

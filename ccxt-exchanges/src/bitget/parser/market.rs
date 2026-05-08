//! Market data parser for Bitget.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_decimal, value_to_hashmap},
    types::{Market, MarketLimits, MarketPrecision, MarketType, MinMax, Symbol},
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse market data from Bitget exchange info.
///
/// # Arguments
///
/// * `data` - Bitget market data JSON object
///
/// # Returns
///
/// Returns a CCXT [`Market`] structure.
pub fn parse_market(data: &Value) -> Result<Market> {
    // Bitget uses "symbol" for the exchange-specific ID (e.g., "BTCUSDT" for spot, "BTCUSDT" for swap)
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

    // Market status
    let status = data["status"].as_str().unwrap_or("online");
    let active = status == "online";

    // 根据 category 判断市场类型
    // SPOT: 现货, MARGIN: 杠杆, USDT-FUTURES: U本位合约, COIN-FUTURES: 币本位合约, USDC-FUTURES: USDC合约
    let category = data["category"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("category")))?
        .to_string();

    let is_contract = matches!(
        category.as_str(),
        "USDT-FUTURES" | "COIN-FUTURES" | "USDC-FUTURES"
    );

    // Determine settle currency, market type, and build unified symbol
    let (symbol, settle, settle_id, market_type, contract, linear, inverse, contract_size) =
        if is_contract {
            // 合约市场: 根据 category 判断结算币种
            // USDT-FUTURES: 线性合约, USDT 结算
            // COIN-FUTURES: 反向合约, 基础币种结算
            // USDC-FUTURES: 线性合约, USDC 结算
            let is_linear = category == "USDT-FUTURES" || category == "USDC-FUTURES";

            // 结算币种: 线性合约为 quote, 反向合约为 base
            let settle_coin = if is_linear {
                quote.clone()
            } else {
                base.clone()
            };

            // Build unified symbol
            // Contract: BASE/QUOTE:SETTLE (e.g., BTC/USDT:USDT or BTC/USD:BTC)
            let symbol = format!("{}/{}:{}", base, quote, settle_coin);

            // Parse contract size/multiplier from quantityMultiplier field
            let contract_size = parse_decimal(data, "quantityMultiplier");

            (
                symbol,
                Some(settle_coin.clone()),
                Some(settle_coin),
                MarketType::Swap,
                Some(true),
                Some(is_linear),
                Some(!is_linear),
                contract_size,
            )
        } else {
            // Spot or Margin market
            let symbol = format!("{}/{}", base, quote);
            (
                symbol,
                None,
                None,
                MarketType::Spot,
                Some(false),
                None,
                None,
                None,
            )
        };

    // Parse precision
    // 新API使用 pricePrecision 和 quantityPrecision 表示小数位数
    let price_precision = parse_decimal(data, "pricePrecision").map(|p| {
        // Convert decimal places to tick size (e.g., 2 -> 0.01)
        if p.is_integer() {
            let places = p.to_string().parse::<i32>().unwrap_or(0);
            Decimal::new(1, places as u32)
        } else {
            p
        }
    });

    let amount_precision = parse_decimal(data, "quantityPrecision").map(|p| {
        if p.is_integer() {
            let places = p.to_string().parse::<i32>().unwrap_or(0);
            Decimal::new(1, places as u32)
        } else {
            p
        }
    });

    // Parse limits
    // 新API使用 minOrderQty, maxOrderQty, minOrderAmount
    let min_amount = parse_decimal(data, "minOrderQty");
    let max_amount = parse_decimal(data, "maxOrderQty")
        .and_then(|v| if v == Decimal::ZERO { None } else { Some(v) });
    let min_cost = parse_decimal(data, "minOrderAmount");

    // Parse fees
    let maker_fee = parse_decimal(data, "makerFeeRate");
    let taker_fee = parse_decimal(data, "takerFeeRate");

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
        margin: category == "MARGIN",
        contract,
        linear,
        inverse,
        contract_size,
        expiry: None,
        expiry_datetime: None,
        strike: None,
        option_type: None,
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
            cost: Some(MinMax {
                min: min_cost,
                max: None,
            }),
            leverage: None,
        },
        maker: maker_fee,
        taker: taker_fee,
        percentage: Some(true),
        tier_based: Some(false),
        fee_side: Some("quote".to_string()),
        info: value_to_hashmap(data),
    })
}

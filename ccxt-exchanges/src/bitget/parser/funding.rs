//! Funding rate data parser for Bitget.

use ccxt_core::{
    Result,
    parser_utils::parse_timestamp,
    types::{FundingRate, FundingRateHistory},
};
use serde_json::Value;

use super::{parse_f64_field, timestamp_to_datetime};

/// Parse funding rate data from Bitget response.
pub fn parse_funding_rate(data: &Value, symbol: &str) -> Result<FundingRate> {
    let funding_rate = parse_f64_field(data, "fundingRate");
    // V3 uses nextUpdate for next funding time
    let funding_time = parse_timestamp(data, "nextUpdate")
        .or_else(|| parse_timestamp(data, "fundingTime"))
        .or_else(|| parse_timestamp(data, "nextFundingTime"));

    let timestamp = parse_timestamp(data, "ts")
        .or_else(|| parse_timestamp(data, "fundingTime"))
        .or_else(|| parse_timestamp(data, "nextUpdate"));
    let datetime = timestamp.and_then(timestamp_to_datetime);
    let funding_datetime = funding_time.and_then(timestamp_to_datetime);

    Ok(FundingRate {
        info: data.clone(),
        symbol: symbol.to_string(),
        mark_price: parse_f64_field(data, "markPrice"),
        index_price: parse_f64_field(data, "indexPrice"),
        interest_rate: None,
        estimated_settle_price: None,
        funding_rate,
        funding_timestamp: funding_time,
        funding_datetime,
        previous_funding_rate: None,
        previous_funding_timestamp: None,
        previous_funding_datetime: None,
        timestamp,
        datetime,
    })
}

/// Parse funding rate history data from Bitget response.
pub fn parse_funding_rate_history(data: &Value, symbol: &str) -> Result<FundingRateHistory> {
    let funding_rate = parse_f64_field(data, "fundingRate");
    // V3 uses fundingRateTimestamp instead of fundingTime
    let timestamp = parse_timestamp(data, "fundingRateTimestamp")
        .or_else(|| parse_timestamp(data, "fundingTime"))
        .or_else(|| parse_timestamp(data, "settleTime"));
    let datetime = timestamp.and_then(timestamp_to_datetime);

    Ok(FundingRateHistory {
        info: data.clone(),
        symbol: symbol.to_string(),
        funding_rate,
        timestamp,
        datetime,
    })
}

//! Funding rate parser for OKX.

use ccxt_core::{Result, parser_utils::parse_timestamp};
use serde_json::Value;

/// Parse funding rate data from OKX public funding-rate response.
///
/// OKX funding rate fields:
/// - instId: instrument ID
/// - fundingRate: current funding rate
/// - fundingTime: next funding time
/// - nextFundingRate: estimated next funding rate
/// - nextFundingTime: next funding settlement time
///
/// # Arguments
///
/// * `data` - OKX funding rate data JSON object
/// * `symbol` - Unified symbol string
///
/// # Returns
///
/// Returns a CCXT [`FundingRate`] structure.
pub fn parse_funding_rate(data: &Value, symbol: &str) -> Result<ccxt_core::types::FundingRate> {
    let funding_rate = parse_f64_field(data, "fundingRate");
    let funding_time = parse_timestamp(data, "fundingTime");
    let next_funding_time = parse_timestamp(data, "nextFundingTime");

    let timestamp = funding_time.or_else(|| parse_timestamp(data, "ts"));
    let datetime = timestamp.and_then(ccxt_core::parser_utils::timestamp_to_datetime);

    let funding_datetime =
        next_funding_time.and_then(ccxt_core::parser_utils::timestamp_to_datetime);

    Ok(ccxt_core::types::FundingRate {
        info: data.clone(),
        symbol: symbol.to_string(),
        mark_price: parse_f64_field(data, "markPx"),
        index_price: parse_f64_field(data, "idxPx"),
        interest_rate: None,
        estimated_settle_price: None,
        funding_rate,
        funding_timestamp: next_funding_time,
        funding_datetime,
        previous_funding_rate: None,
        previous_funding_timestamp: None,
        previous_funding_datetime: None,
        timestamp,
        datetime,
    })
}

/// Parse funding rate history data from OKX public funding-rate-history response.
///
/// OKX funding rate history fields:
/// - instId: instrument ID
/// - fundingRate: historical funding rate
/// - fundingTime: funding settlement time
/// - realizedRate: realized funding rate
///
/// # Arguments
///
/// * `data` - OKX funding rate history data JSON object
/// * `symbol` - Unified symbol string
///
/// # Returns
///
/// Returns a CCXT [`FundingRateHistory`] structure.
pub fn parse_funding_rate_history(
    data: &Value,
    symbol: &str,
) -> Result<ccxt_core::types::FundingRateHistory> {
    let funding_rate =
        parse_f64_field(data, "fundingRate").or_else(|| parse_f64_field(data, "realizedRate"));
    let timestamp = parse_timestamp(data, "fundingTime");
    let datetime = timestamp.and_then(ccxt_core::parser_utils::timestamp_to_datetime);

    Ok(ccxt_core::types::FundingRateHistory {
        info: data.clone(),
        symbol: symbol.to_string(),
        funding_rate,
        timestamp,
        datetime,
    })
}

/// Helper to parse a string field as f64.
pub(super) fn parse_f64_field(data: &Value, field: &str) -> Option<f64> {
    data[field]
        .as_str()
        .and_then(|s| {
            if s.is_empty() {
                None
            } else {
                s.parse::<f64>().ok()
            }
        })
        .or_else(|| data[field].as_f64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_funding_rate() {
        let data = json!({
            "instId": "BTC-USDT-SWAP",
            "fundingRate": "0.0001",
            "fundingTime": "1700000000000",
            "nextFundingRate": "0.00015",
            "nextFundingTime": "1700028800000"
        });

        let rate = parse_funding_rate(&data, "BTC/USDT:USDT").unwrap();
        assert_eq!(rate.symbol, "BTC/USDT:USDT");
        assert_eq!(rate.funding_rate, Some(0.0001));
        assert_eq!(rate.funding_timestamp, Some(1700028800000));
        assert_eq!(rate.timestamp, Some(1700000000000));
    }

    #[test]
    fn test_parse_funding_rate_history() {
        let data = json!({
            "instId": "BTC-USDT-SWAP",
            "fundingRate": "0.0001",
            "fundingTime": "1700000000000",
            "realizedRate": "0.00009"
        });

        let history = parse_funding_rate_history(&data, "BTC/USDT:USDT").unwrap();
        assert_eq!(history.symbol, "BTC/USDT:USDT");
        assert_eq!(history.funding_rate, Some(0.0001));
        assert_eq!(history.timestamp, Some(1700000000000));
    }

    #[test]
    fn test_parse_f64_field() {
        let data = json!({
            "a": "123.45",
            "b": "",
            "c": 67.89,
            "d": null
        });

        assert_eq!(parse_f64_field(&data, "a"), Some(123.45));
        assert_eq!(parse_f64_field(&data, "b"), None);
        assert_eq!(parse_f64_field(&data, "c"), Some(67.89));
        assert_eq!(parse_f64_field(&data, "d"), None);
        assert_eq!(parse_f64_field(&data, "missing"), None);
    }
}

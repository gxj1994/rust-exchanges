//! Binance ledger entry parser.
//!
//! Converts Binance API account snapshot and ledger data into standardized CCXT format.

use ccxt_core::{
    Error, ParseError, Result,
    types::{LedgerDirection, LedgerEntry, LedgerEntryType},
};
use serde_json::Value;

/// Parse ledger entries from Binance account snapshot data.
///
/// Binance's `GET /sapi/v1/accountSnapshot` endpoint returns balance snapshots,
/// not true ledger entries. This function converts the snapshot data into
/// ledger entries for tracking balance changes over time.
///
/// # Arguments
///
/// * `data` - JSON response from Binance account snapshot API.
/// * `currency` - Optional currency filter.
///
/// # Returns
///
/// Returns a vector of [`LedgerEntry`] records.
pub fn parse_ledger_entries(data: &Value, currency: Option<&str>) -> Result<Vec<LedgerEntry>> {
    let mut entries = Vec::new();

    // Binance account snapshot returns data in "snapshotVos" array
    let snapshots = data["snapshotVos"]
        .as_array()
        .or_else(|| data["snapshotVos"].as_array())
        .ok_or_else(|| Error::from(ParseError::invalid_format("snapshotVos", "expected array")))?;

    for snapshot in snapshots {
        let timestamp = snapshot["updateTime"]
            .as_i64()
            .or_else(|| snapshot["timestamp"].as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();

        // Extract balance data from the snapshot
        if let Some(balances) = snapshot["data"]["balances"].as_array() {
            for balance in balances {
                let asset = balance["asset"]
                    .as_str()
                    .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?;

                // Filter by currency if specified
                if let Some(filter_currency) = currency {
                    if !asset.eq_ignore_ascii_case(filter_currency) {
                        continue;
                    }
                }

                let free = balance["free"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let locked = balance["locked"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let total = free + locked;

                // Only create ledger entry if there's a balance
                if total > 0.0 {
                    entries.push(LedgerEntry {
                        info: balance.clone(),
                        id: format!("{}-{}", asset, timestamp),
                        timestamp,
                        datetime: datetime.clone(),
                        direction: LedgerDirection::In,
                        currency: asset.to_string(),
                        amount: total,
                        type_: LedgerEntryType::Transfer,
                        account: Some("spot".to_string()),
                        reference_account: None,
                        reference_id: None,
                        before: Some(0.0),
                        after: Some(total),
                        status: Some("completed".to_string()),
                        fee: None,
                    });
                }
            }
        }

        // Handle futures account snapshots
        if let Some(assets) = snapshot["data"]["assets"].as_array() {
            for asset in assets {
                let asset_name = asset["asset"]
                    .as_str()
                    .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?;

                // Filter by currency if specified
                if let Some(filter_currency) = currency {
                    if !asset_name.eq_ignore_ascii_case(filter_currency) {
                        continue;
                    }
                }

                let wallet_balance = asset["walletBalance"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);

                if wallet_balance > 0.0 {
                    entries.push(LedgerEntry {
                        info: asset.clone(),
                        id: format!("{}-{}", asset_name, timestamp),
                        timestamp,
                        datetime: datetime.clone(),
                        direction: LedgerDirection::In,
                        currency: asset_name.to_string(),
                        amount: wallet_balance,
                        type_: LedgerEntryType::Transfer,
                        account: Some("futures".to_string()),
                        reference_account: None,
                        reference_id: None,
                        before: Some(0.0),
                        after: Some(wallet_balance),
                        status: Some("completed".to_string()),
                        fee: None,
                    });
                }
            }
        }
    }

    Ok(entries)
}

/// Parse a single ledger entry from Binance API response.
///
/// This is used for endpoints that return actual ledger/trade history data.
///
/// # Arguments
///
/// * `data` - JSON object containing ledger entry data.
///
/// # Returns
///
/// Returns a [`LedgerEntry`] record.
pub fn parse_ledger_entry(data: &Value) -> Result<LedgerEntry> {
    let id = data["id"]
        .as_str()
        .or_else(|| data["tranId"].as_str())
        .or_else(|| data["orderId"].as_str())
        .map(ToString::to_string)
        .ok_or_else(|| Error::from(ParseError::missing_field("id")))?;

    let timestamp = data["time"]
        .as_i64()
        .or_else(|| data["timestamp"].as_i64())
        .or_else(|| data["transactTime"].as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let datetime = chrono::DateTime::from_timestamp_millis(timestamp)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    let currency = data["asset"]
        .as_str()
        .or_else(|| data["currency"].as_str())
        .ok_or_else(|| Error::from(ParseError::missing_field("asset")))?
        .to_string();

    // Determine amount and direction
    let amount = data["amount"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["amount"].as_f64())
        .or_else(|| data["income"].as_str().and_then(|s| s.parse::<f64>().ok()))
        .or_else(|| data["income"].as_f64())
        .unwrap_or(0.0);

    let direction = if amount >= 0.0 {
        LedgerDirection::In
    } else {
        LedgerDirection::Out
    };

    // Determine entry type
    let type_ = parse_ledger_type(data["type"].as_str());

    let account = data["account"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| Some("spot".to_string()));

    let reference_id = data["orderId"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| data["tradeId"].as_str().map(ToString::to_string));

    let status = data["status"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| Some("completed".to_string()));

    let fee = data["fee"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| data["fee"].as_f64());

    Ok(LedgerEntry {
        info: data.clone(),
        id,
        timestamp,
        datetime,
        direction,
        currency,
        amount: amount.abs(),
        type_,
        account,
        reference_account: None,
        reference_id,
        before: None,
        after: None,
        status,
        fee,
    })
}

/// Parse ledger entry type from string.
fn parse_ledger_type(type_str: Option<&str>) -> LedgerEntryType {
    match type_str {
        Some(t) => match t.to_lowercase().as_str() {
            "trade" | "order" => LedgerEntryType::Trade,
            "fee" | "commission" | "trading_fee" => LedgerEntryType::Fee,
            "deposit" => LedgerEntryType::Deposit,
            "withdrawal" | "withdraw" => LedgerEntryType::Withdrawal,
            "transfer" => LedgerEntryType::Transfer,
            "settlement" | "funding_fee" | "insurance_clear" => LedgerEntryType::Settlement,
            "rebate" | "commission_rebate" => LedgerEntryType::Rebate,
            "referral" => LedgerEntryType::Referral,
            "cashback" => LedgerEntryType::Cashback,
            _ => LedgerEntryType::Transfer,
        },
        None => LedgerEntryType::Transfer,
    }
}

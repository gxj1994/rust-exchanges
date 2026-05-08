//! Binance 账户模块测试
//!
//! 测试覆盖：
//! - 多账户类型余额查询
//! - 内部转账
//! - 最大可借/可转金额

use crate::support::{create_binance_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::error::Result;
use ccxt_core::types::AccountType;
use ccxt_exchanges::binance::Binance;

/// Create Binance client for account tests using shared helper.
fn create_binance_client() -> Option<Binance> {
    let config = init_test();
    if should_skip_private_tests("binance") {
        return None;
    }
    create_binance_with_credentials(&config).ok()
}

/// Macro to skip test if no credentials available
macro_rules! skip_if_no_credentials {
    ($client:expr) => {
        if $client.is_none() {
            println!("SKIPPED: No Binance credentials configured");
            return Ok(());
        }
    };
}

#[tokio::test]
async fn test_fetch_spot_balance() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let binance = binance.unwrap();

    let balance = binance.fetch_balance(Some(AccountType::Spot)).await?;

    assert!(
        !balance.balances.is_empty(),
        "Balance data should not be empty"
    );

    println!(
        "Spot balance query successful, currencies: {}",
        balance.balances.len()
    );

    if let Some(btc) = balance.balances.get("BTC") {
        assert!(btc.total >= btc.used + btc.free);
        println!(
            "BTC balance - total: {}, free: {}, used: {}",
            btc.total, btc.free, btc.used
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_margin_balance() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let binance = binance.unwrap();

    match binance.fetch_balance(Some(AccountType::Margin)).await {
        Ok(balance) => {
            println!(
                "Margin account balance query successful, currencies: {}",
                balance.balances.len()
            );
        }
        Err(e) => {
            // Margin account may not be enabled for this API key
            println!(
                "Margin balance query skipped: {} (margin account may not be enabled)",
                e
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_isolated_margin_balance() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let binance = binance.unwrap();

    match binance
        .fetch_balance(Some(AccountType::IsolatedMargin))
        .await
    {
        Ok(balance) => {
            println!(
                "Isolated margin account balance query successful, currencies: {}",
                balance.balances.len()
            );
        }
        Err(e) => {
            // Isolated margin may not be enabled
            println!(
                "Isolated margin balance query skipped: {} (isolated margin may not be enabled)",
                e
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_future_balance() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let binance = binance.unwrap();

    match binance.fetch_balance(Some(AccountType::Futures)).await {
        Ok(balance) => {
            println!(
                "USDT-margined futures account balance query successful, currencies: {}",
                balance.balances.len()
            );
        }
        Err(e) => {
            // Futures permission may not be enabled for this API key
            println!(
                "Futures balance query skipped: {} (futures permission may not be enabled in API settings)",
                e
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch_funding_balance() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let binance = binance.unwrap();

    match binance.fetch_balance(Some(AccountType::Funding)).await {
        Ok(balance) => {
            println!(
                "Funding account balance query successful, currencies: {}",
                balance.balances.len()
            );
        }
        Err(e) => {
            // Funding account may not exist or be empty
            println!(
                "Funding balance query skipped: {} (funding account may not be available)",
                e
            );
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "transfer method not yet migrated to new modular structure"]
async fn test_internal_transfer() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    // TODO: Implement when transfer is available
    Ok(())
}

#[tokio::test]
#[ignore = "fetch_transfers method not yet migrated to new modular structure"]
async fn test_fetch_transfers() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    // TODO: Implement when fetch_transfers is available
    Ok(())
}

#[tokio::test]
#[ignore = "fetch_cross_margin_max_borrowable method not yet migrated to new modular structure"]
async fn test_fetch_cross_margin_max_borrowable() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    // TODO: Implement when fetch_cross_margin_max_borrowable is available
    Ok(())
}

#[tokio::test]
#[ignore = "fetch_isolated_margin_max_borrowable method not yet migrated to new modular structure"]
async fn test_fetch_isolated_margin_max_borrowable() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    // TODO: Implement when fetch_isolated_margin_max_borrowable is available
    Ok(())
}

#[tokio::test]
#[ignore = "fetch_max_transferable method not yet migrated to new modular structure"]
async fn test_fetch_max_transferable() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    // TODO: Implement when fetch_max_transferable is available
    Ok(())
}

#[tokio::test]
async fn test_multiple_account_types() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let binance = binance.unwrap();

    let account_types = vec![
        AccountType::Spot,
        AccountType::Margin,
        AccountType::Futures,
        AccountType::Funding,
    ];

    for account_type in account_types {
        match binance.fetch_balance(Some(account_type)).await {
            Ok(balance) => {
                println!(
                    "{} account balance query successful, currencies: {}",
                    account_type,
                    balance.balances.len()
                );
            }
            Err(e) => {
                println!(
                    "{} account query skipped: {} (account type may not be enabled)",
                    account_type, e
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test complete account management workflow.
    #[tokio::test]
    #[ignore = "Account management methods not yet fully migrated to new modular structure"]
    async fn test_complete_account_workflow() -> Result<()> {
        let binance = create_binance_client();
        skip_if_no_credentials!(binance);
        // TODO: Implement when all account management methods are available
        Ok(())
    }
}

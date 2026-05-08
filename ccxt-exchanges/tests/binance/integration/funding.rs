//! Binance 资金模块测试
//!
//! 测试覆盖：
//! - 充值地址查询
//! - 提现功能
//! - 内部转账
//! - 资金费率
use crate::support::{create_binance_with_credentials, init_test, should_skip_private_tests};
use ccxt_core::error::Result;
use ccxt_exchanges::binance::Binance;

/// Create Binance client for funding tests
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
async fn test_fetch_deposit_address() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let _binance = binance.unwrap();

    // Note: fetch_deposit_address method may not be fully implemented
    println!("Deposit address test - placeholder");
    Ok(())
}

#[tokio::test]
async fn test_fetch_funding_rate() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let _binance = binance.unwrap();

    // Note: fetch_funding_rate method may not be fully implemented
    println!("Funding rate test - placeholder");
    Ok(())
}

#[tokio::test]
async fn test_fetch_funding_rate_history() -> Result<()> {
    let binance = create_binance_client();
    skip_if_no_credentials!(binance);
    let _binance = binance.unwrap();

    // Note: fetch_funding_rate_history method may not be fully implemented
    println!("Funding rate history test - placeholder");
    Ok(())
}

#[tokio::test]
#[ignore = "transfer method not yet migrated to new modular structure"]
async fn test_transfer() -> Result<()> {
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

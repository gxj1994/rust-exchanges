//! Shared support utilities for `ccxt-exchanges` tests.
//!
//! This is the stable import surface for exchange integration helpers.

#![allow(clippy::disallowed_methods)]

use anyhow::{Context, Result as AnyhowResult};
use ccxt_core::ExchangeConfig;
use ccxt_core::types::common::default_type::DefaultSubType;
use ccxt_exchanges::binance::Binance;
use ccxt_exchanges::gate::{Gate, GateBuilder};

pub mod config;
use ccxt_exchanges::bitget::{Bitget, BitgetBuilder};
use ccxt_exchanges::bybit::{Bybit, BybitBuilder};
use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidBuilder};
use ccxt_exchanges::okx::{Okx, OkxBuilder};
pub use config::{ConfigError, TestConfig};

/// Initialize test configuration from `.env` and environment variables.
pub fn init_test() -> TestConfig {
    dotenvy::dotenv().ok();
    TestConfig::from_env().unwrap_or_default()
}

/// Check whether private tests should be skipped for a given exchange.
pub fn should_skip_private_tests(exchange: &str) -> bool {
    let config = init_test();
    if config.should_skip_private_tests() {
        return true;
    }

    let has_credentials = match exchange.to_lowercase().as_str() {
        "binance" => config.has_binance_credentials(),
        "okx" => config.has_okx_credentials(),
        "bybit" => config.has_bybit_credentials(),
        "hyperliquid" => config.has_hyperliquid_credentials(),
        "bitget" => config.has_bitget_credentials(),
        "gate" => config.has_gate_credentials(),
        _ => false,
    };

    !has_credentials
}

/// Create a public Binance client for tests.
pub fn create_binance(config: &TestConfig) -> AnyhowResult<Binance> {
    let exchange_config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        sandbox: config.binance.use_testnet,
        api_key: None,
        secret: None,
        ..Default::default()
    };

    Binance::new(exchange_config).context("Failed to create Binance instance")
}

/// Create an authenticated Binance client for tests.
pub fn create_binance_with_credentials(config: &TestConfig) -> AnyhowResult<Binance> {
    let (api_key, api_secret) = config
        .get_active_api_key("binance")
        .context("No Binance credentials configured")?;

    let exchange_config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        sandbox: config.binance.use_testnet,
        api_key: Some(ccxt_core::SecretString::new(api_key)),
        secret: Some(ccxt_core::SecretString::new(api_secret)),
        ..Default::default()
    };

    Binance::new(exchange_config).context("Failed to create Binance instance with credentials")
}

/// Create an authenticated Binance spot client for tests.
pub fn create_binance_spot_with_credentials(config: &TestConfig) -> AnyhowResult<Binance> {
    create_binance_with_credentials(config)
}

// ============================================================================
// Gate Helpers
// ============================================================================

/// Create a public Gate client for tests.
pub fn create_gate(config: &TestConfig) -> AnyhowResult<Gate> {
    let mut builder = GateBuilder::new();

    if config.gate.use_testnet {
        builder = builder.sandbox(true);
    }

    builder.build().context("Failed to create Gate instance")
}

/// Create an authenticated Gate client for tests.
pub fn create_gate_with_credentials(config: &TestConfig) -> AnyhowResult<Gate> {
    let (api_key, api_secret) = config
        .get_active_api_key("gate")
        .context("No Gate credentials configured")?;

    println!("[DEBUG] Gate use_testnet: {}", config.gate.use_testnet);

    let mut builder = GateBuilder::new().api_key(api_key).secret(api_secret);

    if config.gate.use_testnet {
        builder = builder.sandbox(true);
    }

    builder
        .build()
        .context("Failed to create Gate instance with credentials")
}

/// Create an authenticated Gate spot client for tests.
pub fn create_gate_spot_with_credentials(config: &TestConfig) -> AnyhowResult<Gate> {
    create_gate_with_credentials(config)
}

/// Create an authenticated Binance swap (U本位合约) client for tests.
pub fn create_binance_swap_with_credentials(config: &TestConfig) -> AnyhowResult<Binance> {
    let (api_key, api_secret) = config
        .get_active_api_key("binance")
        .context("No Binance credentials configured")?;

    let exchange_config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        sandbox: config.binance.use_testnet,
        api_key: Some(ccxt_core::SecretString::new(api_key)),
        secret: Some(ccxt_core::SecretString::new(api_secret)),
        ..Default::default()
    };

    Binance::new_swap(exchange_config)
        .context("Failed to create Binance swap instance with credentials")
}

/// Create a public OKX client for tests.
pub fn create_okx(config: &TestConfig) -> AnyhowResult<Okx> {
    OkxBuilder::new()
        .sandbox(config.okx.use_testnet)
        .build()
        .context("Failed to create OKX instance")
}

/// Create an authenticated OKX client for tests.
pub fn create_okx_with_credentials(config: &TestConfig, default_type: &str) -> AnyhowResult<Okx> {
    let (api_key, api_secret) = config
        .get_active_api_key("okx")
        .context("No OKX credentials configured")?;

    let passphrase = std::env::var("OKX_PASSPHRASE").context("OKX_PASSPHRASE not set")?;

    let mut builder = OkxBuilder::new()
        .sandbox(config.okx.use_testnet)
        .api_key(api_key)
        .secret(api_secret)
        .passphrase(passphrase)
        .default_type(default_type);

    // 根据交易类型设置 account_mode (对应 OKX 的 tdMode 参数)
    // - spot: cash (现货)
    // - swap/futures: cross (全仓合约)
    builder = match default_type {
        "spot" => builder.account_mode("cash"),
        "swap" | "futures" => builder.account_mode("cross"),
        _ => builder.account_mode("cash"), // 默认现货
    };

    builder
        .build()
        .context("Failed to create OKX instance with credentials")
}

/// Create a public Bybit client for tests.
pub fn create_bybit(config: &TestConfig) -> AnyhowResult<Bybit> {
    BybitBuilder::new()
        .testnet(config.bybit.use_testnet)
        .build()
        .context("Failed to create Bybit instance")
}

/// Create an authenticated Bybit client for tests.
pub fn create_bybit_with_credentials(
    config: &TestConfig,
    market_type: &str,
) -> AnyhowResult<Bybit> {
    let (api_key, api_secret) = config
        .get_active_api_key("bybit")
        .context("No Bybit credentials configured")?;

    let mut builder = BybitBuilder::new()
        .testnet(config.bybit.use_testnet)
        .api_key(api_key)
        .secret(api_secret);
    // Set market type (spot or swap)
    if market_type == "swap" {
        // For swap, set both default_type and default_sub_type to ensure correct category (linear)
        builder = builder
            .default_type("swap")
            .default_sub_type(DefaultSubType::Linear);
    }
    // Default is spot, so no need to set for "spot"

    builder
        .build()
        .context("Failed to create Bitget instance with credentials")
}

/// Create a public HyperLiquid client for tests.
pub fn create_hyperliquid(config: &TestConfig) -> AnyhowResult<HyperLiquid> {
    HyperLiquidBuilder::new()
        .testnet(config.hyperliquid.use_testnet)
        .build()
        .context("Failed to create HyperLiquid instance")
}

/// Create an authenticated HyperLiquid client for tests.
pub fn create_hyperliquid_with_credentials(
    config: &TestConfig,
    default_type: &str,
) -> AnyhowResult<HyperLiquid> {
    let private_key =
        std::env::var("HYPERLIQUID_PRIVATE_KEY").context("HYPERLIQUID_PRIVATE_KEY not set")?;

    HyperLiquidBuilder::new()
        .testnet(config.hyperliquid.use_testnet)
        .private_key(&private_key)
        .default_type(default_type)
        .build()
        .context("Failed to create HyperLiquid instance with credentials")
}

/// Create a public Bitget client for tests.
pub fn create_bitget(config: &TestConfig) -> AnyhowResult<Bitget> {
    BitgetBuilder::new()
        .sandbox(config.bitget.use_testnet)
        .build()
        .context("Failed to create Bitget instance")
}

/// Create an authenticated Bitget client for tests.
///
/// # Arguments
/// * `config` - Test configuration
/// * `market_type` - Market type: "spot" for spot trading, "swap" for contract/futures trading
///
/// # Examples
/// ```
/// // Create spot client
/// let spot_client = create_bitget_with_credentials(&config, "spot")?;
///
/// // Create contract client
/// let contract_client = create_bitget_with_credentials(&config, "swap")?;
/// ```
pub fn create_bitget_with_credentials(
    config: &TestConfig,
    market_type: &str,
) -> AnyhowResult<Bitget> {
    let (api_key, api_secret) = config
        .get_active_api_key("bitget")
        .context("No Bitget credentials configured")?;

    let passphrase = std::env::var("BITGET_PASSPHRASE").context("BITGET_PASSPHRASE not set")?;

    let mut builder = BitgetBuilder::new()
        .sandbox(config.bitget.use_testnet)
        .api_key(api_key)
        .secret(api_secret)
        .passphrase(passphrase);

    // Set market type (spot or swap)
    if market_type == "swap" {
        builder = builder.default_type("swap");
    }
    // Default is spot, so no need to set for "spot"

    builder
        .build()
        .context("Failed to create Bitget instance with credentials")
}

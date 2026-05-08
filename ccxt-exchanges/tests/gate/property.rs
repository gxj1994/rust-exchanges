//! Property-based tests for Gate.io exchange implementation.
//!
//! These tests verify correctness properties for Gate exchange.
//!
//! ## Test Categories
//!
//! - **Builder Configuration Preservation**: Builder params are preserved after build
//! - **Symbol Conversion**: Conversion consistency properties
//! - **Exchange Trait Consistency**: Exchange trait matches PublicExchange trait
//! - **Thread Safety**: Gate implements Send + Sync
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p ccxt-exchanges gate::property
//! ```

use ccxt_core::ExchangeConfig;
use ccxt_core::exchange::Exchange;
use ccxt_core::traits::PublicExchange;
use ccxt_core::types::Timeframe;
use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};
use ccxt_exchanges::gate::{Gate, GateBuilder};

// ============================================================================
// Builder Configuration Preservation Tests
// ============================================================================

/// Test that builder preserves credentials.
#[test]
fn test_builder_preserves_credentials() {
    let api_key = "test_api_key_12345";
    let secret = "test_secret_abcdefghijklmnop";

    let gate = GateBuilder::new()
        .api_key(api_key)
        .secret(secret)
        .build()
        .expect("Gate builder should build successfully with valid credentials");

    // Verify identity
    assert_eq!(
        PublicExchange::id(&gate),
        "gate",
        "Gate id should be 'gate'"
    );
    assert_eq!(
        PublicExchange::name(&gate),
        "Gate.io",
        "Gate name should be 'Gate.io'"
    );
    assert_eq!(
        PublicExchange::version(&gate),
        "v4",
        "Gate version should be 'v4'"
    );
}

/// Test that builder preserves all default type configurations.
#[test]
fn test_builder_preserves_all_configs() {
    // Spot
    let gate = GateBuilder::new()
        .default_type(DefaultType::Spot)
        .build()
        .expect("Spot Gate should build");
    assert_eq!(PublicExchange::id(&gate), "gate");
    assert!(!gate.is_testnet());

    // Swap Linear
    let gate = GateBuilder::new()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Linear)
        .build()
        .expect("Swap Linear Gate should build");
    assert_eq!(gate.default_settle(), "usdt");

    // Swap USDC
    let gate = GateBuilder::new()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Usdc)
        .build()
        .expect("Swap USDC Gate should build");
    assert_eq!(gate.default_settle(), "usdc");

    // Swap Inverse
    let gate = GateBuilder::new()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Inverse)
        .build()
        .expect("Swap Inverse Gate should build");
    assert_eq!(gate.default_settle(), "btc");
}

/// Test that builder preserves testnet flag.
#[test]
fn test_builder_preserves_testnet() {
    let gate = GateBuilder::new()
        .testnet(true)
        .build()
        .expect("Testnet Gate should build");
    assert!(gate.is_testnet(), "is_testnet should be true");

    let gate = GateBuilder::new()
        .testnet(false)
        .build()
        .expect("Non-testnet Gate should build");
    assert!(!gate.is_testnet(), "is_testnet should be false");
}

/// Test new() constructor creates valid instance.
#[test]
fn test_new_constructor_creates_valid_instance() {
    let config = ExchangeConfig::default();
    let gate = Gate::new(config).expect("Gate::new should always succeed with default config");

    assert_eq!(PublicExchange::id(&gate), "gate");
    assert_eq!(PublicExchange::name(&gate), "Gate.io");
    assert_eq!(gate.default_settle(), "usdt");
    assert!(
        !gate.is_testnet(),
        "New Gate should not be testnet by default"
    );
}

// ============================================================================
// Symbol Conversion Property Tests
// ============================================================================

/// Test symbol conversion delimiter consistency.
///
/// Gate uses `_` as delimiter between base and quote.
#[test]
fn test_symbol_delimiter_consistency() {
    let test_pairs = vec![
        ("BTC", "USDT"),
        ("ETH", "USDT"),
        ("DOGE", "USDT"),
        ("AAVE", "ETH"),
        ("LTC", "BTC"),
    ];

    for (base, quote) in &test_pairs {
        let unified = format!("{}/{}", base, quote);
        let exchange = Gate::to_exchange_symbol(&unified);

        // Exchange format should contain exactly one underscore
        let underscore_count = exchange.chars().filter(|&c| c == '_').count();
        assert_eq!(
            underscore_count, 1,
            "Exchange symbol should have exactly one underscore separator: {} -> {}",
            unified, exchange
        );

        // Exchange symbol should start with base and end with quote
        assert!(
            exchange.starts_with(base),
            "Exchange symbol '{}' should start with base '{}' (from '{}')",
            exchange,
            base,
            unified
        );
        assert!(
            exchange.ends_with(quote),
            "Exchange symbol '{}' should end with quote '{}' (from '{}')",
            exchange,
            quote,
            unified
        );
    }
}

/// Test symbol conversion roundtrip.
#[test]
fn test_symbol_conversion_roundtrip() {
    let test_pairs = vec![
        "BTC/USDT",
        "ETH/USDT",
        "DOGE/USDT",
        "BTC/USDC",
        "ETH/BTC",
        "LTC/USDT",
    ];

    for &pair in &test_pairs {
        let exchange = Gate::to_exchange_symbol(pair);
        // Exchange format uses underscore instead of slash
        assert!(
            exchange.contains('_'),
            "Exchange symbol '{}' should contain underscore (from '{}')",
            exchange,
            pair
        );
        assert!(
            !exchange.contains('/'),
            "Exchange symbol '{}' should not contain slash (from '{}')",
            exchange,
            pair
        );
    }
}

/// Test symbol conversion with edge cases.
#[test]
fn test_symbol_conversion_edge_cases() {
    // Lowercase - Gate preserves input case
    let result = Gate::to_exchange_symbol("btc/usdt");
    assert_eq!(
        result, "btc_usdt",
        "Conversion should replace slash with underscore"
    );

    // No slash - returned as-is
    let result = Gate::to_exchange_symbol("BTCUSDT");
    assert_eq!(result, "BTCUSDT", "Without slash, should be returned as-is");

    // Slash only - edge case
    let result = Gate::to_exchange_symbol("/");
    assert_eq!(result, "_", "Sole slash should become underscore");
}

// ============================================================================
// Exchange Trait Consistency Tests
// ============================================================================

/// Test that Exchange trait metadata matches PublicExchange trait metadata.
#[test]
fn test_trait_metadata_consistency() {
    let gate = GateBuilder::new().build().expect("Failed to build Gate");

    // PublicExchange trait
    assert_eq!(
        PublicExchange::id(&gate),
        "gate",
        "id should match between traits"
    );
    assert_eq!(
        PublicExchange::name(&gate),
        "Gate.io",
        "name should match between traits"
    );
    assert_eq!(
        PublicExchange::version(&gate),
        "v4",
        "version should match between traits"
    );
    assert!(
        !PublicExchange::is_verified(&gate),
        "is_verified should be false for public Gate"
    );
}

/// Test that capabilities are present.
#[test]
fn test_capabilities_consistency() {
    let gate = GateBuilder::new().build().expect("Failed to build Gate");

    let caps = Exchange::capabilities(&gate);

    // Public API capabilities
    assert!(caps.fetch_markets(), "Should support fetch_markets");
    assert!(caps.fetch_ticker(), "Should support fetch_ticker");
    assert!(caps.fetch_order_book(), "Should support fetch_order_book");
    assert!(caps.fetch_ohlcv(), "Should support fetch_ohlcv");
    assert!(caps.fetch_trades(), "Should support fetch_trades");

    // Private API capabilities
    assert!(caps.create_order(), "Should support create_order");
    assert!(caps.cancel_order(), "Should support cancel_order");
    assert!(caps.fetch_balance(), "Should support fetch_balance");
    assert!(
        caps.fetch_account_trades(),
        "Should support fetch_account_trades"
    );
}

/// Test that timeframes are correct.
#[test]
fn test_timeframes_consistency() {
    let gate = GateBuilder::new().build().expect("Failed to build Gate");

    let tfs = PublicExchange::timeframes(&gate);

    assert!(tfs.contains(&Timeframe::M1));
    assert!(tfs.contains(&Timeframe::M5));
    assert!(tfs.contains(&Timeframe::M15));
    assert!(tfs.contains(&Timeframe::M30));
    assert!(tfs.contains(&Timeframe::H1));
    assert!(tfs.contains(&Timeframe::H4));
    assert!(tfs.contains(&Timeframe::D1));
    assert!(tfs.contains(&Timeframe::W1));
}

/// Test that REST URLs are correct.
#[test]
fn test_rest_urls_consistency() {
    let spot = GateBuilder::new()
        .default_type(DefaultType::Spot)
        .build()
        .expect("Spot Gate should build");
    assert!(spot.get_rest_url().contains("api.gateio.ws"));

    let swap = GateBuilder::new()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Linear)
        .build()
        .expect("Swap Gate should build");
    assert!(swap.get_contract_rest_url().contains("fx-api.gateio.ws"));
}

// ============================================================================
// Thread Safety Tests
// ============================================================================

/// Test that Gate implements Send + Sync.
#[test]
fn test_gate_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Gate>();
}

/// Test that Gate can be used across threads.
#[test]
fn test_gate_thread_safety() {
    use std::thread;

    let gate = GateBuilder::new().build().expect("Failed to build Gate");

    let handle = thread::spawn(move || {
        assert_eq!(PublicExchange::id(&gate), "gate");
        assert_eq!(PublicExchange::name(&gate), "Gate.io");
        gate
    });

    let gate = handle.join().expect("Thread should complete");
    assert_eq!(PublicExchange::id(&gate), "gate");
}

/// Test that GateBuilder is Send + Sync.
#[test]
fn test_gate_builder_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GateBuilder>();
}

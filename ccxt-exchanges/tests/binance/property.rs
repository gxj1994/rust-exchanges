//! Property-based tests for backward compatibility.
//!
//! This test module verifies that calling methods through the legacy `Exchange` trait
//! produces the same results as calling methods through the new decomposed trait structure.
//!
//! **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
//! **Validates: Requirements 3.1, 3.2**

use ccxt_core::{
    ExchangeConfig,
    exchange::{Exchange, ExchangeExt},
    traits::PublicExchange,
};
use ccxt_exchanges::binance::Binance;
use proptest::prelude::*;

// ============================================================================
// Strategies
// ============================================================================

/// Strategy to generate various ExchangeConfig configurations
fn arb_exchange_config() -> impl Strategy<Value = ExchangeConfig> {
    (
        prop::bool::ANY,                                                      // sandbox
        prop::option::of(any::<u64>().prop_map(|n| format!("key_{}", n))),    // api_key
        prop::option::of(any::<u64>().prop_map(|n| format!("secret_{}", n))), // secret
    )
        .prop_map(|(sandbox, api_key, secret)| ExchangeConfig {
            sandbox,
            api_key: api_key.map(ccxt_core::SecretString::new),
            secret: secret.map(ccxt_core::SecretString::new),
            ..Default::default()
        })
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling metadata methods through the
    /// legacy `Exchange` trait SHALL produce identical results to calling them directly
    /// on the Binance struct.
    ///
    /// This property ensures that existing code using `dyn Exchange` continues to work
    /// without modification after the refactoring.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_id(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get trait object reference
        let exchange: &dyn Exchange = &binance;

        // Property: id() through trait object should match direct call
        prop_assert_eq!(
            exchange.id(),
            Binance::id(&binance),
            "id() should be consistent between trait object and direct call"
        );

        // Property: id() should always return "binance"
        prop_assert_eq!(
            exchange.id(),
            "binance",
            "Binance id() should always return 'binance'"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling name() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_name(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: name() through trait object should match direct call
        prop_assert_eq!(
            exchange.name(),
            Binance::name(&binance),
            "name() should be consistent between trait object and direct call"
        );

        // Property: name() should always return "Binance"
        prop_assert_eq!(
            exchange.name(),
            "Binance",
            "Binance name() should always return 'Binance'"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling version() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_version(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: version() through trait object should match direct call
        prop_assert_eq!(
            exchange.version(),
            Binance::version(&binance),
            "version() should be consistent between trait object and direct call"
        );

        // Property: version() should always return "v3"
        prop_assert_eq!(
            exchange.version(),
            "v3",
            "Binance version() should always return 'v3'"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling certified() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_certified(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: certified() through trait object should match direct call
        prop_assert_eq!(
            exchange.is_verified(),
            Binance::is_verified(&binance),
            "certified() should be consistent between trait object and direct call"
        );

        // Property: Binance should always be certified
        prop_assert!(
            exchange.is_verified(),
            "Binance should always be certified"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling rate_limit() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_rate_limit(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: rate_limit() through trait object should match direct call
        prop_assert_eq!(
            exchange.requests_per_second(),
            Binance::requests_per_second(&binance),
            "rate_limit() should be consistent between trait object and direct call"
        );

        // Property: rate_limit() should return a positive value
        prop_assert!(
            exchange.requests_per_second() > 0,
            "rate_limit() should return a positive value"
        );

        // Property: Binance rate_limit should be 50
        prop_assert_eq!(
            exchange.requests_per_second(),
            50,
            "Binance rate_limit() should be 50"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling timeframes() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_timeframes(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: timeframes() through trait object should match direct call
        let trait_timeframes = exchange.timeframes();
        let direct_timeframes = <Binance as Exchange>::timeframes(&binance);

        prop_assert_eq!(
            trait_timeframes.len(),
            direct_timeframes.len(),
            "timeframes() should return same number of timeframes"
        );

        // Property: timeframes should not be empty
        prop_assert!(
            !trait_timeframes.is_empty(),
            "timeframes() should not be empty"
        );

        // Property: all timeframes from trait object should be in direct call
        for tf in trait_timeframes {
            prop_assert!(
                direct_timeframes.iter().any(|dtf| dtf == tf),
                "All timeframes from trait object should be in direct call"
            );
        }
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, calling capabilities() through the legacy
    /// `Exchange` trait SHALL produce identical results to calling it directly.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_capabilities(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: capabilities() through trait object should match direct call
        let trait_caps = exchange.capabilities();
        let direct_caps = <Binance as Exchange>::capabilities(&binance);

        prop_assert_eq!(
            trait_caps, direct_caps,
            "capabilities() should be consistent between trait object and direct call"
        );

        // Property: Binance should support market data capabilities
        prop_assert!(
            trait_caps.fetch_markets(),
            "Binance should support fetch_markets"
        );
        prop_assert!(
            trait_caps.fetch_ticker(),
            "Binance should support fetch_ticker"
        );
        prop_assert!(
            trait_caps.fetch_order_book(),
            "Binance should support fetch_order_book"
        );
        prop_assert!(
            trait_caps.fetch_trades(),
            "Binance should support fetch_trades"
        );
        prop_assert!(
            trait_caps.fetch_ohlcv(),
            "Binance should support fetch_ohlcv"
        );

        // Property: Binance should support trading capabilities
        prop_assert!(
            trait_caps.create_order(),
            "Binance should support create_order"
        );
        prop_assert!(
            trait_caps.cancel_order(),
            "Binance should support cancel_order"
        );
        prop_assert!(
            trait_caps.fetch_order(),
            "Binance should support fetch_order"
        );

        // Property: Binance should support account capabilities
        prop_assert!(
            trait_caps.fetch_balance(),
            "Binance should support fetch_balance"
        );
        prop_assert!(
            trait_caps.fetch_account_trades(),
            "Binance should support fetch_account_trades"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, the Exchange trait object should be
    /// usable in the same way as the direct Binance struct for all metadata operations.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_all_metadata(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: All metadata methods should be callable and consistent
        let id = exchange.id();
        let name = exchange.name();
        let version = exchange.version();
        let certified = exchange.is_verified();
        let rate_limit = exchange.requests_per_second();
        let timeframes = exchange.timeframes();
        let capabilities = exchange.capabilities();

        // Verify all values are valid
        prop_assert!(!id.is_empty(), "id should not be empty");
        prop_assert!(!name.is_empty(), "name should not be empty");
        prop_assert!(!version.is_empty(), "version should not be empty");
        prop_assert!(rate_limit > 0, "rate_limit should be positive");
        prop_assert!(!timeframes.is_empty(), "timeframes should not be empty");

        // Verify consistency with direct calls
        prop_assert_eq!(id, <Binance as Exchange>::id(&binance));
        prop_assert_eq!(name, <Binance as Exchange>::name(&binance));
        prop_assert_eq!(version, <Binance as Exchange>::version(&binance));
        prop_assert_eq!(certified, <Binance as Exchange>::is_verified(&binance));
        prop_assert_eq!(rate_limit, <Binance as Exchange>::requests_per_second(&binance));
        prop_assert_eq!(timeframes.len(), <Binance as Exchange>::timeframes(&binance).len());
        prop_assert_eq!(capabilities, <Binance as Exchange>::capabilities(&binance));
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, the Exchange trait should support
    /// the ExchangeExt extension trait methods for capability checking.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_exchange_ext(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;

        // Property: ExchangeExt methods should work through trait object
        prop_assert!(
            exchange.supports_market_data(),
            "Binance should support market data"
        );
        prop_assert!(
            exchange.supports_trading(),
            "Binance should support trading"
        );
        prop_assert!(
            exchange.supports_account(),
            "Binance should support account"
        );
        prop_assert!(
            exchange.supports_margin(),
            "Binance should support margin"
        );
        prop_assert!(
            exchange.supports_funding(),
            "Binance should support funding"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 1: Backward Compatibility - Method Behavior Equivalence**
    ///
    /// *For any* valid exchange configuration, both Exchange and PublicExchange traits
    /// should return identical metadata values.
    ///
    /// **Validates: Requirements 3.1, 3.2**
    #[test]
    fn prop_backward_compatibility_exchange_vs_public_exchange(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let exchange: &dyn Exchange = &binance;
        let public_exchange: &dyn PublicExchange = &binance;

        // Property: Both traits should return identical values
        prop_assert_eq!(
            exchange.id(),
            public_exchange.id(),
            "id() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.name(),
            public_exchange.name(),
            "name() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.version(),
            public_exchange.version(),
            "version() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.is_verified(),
            public_exchange.is_verified(),
            "certified() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.requests_per_second(),
            public_exchange.requests_per_second(),
            "rate_limit() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.timeframes(),
            public_exchange.timeframes(),
            "timeframes() should be identical between Exchange and PublicExchange"
        );

        prop_assert_eq!(
            exchange.capabilities(),
            public_exchange.capabilities(),
            "capabilities() should be identical between Exchange and PublicExchange"
        );
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_backward_compatibility_metadata_consistency() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let exchange: &dyn Exchange = &binance;

    // Verify all metadata methods work through trait object
    assert_eq!(exchange.id(), "binance");
    assert_eq!(exchange.name(), "Binance");
    assert_eq!(exchange.version(), "v3");
    assert!(exchange.is_verified());
    assert_eq!(exchange.requests_per_second(), 50);
    assert!(!exchange.timeframes().is_empty());
}

#[test]
fn test_backward_compatibility_capabilities_consistency() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let exchange: &dyn Exchange = &binance;
    let caps = exchange.capabilities();

    // Verify capabilities are consistent
    assert!(caps.fetch_markets());
    assert!(caps.fetch_ticker());
    assert!(caps.create_order());
    assert!(caps.fetch_balance());
}

#[test]
fn test_backward_compatibility_trait_object_creation() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify we can create a trait object
    let _exchange: Box<dyn Exchange> = Box::new(binance);
}

#[test]
fn test_backward_compatibility_exchange_ext_methods() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let exchange: &dyn Exchange = &binance;

    // Verify ExchangeExt methods work
    assert!(exchange.supports_market_data());
    assert!(exchange.supports_trading());
    assert!(exchange.supports_account());
    assert!(exchange.supports_margin());
    assert!(exchange.supports_funding());
}

#[test]
fn test_backward_compatibility_exchange_vs_public_exchange() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let exchange: &dyn Exchange = &binance;
    let public_exchange: &dyn PublicExchange = &binance;

    // Verify both traits return identical values
    assert_eq!(exchange.id(), public_exchange.id());
    assert_eq!(exchange.name(), public_exchange.name());
    assert_eq!(exchange.version(), public_exchange.version());
    assert_eq!(exchange.is_verified(), public_exchange.is_verified());
    assert_eq!(
        exchange.requests_per_second(),
        public_exchange.requests_per_second()
    );
    assert_eq!(exchange.timeframes(), public_exchange.timeframes());
    assert_eq!(exchange.capabilities(), public_exchange.capabilities());
}

// Property-based tests for capability-trait consistency.
//
// This test module verifies that exchange capability flags match the implemented traits.
// For any exchange that implements a specific trait (e.g., MarketData), all corresponding
// capability flags in ExchangeCapabilities SHALL return true.
//
// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
// **Validates: Requirements 5.2**

use ccxt_core::capability::Capability;

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares market data capabilities, all market data
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// This property ensures that capability flags accurately reflect supported features.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_market_data_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All market data capabilities should be enabled for Binance
        prop_assert!(
            caps.fetch_markets(),
            "Binance should support fetch_markets capability"
        );
        prop_assert!(
            caps.fetch_ticker(),
            "Binance should support fetch_ticker capability"
        );
        prop_assert!(
            caps.fetch_tickers(),
            "Binance should support fetch_tickers capability"
        );
        prop_assert!(
            caps.fetch_order_book(),
            "Binance should support fetch_order_book capability"
        );
        prop_assert!(
            caps.fetch_trades(),
            "Binance should support fetch_trades capability"
        );
        prop_assert!(
            caps.fetch_ohlcv(),
            "Binance should support fetch_ohlcv capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares trading capabilities, all trading
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_trading_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All trading capabilities should be enabled for Binance
        prop_assert!(
            caps.create_order(),
            "Binance should support create_order capability"
        );
        prop_assert!(
            caps.cancel_order(),
            "Binance should support cancel_order capability"
        );
        prop_assert!(
            caps.fetch_order(),
            "Binance should support fetch_order capability"
        );
        prop_assert!(
            caps.fetch_open_orders(),
            "Binance should support fetch_open_orders capability"
        );
        prop_assert!(
            caps.fetch_history_orders(),
            "Binance should support FETCH_HISTORY_ORDERS capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares account capabilities, all account
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_account_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All account capabilities should be enabled for Binance
        prop_assert!(
            caps.fetch_balance(),
            "Binance should support fetch_balance capability"
        );
        prop_assert!(
            caps.fetch_account_trades(),
            "Binance should support fetch_account_trades capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares margin capabilities, all margin
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_margin_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All margin capabilities should be enabled for Binance
        prop_assert!(
            caps.fetch_positions(),
            "Binance should support fetch_positions capability"
        );
        prop_assert!(
            caps.set_leverage(),
            "Binance should support set_leverage capability"
        );
        prop_assert!(
            caps.set_margin_mode(),
            "Binance should support set_margin_mode capability"
        );
        prop_assert!(
            caps.fetch_funding_rate(),
            "Binance should support fetch_funding_rate capability"
        );
        prop_assert!(
            caps.fetch_funding_rates(),
            "Binance should support fetch_funding_rates capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares funding capabilities, all funding
    /// capability flags in `ExchangeCapabilities` SHALL return `true`.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_funding_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All funding capabilities should be enabled for Binance
        prop_assert!(
            caps.fetch_deposit_address(),
            "Binance should support fetch_deposit_address capability"
        );
        prop_assert!(
            caps.withdraw(),
            "Binance should support withdraw capability"
        );
        prop_assert!(
            caps.fetch_deposits(),
            "Binance should support fetch_deposits capability"
        );
        prop_assert!(
            caps.fetch_withdrawals(),
            "Binance should support fetch_withdrawals capability"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange configuration, the capabilities returned by the exchange
    /// SHALL be consistent across multiple calls.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Property: Multiple calls to capabilities() should return identical results
        let caps1 = Exchange::capabilities(&binance);
        let caps2 = Exchange::capabilities(&binance);
        let caps3 = Exchange::capabilities(&binance);

        prop_assert_eq!(
            caps1, caps2,
            "capabilities() should return consistent results on multiple calls"
        );
        prop_assert_eq!(
            caps2, caps3,
            "capabilities() should return consistent results on multiple calls"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange that declares all capabilities, all corresponding
    /// capability flags SHALL be enabled.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_all_capabilities_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        // Get capabilities
        let caps = Exchange::capabilities(&binance);

        // Property: All capabilities should be enabled (except those explicitly disabled)
        // Market Data
        prop_assert!(caps.fetch_markets());
        prop_assert!(caps.fetch_ticker());
        prop_assert!(caps.fetch_tickers());
        prop_assert!(caps.fetch_order_book());
        prop_assert!(caps.fetch_trades());
        prop_assert!(caps.fetch_ohlcv());

        // Trading
        prop_assert!(caps.create_order());
        prop_assert!(caps.cancel_order());
        prop_assert!(caps.fetch_order());
        prop_assert!(caps.fetch_open_orders());
        prop_assert!(caps.fetch_history_orders());

        // Account
        prop_assert!(caps.fetch_balance());
        prop_assert!(caps.fetch_account_trades());

        // Margin
        prop_assert!(caps.fetch_positions());
        prop_assert!(caps.set_leverage());
        prop_assert!(caps.set_margin_mode());
        prop_assert!(caps.fetch_funding_rate());
        prop_assert!(caps.fetch_funding_rates());

        // Funding
        prop_assert!(caps.fetch_deposit_address());
        prop_assert!(caps.withdraw());
        prop_assert!(caps.fetch_deposits());
        prop_assert!(caps.fetch_withdrawals());
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* capability that is enabled, the corresponding capability name
    /// should be in the list of supported capabilities.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_enabled_capabilities_in_supported_list(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let caps = Exchange::capabilities(&binance);
        let supported = caps.supported_capabilities();

        // Property: All enabled capabilities should be in the supported list
        for cap in Capability::all().iter() {
            let cap_name = cap.as_ccxt_name();
            let is_enabled = caps.has(cap_name);
            let is_in_list = supported.contains(&cap_name);

            prop_assert_eq!(
                is_enabled, is_in_list,
                "Capability {} should be in supported list if enabled",
                cap_name
            );
        }
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange, the capability count should match the number of
    /// capabilities in the supported list.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_capability_count_matches_supported_list(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let caps = Exchange::capabilities(&binance);
        let supported = caps.supported_capabilities();
        let count = caps.count();

        prop_assert_eq!(
            count as usize, supported.len(),
            "Capability count should match supported list length"
        );
    }

    /// **Feature: binance-rest-api-modularization, Property 2: Capability-Trait Consistency**
    ///
    /// *For any* exchange, querying capabilities by name should be consistent
    /// with the capability flags.
    ///
    /// **Validates: Requirements 5.2**
    #[test]
    fn prop_capability_name_query_consistency(config in arb_exchange_config()) {
        let binance = Binance::new(config).expect("Should create Binance instance");

        let caps = Exchange::capabilities(&binance);

        // Property: has() method should be consistent with individual capability methods
        prop_assert_eq!(
            caps.has("fetchMarkets"),
            caps.fetch_markets(),
            "has('fetchMarkets') should match fetch_markets()"
        );
        prop_assert_eq!(
            caps.has("fetchTicker"),
            caps.fetch_ticker(),
            "has('fetchTicker') should match fetch_ticker()"
        );
        prop_assert_eq!(
            caps.has("createOrder"),
            caps.create_order(),
            "has('createOrder') should match create_order()"
        );
        prop_assert_eq!(
            caps.has("fetchBalance"),
            caps.fetch_balance(),
            "has('fetchBalance') should match fetch_balance()"
        );
        prop_assert_eq!(
            caps.has("fetchPositions"),
            caps.fetch_positions(),
            "has('fetchPositions') should match fetch_positions()"
        );
        prop_assert_eq!(
            caps.has("fetchDepositAddress"),
            caps.fetch_deposit_address(),
            "has('fetchDepositAddress') should match fetch_deposit_address()"
        );
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_capability_trait_consistency_market_data() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all market data capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.fetch_markets());
    assert!(caps.fetch_ticker());
    assert!(caps.fetch_tickers());
    assert!(caps.fetch_order_book());
    assert!(caps.fetch_trades());
    assert!(caps.fetch_ohlcv());
}

#[test]
fn test_capability_trait_consistency_trading() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all trading capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.create_order());
    assert!(caps.cancel_order());
    assert!(caps.fetch_order());
    assert!(caps.fetch_open_orders());
    assert!(caps.fetch_history_orders());
}

#[test]
fn test_capability_trait_consistency_account() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all account capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.fetch_balance());
    assert!(caps.fetch_account_trades());
}

#[test]
fn test_capability_trait_consistency_margin() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all margin capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.fetch_positions());
    assert!(caps.set_leverage());
    assert!(caps.set_margin_mode());
    assert!(caps.fetch_funding_rate());
    assert!(caps.fetch_funding_rates());
}

#[test]
fn test_capability_trait_consistency_funding() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all funding capabilities are enabled
    let caps = Exchange::capabilities(&binance);
    assert!(caps.fetch_deposit_address());
    assert!(caps.withdraw());
    assert!(caps.fetch_deposits());
    assert!(caps.fetch_withdrawals());
}

#[test]
fn test_capability_trait_consistency_all_capabilities() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify all capabilities are enabled (except explicitly disabled ones)
    let caps = Exchange::capabilities(&binance);

    // Market Data
    assert!(caps.fetch_markets());
    assert!(caps.fetch_ticker());
    assert!(caps.fetch_tickers());
    assert!(caps.fetch_order_book());
    assert!(caps.fetch_trades());
    assert!(caps.fetch_ohlcv());

    // Trading
    assert!(caps.create_order());
    assert!(caps.cancel_order());
    assert!(caps.fetch_order());
    assert!(caps.fetch_open_orders());
    assert!(caps.fetch_history_orders());

    // Account
    assert!(caps.fetch_balance());
    assert!(caps.fetch_account_trades());

    // Margin
    assert!(caps.fetch_positions());
    assert!(caps.set_leverage());
    assert!(caps.set_margin_mode());
    assert!(caps.fetch_funding_rate());
    assert!(caps.fetch_funding_rates());

    // Funding
    assert!(caps.fetch_deposit_address());
    assert!(caps.withdraw());
    assert!(caps.fetch_deposits());
    assert!(caps.fetch_withdrawals());
}

#[test]
fn test_capability_consistency_across_calls() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    // Verify capabilities are consistent across multiple calls
    let caps1 = Exchange::capabilities(&binance);
    let caps2 = Exchange::capabilities(&binance);
    let caps3 = Exchange::capabilities(&binance);

    assert_eq!(caps1, caps2);
    assert_eq!(caps2, caps3);
}

#[test]
fn test_capability_supported_list_consistency() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let caps = Exchange::capabilities(&binance);
    let supported = caps.supported_capabilities();

    // Verify all supported capabilities are actually enabled
    for cap_name in supported {
        assert!(
            caps.has(cap_name),
            "Capability {} should be enabled if in supported list",
            cap_name
        );
    }
}

#[test]
fn test_capability_count_matches_supported_list() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let caps = Exchange::capabilities(&binance);
    let supported = caps.supported_capabilities();
    let count = caps.count();

    assert_eq!(
        count as usize,
        supported.len(),
        "Capability count should match supported list length"
    );
}

#[test]
fn test_capability_name_query_consistency() {
    let config = ExchangeConfig::default();
    let binance = Binance::new(config).expect("Should create Binance instance");

    let caps = Exchange::capabilities(&binance);

    // Verify has() method is consistent with individual capability methods
    assert_eq!(caps.has("fetchMarkets"), caps.fetch_markets());
    assert_eq!(caps.has("fetchTicker"), caps.fetch_ticker());
    assert_eq!(caps.has("createOrder"), caps.create_order());
    assert_eq!(caps.has("fetchBalance"), caps.fetch_balance());
    assert_eq!(caps.has("fetchPositions"), caps.fetch_positions());
    assert_eq!(
        caps.has("fetchDepositAddress"),
        caps.fetch_deposit_address()
    );
}

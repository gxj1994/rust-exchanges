//! Enhanced capability-aware conformance tests.
//!
//! This module provides detailed conformance tests organized by capability:
//! - PublicExchange conformance
//! - MarketData conformance
//! - Trading conformance
//! - Account conformance

use ccxt_core::exchange::{Exchange, PublicExchange};
use ccxt_core::traits::{Account, MarketData, Trading};
use ccxt_core::types::Timeframe;

/// Full exchange conformance registration with capability tracking.
pub struct CapabilityConformanceRegistration<E>
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    pub factory: fn() -> E,
    pub id: &'static str,
    pub name: &'static str,
    pub expected_capabilities: Vec<&'static str>,
    pub expected_timeframes: Vec<Timeframe>,
}

// ============================================================================
// PublicExchange Conformance
// ============================================================================

/// Assert PublicExchange metadata consistency.
pub fn assert_public_exchange_metadata<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    let exchange = (registration.factory)();
    let public_view: &dyn PublicExchange = &exchange;
    let exchange_view: &dyn Exchange = &exchange;

    // Metadata consistency
    assert_eq!(
        public_view.id(),
        exchange_view.id(),
        "ID mismatch between PublicExchange and Exchange"
    );
    assert_eq!(public_view.name(), exchange_view.name(), "Name mismatch");
    assert_eq!(
        public_view.version(),
        exchange_view.version(),
        "Version mismatch"
    );
    assert_eq!(
        public_view.is_verified(),
        exchange_view.is_verified(),
        "Certified flag mismatch"
    );

    // Registration consistency
    assert_eq!(
        public_view.id(),
        registration.id,
        "ID doesn't match registration"
    );
    assert_eq!(
        public_view.name(),
        registration.name,
        "Name doesn't match registration"
    );
}

/// Assert capabilities are consistent across views.
pub fn assert_capabilities_consistency<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    let exchange = (registration.factory)();
    let public_view: &dyn PublicExchange = &exchange;
    let exchange_view: &dyn Exchange = &exchange;

    // Capabilities must be identical
    let public_caps = public_view.capabilities();
    let exchange_caps = exchange_view.capabilities();
    assert_eq!(
        public_caps, exchange_caps,
        "Capabilities mismatch between PublicExchange and Exchange"
    );

    // Expected capabilities must be present
    for cap_name in &registration.expected_capabilities {
        match *cap_name {
            "fetch_markets" => assert!(
                exchange_caps.fetch_markets(),
                "Expected capability: fetch_markets"
            ),
            "fetch_ticker" => assert!(
                exchange_caps.fetch_ticker(),
                "Expected capability: fetch_ticker"
            ),
            "create_order" => assert!(
                exchange_caps.create_order(),
                "Expected capability: create_order"
            ),
            "cancel_order" => assert!(
                exchange_caps.cancel_order(),
                "Expected capability: cancel_order"
            ),
            "fetch_balance" => assert!(
                exchange_caps.fetch_balance(),
                "Expected capability: fetch_balance"
            ),
            "fetch_account_trades" => assert!(
                exchange_caps.fetch_account_trades(),
                "Expected capability: fetch_account_trades"
            ),
            _ => {} // Unknown capability, skip
        }
    }
}

// ============================================================================
// MarketData Conformance
// ============================================================================

/// Assert MarketData capability contract.
pub fn assert_market_data_contract<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    let exchange = (registration.factory)();
    let market_data: &dyn MarketData = &exchange;
    let exchange_view: &dyn Exchange = &exchange;

    // ID consistency
    assert_eq!(
        market_data.id(),
        exchange_view.id(),
        "MarketData ID mismatch"
    );

    // Timeframes consistency
    let market_data_timeframes = market_data.timeframes();
    let exchange_timeframes = exchange_view.timeframes();
    assert_eq!(
        market_data_timeframes, exchange_timeframes,
        "Timeframes mismatch between MarketData and Exchange"
    );

    // Expected timeframes
    if !registration.expected_timeframes.is_empty() {
        for expected_tf in &registration.expected_timeframes {
            assert!(
                market_data_timeframes.contains(expected_tf),
                "Expected timeframe {:?} not found",
                expected_tf
            );
        }
    }

    // OHLCV capability requires non-empty timeframes
    if exchange_view.capabilities().fetch_ohlcv() {
        assert!(
            !market_data_timeframes.is_empty(),
            "OHLCV-capable exchange must declare timeframes"
        );
    }
}

/// Assert MarketData trait object safety.
pub fn assert_market_data_trait_object<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    let exchange = (registration.factory)();
    let market_data: Box<dyn MarketData> = Box::new(exchange);

    assert_eq!(
        market_data.id(),
        registration.id,
        "MarketData trait object ID mismatch"
    );
    assert!(
        !market_data.timeframes().is_empty(),
        "MarketData must have timeframes"
    );
}

// ============================================================================
// Trading Conformance
// ============================================================================

/// Assert Trading capability contract.
pub fn assert_trading_contract<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    let exchange = (registration.factory)();
    let trading: &dyn Trading = &exchange;
    let exchange_view: &dyn Exchange = &exchange;

    // ID consistency
    assert_eq!(trading.id(), exchange_view.id(), "Trading ID mismatch");

    // Capabilities consistency
    let trading_caps = trading.capabilities();
    let exchange_caps = exchange_view.capabilities();
    assert_eq!(
        trading_caps, exchange_caps,
        "Capabilities mismatch between Trading and Exchange"
    );
}

/// Assert Trading trait object safety.
pub fn assert_trading_trait_object<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    let exchange = (registration.factory)();
    let trading: Box<dyn Trading> = Box::new(exchange);

    assert_eq!(
        trading.id(),
        registration.id,
        "Trading trait object ID mismatch"
    );
}

// ============================================================================
// Account Conformance
// ============================================================================

/// Assert Account capability contract.
pub fn assert_account_contract<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    let exchange = (registration.factory)();
    let account: &dyn Account = &exchange;
    let exchange_view: &dyn Exchange = &exchange;

    // ID consistency
    assert_eq!(account.id(), exchange_view.id(), "Account ID mismatch");

    // Capabilities consistency
    let account_caps = account.capabilities();
    let exchange_caps = exchange_view.capabilities();
    assert_eq!(
        account_caps, exchange_caps,
        "Capabilities mismatch between Account and Exchange"
    );
}

/// Assert Account trait object safety.
pub fn assert_account_trait_object<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    let exchange = (registration.factory)();
    let account: Box<dyn Account> = Box::new(exchange);

    assert_eq!(
        account.id(),
        registration.id,
        "Account trait object ID mismatch"
    );
}

// ============================================================================
// Full Conformance Suite
// ============================================================================

/// Run all conformance tests for an exchange.
pub fn assert_full_conformance<E>(registration: &CapabilityConformanceRegistration<E>)
where
    E: Exchange + PublicExchange + MarketData + Trading + Account + 'static,
{
    // PublicExchange conformance
    assert_public_exchange_metadata(registration);
    assert_capabilities_consistency(registration);

    // MarketData conformance
    assert_market_data_contract(registration);
    assert_market_data_trait_object(registration);

    // Trading conformance
    assert_trading_contract(registration);
    assert_trading_trait_object(registration);

    // Account conformance
    assert_account_contract(registration);
    assert_account_trait_object(registration);
}

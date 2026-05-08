use super::capability_conformance::{CapabilityConformanceRegistration, assert_full_conformance};
use ccxt_core::ExchangeConfig;
use ccxt_core::types::Timeframe;
use ccxt_exchanges::gate::Gate;

fn registration() -> CapabilityConformanceRegistration<Gate> {
    CapabilityConformanceRegistration {
        factory: || Gate::new(ExchangeConfig::default()).expect("Gate should construct"),
        id: "gate",
        name: "Gate.io",
        expected_capabilities: vec![
            "fetch_markets",
            "fetch_ticker",
            "create_order",
            "cancel_order",
            "fetch_balance",
            "fetch_account_trades",
        ],
        expected_timeframes: vec![
            Timeframe::M1,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::H1,
            Timeframe::H4,
            Timeframe::D1,
        ],
    }
}

#[test]
fn test_gate_full_conformance() {
    assert_full_conformance(&registration());
}

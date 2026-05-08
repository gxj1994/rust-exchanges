use super::capability_conformance::{CapabilityConformanceRegistration, assert_full_conformance};
use ccxt_core::ExchangeConfig;
use ccxt_core::types::Timeframe;
use ccxt_exchanges::bybit::Bybit;

fn registration() -> CapabilityConformanceRegistration<Bybit> {
    CapabilityConformanceRegistration {
        factory: || Bybit::new(ExchangeConfig::default()).expect("Bybit should construct"),
        id: "bybit",
        name: "Bybit",
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
fn test_bybit_full_conformance() {
    assert_full_conformance(&registration());
}

use super::capability_conformance::{CapabilityConformanceRegistration, assert_full_conformance};
use ccxt_core::ExchangeConfig;
use ccxt_core::types::Timeframe;
use ccxt_exchanges::okx::Okx;

fn registration() -> CapabilityConformanceRegistration<Okx> {
    CapabilityConformanceRegistration {
        factory: || Okx::new(ExchangeConfig::default()).expect("OKX should construct"),
        id: "okx",
        name: "OKX",
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
fn test_okx_full_conformance() {
    assert_full_conformance(&registration());
}

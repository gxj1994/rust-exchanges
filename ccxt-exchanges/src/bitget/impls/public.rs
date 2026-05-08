//! PublicExchange trait implementation for Bitget.
//!
//! This module implements the PublicExchange trait for Bitget exchange,
//! providing metadata, capabilities, and configuration.

use ccxt_core::{exchange::ExchangeCapabilities, traits::PublicExchange, types::Timeframe};

use crate::bitget::Bitget;
use crate::bitget::{bitget_capabilities, bitget_timeframes};

impl PublicExchange for Bitget {
    fn id(&self) -> &'static str {
        "bitget"
    }

    fn name(&self) -> &'static str {
        "Bitget"
    }

    fn version(&self) -> &'static str {
        "v2"
    }

    fn is_verified(&self) -> bool {
        false
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        bitget_capabilities()
    }

    fn timeframes(&self) -> &'static [Timeframe] {
        bitget_timeframes()
    }

    fn requests_per_second(&self) -> u32 {
        20
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_bitget_public_exchange_impl() {
        let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
        let pe: &dyn PublicExchange = &bitget;

        assert_eq!(pe.id(), "bitget");
        assert_eq!(pe.name(), "Bitget");
        assert_eq!(pe.version(), "v2");
        assert!(!pe.is_verified());
        assert!(pe.capabilities().has("websocket"));
        assert_eq!(pe.requests_per_second(), 20);
    }

    #[test]
    fn test_bitget_capabilities_include_margin() {
        let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
        let caps = PublicExchange::capabilities(&bitget);

        assert!(caps.has("fetchPositions"));
        assert!(caps.has("setLeverage"));
        assert!(caps.has("setMarginMode"));
        assert!(caps.has("fetchFundingRate"));
        assert!(caps.has("fetchFundingRates"));
    }
}

use ccxt_core::{capability::ExchangeCapabilities, traits::PublicExchange, types::Timeframe};

use crate::okx::{Okx, okx_capabilities, okx_timeframes};

impl PublicExchange for Okx {
    fn id(&self) -> &'static str {
        "okx"
    }

    fn name(&self) -> &'static str {
        "OKX"
    }

    fn version(&self) -> &'static str {
        "v5"
    }

    fn is_verified(&self) -> bool {
        false
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        okx_capabilities()
    }

    fn timeframes(&self) -> &'static [Timeframe] {
        okx_timeframes()
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
    fn test_okx_public_exchange_impl() {
        let okx = Okx::new(ExchangeConfig::default()).unwrap();
        let pe: &dyn PublicExchange = &okx;

        assert_eq!(pe.id(), "okx");
        assert_eq!(pe.name(), "OKX");
        assert_eq!(pe.version(), "v5");
        assert!(!pe.is_verified());
        assert!(pe.capabilities().has("websocket"));
        assert_eq!(pe.requests_per_second(), 20);
    }

    #[test]
    fn test_okx_capabilities_include_margin() {
        let okx = Okx::new(ExchangeConfig::default()).unwrap();
        let caps = PublicExchange::capabilities(&okx);

        assert!(caps.has("fetchPositions"));
        assert!(caps.has("setLeverage"));
        assert!(caps.has("setMarginMode"));
        assert!(caps.has("fetchFundingRate"));
        assert!(caps.has("fetchFundingRates"));
    }
}

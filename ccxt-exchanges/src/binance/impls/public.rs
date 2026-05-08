use ccxt_core::{
    exchange::{Capability, ExchangeCapabilities},
    traits::PublicExchange,
    types::Timeframe,
};

use crate::binance::Binance;

impl PublicExchange for Binance {
    fn id(&self) -> &'static str {
        "binance"
    }

    fn name(&self) -> &'static str {
        "Binance"
    }

    fn version(&self) -> &'static str {
        "v3"
    }

    fn is_verified(&self) -> bool {
        true
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        ExchangeCapabilities::builder()
            .all()
            .without_capability(Capability::FetchOrders)
            .without_capability(Capability::FetchCanceledOrders)
            .build()
    }

    fn timeframes(&self) -> &'static [Timeframe] {
        &[
            Timeframe::M1,
            Timeframe::M3,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H2,
            Timeframe::H4,
            Timeframe::H6,
            Timeframe::H8,
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::D3,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn requests_per_second(&self) -> u32 {
        self.options.rate_limit
    }
}

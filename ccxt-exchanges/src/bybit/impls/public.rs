//! PublicExchange trait implementation for Bybit.
//!
//! This module implements the PublicExchange trait for Bybit exchange,
//! providing metadata, capabilities, and configuration.

use ccxt_core::{exchange::ExchangeCapabilities, traits::PublicExchange, types::Timeframe};

use crate::bybit::Bybit;
use crate::bybit::{bybit_capabilities, bybit_timeframes};

impl PublicExchange for Bybit {
    fn id(&self) -> &str {
        self.id()
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn version(&self) -> &'static str {
        self.version()
    }

    fn is_verified(&self) -> bool {
        self.is_verified()
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        bybit_capabilities()
    }

    fn timeframes(&self) -> &'static [Timeframe] {
        bybit_timeframes()
    }

    #[allow(deprecated)]
    fn requests_per_second(&self) -> u32 {
        self.requests_per_second()
    }

    fn is_sandbox(&self) -> bool {
        self.is_sandbox()
    }

    fn urls(&self) -> Vec<&'static str> {
        if self.is_sandbox() {
            vec!["https://api-testnet.bybit.com", "https://www.bybit.com"]
        } else {
            vec!["https://api.bybit.com", "https://www.bybit.com"]
        }
    }
}

//! PublicExchange trait implementation for HyperLiquid.
//!
//! This module implements the PublicExchange trait for HyperLiquid DEX,
//! providing metadata, capabilities, and configuration.

use ccxt_core::{
    exchange::{Capability, ExchangeCapabilities},
    traits::PublicExchange,
    types::Timeframe,
};

use crate::hyperliquid::HyperLiquid;

impl PublicExchange for HyperLiquid {
    fn id(&self) -> &'static str {
        "hyperliquid"
    }

    fn name(&self) -> &'static str {
        "HyperLiquid"
    }

    fn version(&self) -> &'static str {
        "1"
    }

    fn is_verified(&self) -> bool {
        false
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        ExchangeCapabilities::builder()
            .market_data()
            .trading()
            .without_capability(Capability::FetchCurrencies)
            .without_capability(Capability::FetchStatus)
            .without_capability(Capability::FetchTime)
            .without_capability(Capability::FetchOrders)
            .without_capability(Capability::FetchCanceledOrders)
            .capability(Capability::FetchOrder)
            .capability(Capability::FetchHistoryOrders)
            .capability(Capability::CancelAllOrders)
            .capability(Capability::FetchBalance)
            .capability(Capability::FetchFundingRate)
            .capability(Capability::FetchFundingRates)
            .capability(Capability::FetchPositions)
            .capability(Capability::SetLeverage)
            .capability(Capability::SetMarginMode)
            .capability(Capability::Websocket)
            .capability(Capability::WatchTicker)
            .capability(Capability::WatchOrderBook)
            .capability(Capability::WatchTrades)
            .capability(Capability::WatchOhlcv)
            .capability(Capability::WatchOrders)
            .build()
    }

    fn timeframes(&self) -> &'static [Timeframe] {
        // HyperLiquid 官方文档确认支持 14 个 timeframe
        // 参考: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint
        // Candle snapshot: "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "8h", "12h", "1d", "3d", "1w", "1M"
        &[
            Timeframe::M1,
            Timeframe::M3,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H2,
            Timeframe::H4,
            Timeframe::H8,
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::D3,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn requests_per_second(&self) -> u32 {
        100
    }
}

//! Exchange trait implementation for HyperLiquid
//!
//! This module implements the unified `Exchange` trait from `ccxt-core` for HyperLiquid.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    exchange::{Exchange, ExchangeCapabilities, PublicExchange},
    traits::Trading,
    types::{Balance, Market, Ohlcv, Order, OrderBook, OrderRequest, Ticker, Timeframe, Trade},
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::hyperliquid::HyperLiquid;

#[async_trait]
impl Exchange for HyperLiquid {
    // ==================== Metadata ====================

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
        <Self as PublicExchange>::capabilities(self)
    }

    fn timeframes(&self) -> &'static [Timeframe] {
        <Self as PublicExchange>::timeframes(self)
    }

    fn requests_per_second(&self) -> u32 {
        <Self as PublicExchange>::requests_per_second(self)
    }

    // ==================== Market Data (Public API) ====================

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let markets = HyperLiquid::fetch_markets(self).await?;
        Ok(markets.values().map(|m| (**m).clone()).collect())
    }

    async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        HyperLiquid::load_markets(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        HyperLiquid::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        let symbols_vec = symbols.map(<[String]>::to_vec);
        HyperLiquid::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        HyperLiquid::fetch_order_book(self, symbol, limit).await
    }

    async fn fetch_market_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        HyperLiquid::fetch_market_trades(self, symbol, limit).await
    }

    async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>> {
        use ccxt_core::traits::MarketData;
        use ccxt_core::types::trading::params::OhlcvParams;

        let params = OhlcvParams {
            timeframe,
            since,
            limit,
            until: None,
            price: None,
        };

        <Self as MarketData>::fetch_ohlcv_with_params(self, symbol, params).await
    }

    // ==================== Trading (Private API) ====================

    async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        <Self as Trading>::create_order(self, request).await
    }

    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        use ccxt_core::traits::Trading;

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on HyperLiquid")
        })?;
        <Self as Trading>::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        use ccxt_core::traits::Trading;

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request(
                "Symbol is required for cancel_all_orders on HyperLiquid",
            )
        })?;
        <Self as Trading>::cancel_all_orders(self, symbol_str).await
    }

    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        use ccxt_core::traits::Trading;

        let symbol_str = symbol.unwrap_or("");
        <Self as Trading>::fetch_order(self, id, symbol_str).await
    }

    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // Exchange trait has since/limit params but Trading trait doesn't use them
        HyperLiquid::fetch_open_orders(self, symbol, since, limit).await
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        use ccxt_core::traits::Trading;

        <Self as Trading>::fetch_history_orders(self, symbol, since, limit).await
    }

    // ==================== Account (Private API) ====================

    async fn fetch_balance(&self) -> Result<Balance> {
        use ccxt_core::traits::Account;
        <Self as Account>::fetch_balance(self).await
    }

    async fn fetch_account_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        use ccxt_core::traits::Account;

        let symbol_str = symbol.unwrap_or("");
        <Self as Account>::fetch_account_trades_since(self, symbol_str, since, limit).await
    }

    // ==================== Helper Methods ====================

    async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        self.base().market(symbol).await
    }

    async fn markets(&self) -> Arc<HashMap<String, Arc<Market>>> {
        let cache = self.base().market_cache.read().await;
        cache.markets()
    }
}

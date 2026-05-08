//! Exchange trait implementation for Bybit.
//!
//! This module wires Bybit into the unified `Exchange` and `PublicExchange`
//! traits so it can participate in shared conformance tests and facade-based
//! usage alongside the other exchanges.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    exchange::{Exchange, ExchangeCapabilities},
    traits::Trading,
    types::{Balance, Market, Ohlcv, Order, OrderRequest, Ticker, Timeframe, Trade},
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::bybit::Bybit;
use crate::bybit::{bybit_capabilities, bybit_timeframes};

#[async_trait]
impl Exchange for Bybit {
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

    fn requests_per_second(&self) -> u32 {
        self.requests_per_second()
    }

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let markets = Bybit::fetch_markets(self).await?;
        Ok(markets.values().map(|market| (**market).clone()).collect())
    }

    async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        Bybit::load_markets(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        Bybit::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        let symbols_vec = symbols.map(<[String]>::to_vec);
        Bybit::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<ccxt_core::types::OrderBook> {
        Bybit::fetch_order_book(self, symbol, limit).await
    }

    async fn fetch_market_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        Bybit::fetch_market_trades(self, symbol, limit).await
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

    async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        <Self as Trading>::create_order(self, request).await
    }

    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        use ccxt_core::traits::Trading;

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on Bybit")
        })?;
        <Self as Trading>::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        use ccxt_core::traits::Trading;

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_all_orders on Bybit")
        })?;
        <Self as Trading>::cancel_all_orders(self, symbol_str).await
    }

    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        use ccxt_core::traits::Trading;

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_order on Bybit")
        })?;
        <Self as Trading>::fetch_order(self, id, symbol_str).await
    }

    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // Exchange trait has since/limit params but Trading trait doesn't use them
        // For Bybit, we delegate to the underlying implementation directly
        Bybit::fetch_open_orders(self, symbol, since, limit).await
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // Exchange trait has since/limit params, delegate to underlying implementation
        Bybit::fetch_history_orders(self, symbol, since, limit).await
    }

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

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request(
                "Symbol is required for fetch_account_trades on Bybit",
            )
        })?;
        <Self as Account>::fetch_account_trades_since(self, symbol_str, since, limit).await
    }

    async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        let cache = self.base().market_cache.read().await;

        if !cache.is_loaded() {
            return Err(ccxt_core::Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }

        cache
            .get_market(symbol)
            .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {symbol} not found")))
    }

    async fn markets(&self) -> Arc<HashMap<String, Arc<Market>>> {
        let cache = self.base().market_cache.read().await;
        cache.markets()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_bybit_exchange_trait_metadata() {
        let exchange = Bybit::new(ExchangeConfig::default()).expect("Bybit should construct");
        let trait_view: &dyn Exchange = &exchange;

        assert_eq!(trait_view.id(), "bybit");
        assert_eq!(trait_view.name(), "Bybit");
        assert_eq!(trait_view.version(), "v5");
    }

    #[test]
    fn test_bybit_exchange_trait_capabilities() {
        let exchange = Bybit::new(ExchangeConfig::default()).expect("Bybit should construct");
        let caps = (&exchange as &dyn Exchange).capabilities();

        assert!(caps.fetch_markets());
        assert!(caps.fetch_ticker());
        assert!(caps.fetch_ohlcv());
        assert!(caps.create_order());
        assert!(caps.fetch_balance());
        assert!(caps.watch_ticker());
        assert!(caps.watch_order_book());
        assert!(caps.watch_trades());
        assert!(caps.cancel_all_orders()); // Bybit now supports cancel_all_orders
    }
}

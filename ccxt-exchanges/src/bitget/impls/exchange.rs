//! Exchange trait implementation for Bitget
//!
//! This module implements the unified `Exchange` trait from `ccxt-core` for Bitget.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    exchange::{Exchange, ExchangeCapabilities},
    traits::Trading,
    types::{Balance, Market, Ohlcv, Order, OrderBook, OrderRequest, Ticker, Timeframe, Trade},
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::bitget::Bitget;
use crate::bitget::{bitget_capabilities, bitget_timeframes};

#[async_trait]
impl Exchange for Bitget {
    // ==================== Metadata ====================

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

    // ==================== Market Data (Public API) ====================

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let arc_markets = Bitget::fetch_markets(self).await?;
        Ok(arc_markets.values().map(|v| (**v).clone()).collect())
    }

    async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        Bitget::load_markets(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        Bitget::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        let symbols_vec = symbols.map(<[String]>::to_vec);
        Bitget::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        Bitget::fetch_order_book(self, symbol, limit).await
    }

    async fn fetch_market_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        Bitget::fetch_market_trades(self, symbol, limit).await
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
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on Bitget")
        })?;
        <Self as Trading>::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        use ccxt_core::traits::Trading;

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_all_orders on Bitget")
        })?;
        <Self as Trading>::cancel_all_orders(self, symbol_str).await
    }

    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        use ccxt_core::traits::Trading;

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_order on Bitget")
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
        // For Bitget, we delegate to the underlying implementation directly
        Bitget::fetch_open_orders(self, symbol, since, limit).await
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // Exchange trait has since/limit params, delegate to underlying implementation
        Bitget::fetch_history_orders(self, symbol, since, limit).await
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

        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request(
                "Symbol is required for fetch_account_trades on Bitget",
            )
        })?;
        <Self as Account>::fetch_account_trades_since(self, symbol_str, since, limit).await
    }

    // ==================== Helper Methods ====================

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
            .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {} not found", symbol)))
    }

    async fn markets(&self) -> Arc<HashMap<String, Arc<Market>>> {
        let cache = self.base().market_cache.read().await;
        cache.markets()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_bitget_exchange_trait_metadata() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        // Test metadata methods via Exchange trait
        let exchange: &dyn Exchange = &bitget;

        assert_eq!(exchange.id(), "bitget");
        assert_eq!(exchange.name(), "Bitget");
        assert_eq!(exchange.version(), "v2");
        assert!(!exchange.is_verified());
    }

    #[test]
    fn test_bitget_exchange_trait_capabilities() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        let exchange: &dyn Exchange = &bitget;
        let caps = exchange.capabilities();

        // Public API capabilities
        assert!(caps.fetch_markets());
        assert!(caps.fetch_ticker());
        assert!(caps.fetch_tickers());
        assert!(caps.fetch_order_book());
        assert!(caps.fetch_trades());
        assert!(caps.fetch_ohlcv());

        // Private API capabilities
        assert!(caps.create_order());
        assert!(caps.cancel_order());
        assert!(caps.fetch_order());
        assert!(caps.fetch_open_orders());
        assert!(caps.fetch_history_orders());
        assert!(caps.fetch_balance());
        assert!(caps.fetch_account_trades());

        // WebSocket capabilities
        assert!(caps.websocket());
        assert!(caps.watch_ticker());
        assert!(caps.watch_order_book());
        assert!(caps.watch_trades());

        // Not implemented capabilities
        assert!(!caps.fetch_currencies());
    }

    #[test]
    fn test_bitget_exchange_trait_timeframes() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        let exchange: &dyn Exchange = &bitget;
        let timeframes = exchange.timeframes();

        assert!(!timeframes.is_empty());
        assert!(timeframes.contains(&Timeframe::M1));
        assert!(timeframes.contains(&Timeframe::M5));
        assert!(timeframes.contains(&Timeframe::M15));
        assert!(timeframes.contains(&Timeframe::H1));
        assert!(timeframes.contains(&Timeframe::H4));
        assert!(timeframes.contains(&Timeframe::D1));
        assert!(timeframes.contains(&Timeframe::W1));
        assert!(timeframes.contains(&Timeframe::Mon1));
    }

    #[test]
    fn test_bitget_exchange_trait_rate_limit() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        let exchange: &dyn Exchange = &bitget;
        assert_eq!(exchange.requests_per_second(), 20);
    }

    #[test]
    fn test_bitget_exchange_trait_object_safety() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        // Test that we can create a trait object (Box<dyn Exchange>)
        let exchange: Box<dyn Exchange> = Box::new(bitget);

        assert_eq!(exchange.id(), "bitget");
        assert_eq!(exchange.name(), "Bitget");
        assert_eq!(exchange.requests_per_second(), 20);
    }

    #[test]
    fn test_bitget_exchange_trait_polymorphic_usage() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        // Test polymorphic usage with &dyn Exchange
        fn check_exchange_metadata(exchange: &dyn Exchange) -> (&str, &str, bool) {
            (
                exchange.id(),
                exchange.name(),
                exchange.capabilities().websocket(),
            )
        }

        let (id, name, has_ws) = check_exchange_metadata(&bitget);
        assert_eq!(id, "bitget");
        assert_eq!(name, "Bitget");
        assert!(has_ws);
    }

    #[test]
    fn test_bitget_capabilities_has_method() {
        let config = ExchangeConfig::default();
        let bitget = Bitget::new(config).unwrap();

        let exchange: &dyn Exchange = &bitget;
        let caps = exchange.capabilities();

        // Test the has() method with CCXT-style camelCase names
        assert!(caps.has("fetchMarkets"));
        assert!(caps.has("fetchTicker"));
        assert!(caps.has("fetchTickers"));
        assert!(caps.has("fetchOrderBook"));
        assert!(caps.has("fetchTrades"));
        assert!(caps.has("fetchOHLCV"));
        assert!(caps.has("createOrder"));
        assert!(caps.has("cancelOrder"));
        assert!(caps.has("fetchOrder"));
        assert!(caps.has("fetchOpenOrders"));
        assert!(caps.has("fetchClosedOrders"));
        assert!(caps.has("fetchBalance"));
        assert!(caps.has("fetchMyTrades"));
        assert!(caps.has("websocket"));

        // Not implemented
        assert!(!caps.has("fetchCurrencies"));
        assert!(!caps.has("unknownCapability"));
    }
}

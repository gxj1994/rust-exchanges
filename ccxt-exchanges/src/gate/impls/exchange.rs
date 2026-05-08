//! Exchange trait implementation for Gate.io
//!
//! This module wires Gate.io into the unified `Exchange` trait so it can participate
//! in shared conformance tests and facade-based usage alongside other exchanges.
//!
//! # Backward Compatibility
//!
//! Gate implements the unified `Exchange` trait, providing a standard interface
//! for all exchange operations including market data, trading, and account management.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    exchange::{Exchange, ExchangeCapabilities},
    traits::{Account, MarketData, Trading},
    types::{Balance, Market, Ohlcv, Order, OrderRequest, Ticker, Timeframe, Trade},
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::gate::Gate;

#[async_trait]
impl Exchange for Gate {
    // ==================== Metadata ====================

    fn id(&self) -> &str {
        <Self as ccxt_core::traits::PublicExchange>::id(self)
    }

    fn name(&self) -> &str {
        <Self as ccxt_core::traits::PublicExchange>::name(self)
    }

    fn version(&self) -> &'static str {
        <Self as ccxt_core::traits::PublicExchange>::version(self)
    }

    fn is_verified(&self) -> bool {
        <Self as ccxt_core::traits::PublicExchange>::is_verified(self)
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        <Self as ccxt_core::traits::PublicExchange>::capabilities(self)
    }

    fn timeframes(&self) -> &'static [Timeframe] {
        <Self as ccxt_core::traits::PublicExchange>::timeframes(self)
    }

    fn requests_per_second(&self) -> u32 {
        <Self as ccxt_core::traits::PublicExchange>::requests_per_second(self)
    }

    // ==================== Market Data ====================

    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        <Self as MarketData>::fetch_markets(self).await
    }

    async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        <Self as MarketData>::load_markets_with_reload(self, reload).await
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        <Self as MarketData>::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, _symbols: Option<&[String]>) -> Result<Vec<Ticker>> {
        // Gate doesn't support batch tickers in current implementation
        // TODO: Implement when fetch_tickers is available
        Err(ccxt_core::Error::not_implemented("fetch_tickers"))
    }

    async fn fetch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<ccxt_core::types::OrderBook> {
        use ccxt_core::types::trading::params::OrderBookParams;

        let params = OrderBookParams {
            limit,
            ..Default::default()
        };

        <Self as MarketData>::fetch_order_book_with_params(self, symbol, params).await
    }

    async fn fetch_market_trades(&self, symbol: &str, limit: Option<u32>) -> Result<Vec<Trade>> {
        <Self as Trading>::fetch_trades_with_limit(self, symbol, None, limit).await
    }

    async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>> {
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

    // ==================== Trading ====================

    async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        <Self as Trading>::create_order(self, request).await
    }

    async fn cancel_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_order on Gate")
        })?;
        <Self as Trading>::cancel_order(self, id, symbol_str).await
    }

    async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for cancel_all_orders on Gate")
        })?;
        <Self as Trading>::cancel_all_orders(self, symbol_str).await
    }

    async fn fetch_order(&self, id: &str, symbol: Option<&str>) -> Result<Order> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_order on Gate")
        })?;
        <Self as Trading>::fetch_order(self, id, symbol_str).await
    }

    async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        <Self as Trading>::fetch_open_orders(self, symbol).await
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        <Self as Trading>::fetch_history_orders(self, symbol, since, limit).await
    }

    // ==================== Account ====================

    async fn fetch_balance(&self) -> Result<Balance> {
        <Self as Account>::fetch_balance(self).await
    }

    async fn fetch_account_trades(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let symbol_str = symbol.ok_or_else(|| {
            ccxt_core::Error::invalid_request("Symbol is required for fetch_account_trades on Gate")
        })?;
        <Self as Account>::fetch_account_trades_since(self, symbol_str, since, limit).await
    }

    // ==================== Market Cache ====================

    async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        <Self as MarketData>::market(self, symbol).await
    }

    async fn markets(&self) -> Arc<HashMap<String, Arc<Market>>> {
        <Self as MarketData>::markets(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::GateBuilder;

    #[test]
    fn test_gate_exchange_trait_metadata() {
        let gate = GateBuilder::default().build().unwrap();
        let trait_view: &dyn Exchange = &gate;

        assert_eq!(trait_view.id(), "gate");
        assert_eq!(trait_view.name(), "Gate.io");
        assert_eq!(trait_view.version(), "v4");
        assert!(!trait_view.is_verified());
    }

    #[test]
    fn test_gate_exchange_trait_capabilities() {
        let gate = GateBuilder::default().build().unwrap();
        let caps = (&gate as &dyn Exchange).capabilities();

        // Gate should have basic capabilities enabled
        assert!(caps.fetch_markets());
        assert!(caps.fetch_ticker());
        // These will be enabled as we implement them
        // assert!(caps.fetch_ohlcv());
        // assert!(caps.create_order());
        // assert!(caps.fetch_balance());
    }

    #[test]
    fn test_gate_exchange_trait_timeframes() {
        let gate = GateBuilder::default().build().unwrap();
        let timeframes = (&gate as &dyn Exchange).timeframes();

        assert!(timeframes.contains(&Timeframe::M1));
        assert!(timeframes.contains(&Timeframe::H1));
        assert!(timeframes.contains(&Timeframe::D1));
    }

    #[test]
    fn test_gate_exchange_trait_object_safety() {
        let gate = GateBuilder::default().build().unwrap();
        let exchange: Box<dyn Exchange> = Box::new(gate);

        // Verify trait object can be created and used
        assert_eq!(exchange.id(), "gate");
        assert_eq!(exchange.name(), "Gate.io");
    }
}

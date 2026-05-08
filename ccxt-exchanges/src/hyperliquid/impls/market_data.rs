use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::MarketData,
    types::{
        Market, Ohlcv, OrderBook, Ticker,
        trading::params::{OhlcvParams, OrderBookParams},
    },
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::hyperliquid::HyperLiquid;

#[async_trait]
impl MarketData for HyperLiquid {
    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let markets = HyperLiquid::fetch_markets(self).await?;
        Ok(markets.values().map(|m| (**m).clone()).collect())
    }

    async fn load_markets_with_reload(
        &self,
        reload: bool,
    ) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        HyperLiquid::load_markets(self, reload).await
    }

    async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        self.base().market(symbol).await
    }

    async fn markets(&self) -> Arc<HashMap<String, Arc<Market>>> {
        let cache = self.base().market_cache.read().await;
        cache.markets()
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        HyperLiquid::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, symbols: &[&str]) -> Result<Vec<Ticker>> {
        let symbols_vec = if symbols.is_empty() {
            None
        } else {
            Some(symbols.iter().map(|s| (*s).to_string()).collect())
        };
        HyperLiquid::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book_with_params(
        &self,
        symbol: &str,
        params: OrderBookParams,
    ) -> Result<OrderBook> {
        HyperLiquid::fetch_order_book(self, symbol, params.limit).await
    }

    async fn fetch_ohlcv_with_params(
        &self,
        symbol: &str,
        params: OhlcvParams,
    ) -> Result<Vec<Ohlcv>> {
        let timeframe_str = params.timeframe.to_string();
        HyperLiquid::fetch_ohlcv(self, symbol, &timeframe_str, params.since, params.limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperliquid::HyperLiquidOptions;
    use ccxt_core::ExchangeConfig;
    use ccxt_core::types::Timeframe;

    #[test]
    fn test_hyperliquid_market_data_trait_object_safety() {
        let options = HyperLiquidOptions::default();
        let hyperliquid =
            HyperLiquid::new_with_options(ExchangeConfig::default(), options, None).unwrap();
        let market_data: Box<dyn MarketData> = Box::new(hyperliquid);

        assert_eq!(market_data.id(), "hyperliquid");
        assert!(market_data.timeframes().contains(&Timeframe::M1));
        assert!(market_data.timeframes().contains(&Timeframe::D1));
    }
}

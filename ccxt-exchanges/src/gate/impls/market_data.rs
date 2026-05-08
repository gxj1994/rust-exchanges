//! Gate.io MarketData trait implementation.

use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::MarketData,
    types::{
        Market, Ohlcv, OrderBook, Ticker,
        market::MarketType,
        trading::params::{OhlcvParams, OrderBookParams},
    },
};
use std::collections::HashMap;
use std::sync::Arc;

use crate::gate::Gate;

#[async_trait]
impl MarketData for Gate {
    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let markets_map = self.load_markets_with_reload(false).await?;
        Ok(markets_map.values().map(|v| (**v).clone()).collect())
    }

    async fn load_markets_with_reload(
        &self,
        reload: bool,
    ) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let is_contract = matches!(
            self.options.default_type,
            ccxt_core::types::common::default_type::DefaultType::Swap
                | ccxt_core::types::common::default_type::DefaultType::Futures
        );

        let loader = || async {
            let markets: Vec<Market> = if is_contract {
                Gate::fetch_contract_markets(self)
                    .await?
                    .values()
                    .map(|arc| (**arc).clone())
                    .collect()
            } else {
                Gate::fetch_spot_markets(self)
                    .await?
                    .values()
                    .map(|arc| (**arc).clone())
                    .collect()
            };
            Ok((markets, None))
        };

        self.base.load_markets_with_loader(reload, loader).await
    }

    async fn market(&self, symbol: &str) -> Result<Arc<Market>> {
        let cache = self.base.market_cache.read().await;

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
        let cache = self.base.market_cache.read().await;
        cache.markets()
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        // Load markets to determine type
        let markets = self.load_markets_with_reload(false).await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {} not found", symbol)))?;

        match market.market_type {
            MarketType::Spot => Gate::fetch_spot_ticker(self, symbol).await,
            MarketType::Swap | MarketType::Futures => {
                Gate::fetch_contract_ticker(self, symbol).await
            }
            _ => Err(ccxt_core::Error::invalid_request(format!(
                "Unsupported market type {:?} for ticker on Gate",
                market.market_type
            ))),
        }
    }

    async fn fetch_tickers(&self, _symbols: &[&str]) -> Result<Vec<Ticker>> {
        // TODO: Implement fetch_tickers for all symbols
        Err(ccxt_core::Error::not_implemented("fetch_tickers"))
    }

    async fn fetch_order_book_with_params(
        &self,
        symbol: &str,
        params: OrderBookParams,
    ) -> Result<OrderBook> {
        // Load markets to determine type
        let markets = self.load_markets_with_reload(false).await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {} not found", symbol)))?;

        match market.market_type {
            MarketType::Spot => Gate::fetch_spot_order_book(self, symbol, params.limit).await,
            MarketType::Swap | MarketType::Futures => {
                Gate::fetch_contract_order_book(self, symbol, params.limit).await
            }
            _ => Err(ccxt_core::Error::invalid_request(format!(
                "Unsupported market type {:?} for order book on Gate",
                market.market_type
            ))),
        }
    }

    async fn fetch_ohlcv_with_params(
        &self,
        symbol: &str,
        params: OhlcvParams,
    ) -> Result<Vec<Ohlcv>> {
        // Load markets and look up the symbol to determine market type
        let markets = self.load_markets_with_reload(false).await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {} not found", symbol)))?;

        // Route to spot or contract OHLCV based on market type
        match market.market_type {
            MarketType::Spot => {
                self.fetch_spot_ohlcv(symbol, params.timeframe, params.since, params.limit)
                    .await
            }
            MarketType::Swap | MarketType::Futures => {
                self.fetch_contract_ohlcv(symbol, params.timeframe, params.since, params.limit)
                    .await
            }
            _ => Err(ccxt_core::Error::invalid_request(format!(
                "Unsupported market type {:?} for OHLCV on Gate",
                market.market_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::GateBuilder;

    #[tokio::test]
    async fn test_market_data_trait_object_safety() {
        let gate = GateBuilder::default().build().unwrap();
        let market_data: Box<dyn MarketData> = Box::new(gate);

        // Just verify the trait object can be created
        drop(market_data);
    }
}

use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::MarketData,
    types::{
        Amount, Market, Ohlcv, OhlcvRequest, OrderBook, Ticker,
        trading::params::{OhlcvParams, OrderBookParams},
    },
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::bybit::Bybit;

#[async_trait]
impl MarketData for Bybit {
    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let markets = Bybit::fetch_markets(self).await?;
        Ok(markets.values().map(|market| (**market).clone()).collect())
    }

    async fn load_markets_with_reload(
        &self,
        reload: bool,
    ) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        Bybit::load_markets(self, reload).await
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

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        Bybit::fetch_ticker(self, symbol).await
    }

    async fn fetch_tickers(&self, symbols: &[&str]) -> Result<Vec<Ticker>> {
        let symbols_vec = if symbols.is_empty() {
            None
        } else {
            Some(symbols.iter().map(|s| (*s).to_string()).collect())
        };
        Bybit::fetch_tickers(self, symbols_vec).await
    }

    async fn fetch_order_book_with_params(
        &self,
        symbol: &str,
        params: OrderBookParams,
    ) -> Result<OrderBook> {
        Bybit::fetch_order_book(self, symbol, params.limit).await
    }

    async fn fetch_ohlcv_with_params(
        &self,
        symbol: &str,
        params: OhlcvParams,
    ) -> Result<Vec<Ohlcv>> {
        let timeframe_str = params.timeframe.to_string();

        let mut builder = OhlcvRequest::builder()
            .symbol(symbol)
            .timeframe(&timeframe_str);

        if let Some(since) = params.since {
            builder = builder.since(since);
        }
        if let Some(limit) = params.limit {
            builder = builder.limit(limit);
        }
        if let Some(until) = params.until {
            builder = builder.until(until);
        }

        let request = builder
            .build()
            .map_err(|e| ccxt_core::Error::invalid_request(e.to_string()))?;

        Bybit::fetch_ohlcv(self, request)
            .await?
            .into_iter()
            .map(|item| -> Result<Ohlcv> {
                Ok(Ohlcv {
                    timestamp: item.timestamp,
                    open: ccxt_core::Price(Decimal::try_from(item.open).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV open",
                            format!("{e}"),
                        ))
                    })?),
                    high: ccxt_core::Price(Decimal::try_from(item.high).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV high",
                            format!("{e}"),
                        ))
                    })?),
                    low: ccxt_core::Price(Decimal::try_from(item.low).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV low",
                            format!("{e}"),
                        ))
                    })?),
                    close: ccxt_core::Price(Decimal::try_from(item.close).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV close",
                            format!("{e}"),
                        ))
                    })?),
                    volume: Amount(Decimal::try_from(item.volume).map_err(|e| {
                        ccxt_core::Error::from(ccxt_core::ParseError::invalid_value(
                            "OHLCV volume",
                            format!("{e}"),
                        ))
                    })?),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;
    use ccxt_core::types::Timeframe;

    #[test]
    fn test_bybit_market_data_trait_object_safety() {
        let bybit = Bybit::new(ExchangeConfig::default()).unwrap();
        let market_data: Box<dyn MarketData> = Box::new(bybit);

        assert_eq!(market_data.id(), "bybit");
        assert!(market_data.timeframes().contains(&Timeframe::M1));
        assert!(market_data.timeframes().contains(&Timeframe::D1));
    }
}

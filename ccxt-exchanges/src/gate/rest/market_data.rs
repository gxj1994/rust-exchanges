//! Gate.io market data operations.
//!
//! This module contains all public market data methods including markets, tickers, and order books.

use crate::gate::parser;
use ccxt_core::{
    Error, ParseError, Result,
    traits::MarketData,
    types::{Market, OrderBook, Ticker},
};
use std::collections::HashMap;
use std::sync::Arc;

use super::Gate;

impl Gate {
    /// Fetch all spot markets from Gate.io.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/spot/currency_pairs`
    ///
    /// # Returns
    ///
    /// Returns a HashMap of market ID to Market struct.
    pub async fn fetch_spot_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let response = self.public_get("/api/v4/spot/currency_pairs", None).await?;

        let markets_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        let mut markets = HashMap::new();

        for market_data in markets_array {
            let market = parser::parse_currency_pair(market_data)?;
            let market_id = market.id.clone();
            let market_arc = Arc::new(market);
            markets.insert(market_id, market_arc);
        }

        Ok(Arc::new(markets))
    }

    /// Fetch spot ticker for a specific symbol.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/spot/tickers?currency_pair=BTC_USDT`
    pub async fn fetch_spot_ticker(&self, symbol: &str) -> Result<Ticker> {
        // Load markets if not already loaded
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))?;

        let currency_pair = Gate::to_exchange_symbol(&market.id);

        let url = format!("/api/v4/spot/tickers?currency_pair={}", currency_pair);
        let response = self.public_get(&url, None).await?;

        let ticker_data = response
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        parser::parse_ticker(ticker_data, Some(market))
    }

    /// Fetch spot order book.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/spot/order_book?currency_pair=BTC_USDT`
    pub async fn fetch_spot_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<OrderBook> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))?;

        let currency_pair = Gate::to_exchange_symbol(&market.id);

        let url = if let Some(l) = limit {
            format!(
                "/api/v4/spot/order_book?currency_pair={}&limit={}",
                currency_pair, l
            )
        } else {
            format!("/api/v4/spot/order_book?currency_pair={}", currency_pair)
        };

        let response = self.public_get(&url, None).await?;

        // Use the parser to parse the order book
        let symbol_obj = ccxt_core::types::Symbol::new_unchecked(symbol);
        parser::parse_order_book(&response, symbol_obj)
    }

    /// Fetch recent market trades.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/spot/trades?currency_pair=BTC_USDT`
    pub async fn fetch_spot_market_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ccxt_core::types::Trade>> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))?;

        let currency_pair = Gate::to_exchange_symbol(&market.id);

        let url = if let Some(l) = limit {
            format!(
                "/api/v4/spot/trades?currency_pair={}&limit={}",
                currency_pair, l
            )
        } else {
            format!("/api/v4/spot/trades?currency_pair={}", currency_pair)
        };

        let response = self.public_get(&url, None).await?;

        let trades_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        trades_array
            .iter()
            .map(|trade_data| parser::parse_trade(trade_data, Some(market)))
            .collect()
    }

    /// Fetch spot OHLCV (candlestick) data.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/spot/candlesticks?currency_pair=BTC_USDT&interval=1h`
    pub async fn fetch_spot_ohlcv(
        &self,
        symbol: &str,
        timeframe: ccxt_core::types::Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<ccxt_core::types::Ohlcv>> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))?;

        let currency_pair = Gate::to_exchange_symbol(&market.id);

        // Map timeframe to Gate interval format
        // Gate 现货官方支持: 1s, 10s, 1m, 5m, 15m, 30m, 1h, 4h, 8h, 1d, 7d, 30d
        // 参考: https://www.gate.com/docs/developers/apiv4/zh_CN/#现货市场-k-线图
        let interval = match timeframe {
            ccxt_core::types::Timeframe::S1 => "1s",
            ccxt_core::types::Timeframe::M1 => "1m",
            ccxt_core::types::Timeframe::M5 => "5m",
            ccxt_core::types::Timeframe::M15 => "15m",
            ccxt_core::types::Timeframe::M30 => "30m",
            ccxt_core::types::Timeframe::H1 => "1h",
            ccxt_core::types::Timeframe::H4 => "4h",
            ccxt_core::types::Timeframe::H8 => "8h",
            ccxt_core::types::Timeframe::D1 => "1d",
            ccxt_core::types::Timeframe::W1 => "7d",
            ccxt_core::types::Timeframe::Mon1 => "30d",
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unsupported timeframe: {:?}. Gate spot supports: 1s, 1m, 5m, 15m, 30m, 1h, 4h, 8h, 1d, 7d, 30d",
                    timeframe
                )));
            }
        };

        let mut url = format!(
            "/api/v4/spot/candlesticks?currency_pair={}&interval={}",
            currency_pair, interval
        );

        if let Some(s) = since {
            url.push_str(&format!("&from={}", s / 1000)); // Convert ms to seconds
        }

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }

        let response = self.public_get(&url, None).await?;

        let candles_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        candles_array
            .iter()
            .map(|candle_data| parser::parse_ohlcv(candle_data, Some(market)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::gate::GateBuilder;

    #[test]
    fn test_get_rest_url() {
        let gate = GateBuilder::default().build().unwrap();
        let url = gate.get_rest_url();
        assert!(url.contains("api.gateio.ws"), "URL: {}", url);
    }
}

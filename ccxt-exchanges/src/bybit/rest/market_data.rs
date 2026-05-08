//! Bybit market data operations.
//!
//! This module contains all public market data methods including tickers,
//! order books, trades, and OHLCV data.

use crate::bybit::{Bybit, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{
        FundingRate, FundingRateHistory, Market, OHLCV, OhlcvRequest, OrderBook, Ticker, Trade,
    },
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use tracing::{debug, info, warn};

impl Bybit {
    /// Fetch all trading markets.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Market`] structures containing market information.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or response parsing fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::bybit::Bybit;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::builder().build()?;
    /// let markets = bybit.fetch_markets().await?;
    /// println!("Found {} markets", markets.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let path = Self::build_api_path("/market/instruments-info");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let instruments = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of instruments",
            ))
        })?;

        let mut markets = Vec::new();
        for instrument in instruments {
            match parser::parse_market(instrument) {
                Ok(market) => markets.push(market),
                Err(e) => {
                    warn!(error = %e, "Failed to parse market");
                }
            }
        }

        // Cache the markets and preserve ownership for the caller
        let markets = self.base().set_markets(markets, None).await?;

        info!("Loaded {} markets for Bybit", markets.len());
        Ok(markets)
    }

    /// Load and cache market data.
    ///
    /// If markets are already loaded and `reload` is false, returns cached data.
    ///
    /// # Arguments
    ///
    /// * `reload` - Whether to force reload market data from the API.
    ///
    /// # Returns
    ///
    /// Returns a `HashMap` containing all market data, keyed by symbol (e.g., "BTC/USDT").
    pub async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        // Acquire the loading lock to serialize concurrent load_markets calls
        // This prevents multiple tasks from making duplicate API calls
        let _loading_guard = self.base().market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.base().market_cache.read().await;
            if cache.is_loaded() && !reload {
                debug!(
                    "Returning cached markets for Bybit ({} markets)",
                    cache.market_count()
                );
                return Ok(cache.markets());
            }
        }

        info!("Loading markets for Bybit (reload: {})", reload);
        let _markets = self.fetch_markets().await?;

        let cache = self.base().market_cache.read().await;
        Ok(cache.markets())
    }

    /// Fetch ticker for a single trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    ///
    /// # Returns
    ///
    /// Returns [`Ticker`] data for the specified symbol.
    pub async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/market/tickers");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());
        params.insert("symbol".to_string(), market.id.clone());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        // Extract timestamp from outer response: {"retCode":0,"result":{...},"time":1234567890}
        let response_time = response.get("time").and_then(|v| v.as_i64());

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let tickers = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of tickers",
            ))
        })?;

        if tickers.is_empty() {
            return Err(Error::bad_symbol(format!("No ticker data for {}", symbol)));
        }

        parser::parse_ticker_with_time(&tickers[0], Some(&market), response_time)
    }

    /// Fetch tickers for multiple trading pairs.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of trading pair symbols; fetches all if `None`.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Ticker`] structures.
    pub async fn fetch_tickers(&self, symbols: Option<Vec<String>>) -> Result<Vec<Ticker>> {
        let cache = self.base().market_cache.read().await;
        if !cache.is_loaded() {
            drop(cache);
            return Err(Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }
        // Build a snapshot of markets by ID for efficient lookup
        let markets_snapshot: std::collections::HashMap<String, Arc<Market>> = cache
            .iter_markets()
            .map(|(_, m)| (m.id.clone(), m))
            .collect();
        drop(cache);

        let path = Self::build_api_path("/market/tickers");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        // Extract timestamp from outer response: {"retCode":0,"result":{...},"time":1234567890}
        let response_time = response.get("time").and_then(|v| v.as_i64());

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let tickers_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of tickers",
            ))
        })?;

        let mut tickers = Vec::new();
        for ticker_data in tickers_array {
            if let Some(symbol_id) = ticker_data["symbol"].as_str() {
                if let Some(market) = markets_snapshot.get(symbol_id) {
                    match parser::parse_ticker_with_time(ticker_data, Some(market), response_time) {
                        Ok(ticker) => {
                            if let Some(ref syms) = symbols {
                                if syms.iter().any(|s| s == ticker.symbol.as_str()) {
                                    tickers.push(ticker);
                                }
                            } else {
                                tickers.push(ticker);
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                symbol = %symbol_id,
                                "Failed to parse ticker"
                            );
                        }
                    }
                }
            }
        }

        Ok(tickers)
    }

    /// Fetch order book for a trading pair.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `limit` - Optional depth limit (valid values: 1-500; default: 25).
    ///
    /// # Returns
    ///
    /// Returns [`OrderBook`] data containing bids and asks.
    pub async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/market/orderbook");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());
        params.insert("symbol".to_string(), market.id.clone());

        // Bybit valid limits: 1-500, default 25
        // Cap to maximum allowed value
        let actual_limit = limit.map_or(25, |l| l.min(500));
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        parser::parse_orderbook(result, market.symbol.as_str().to_string())
    }

    /// Fetch recent public trades.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `limit` - Optional limit on number of trades (maximum: 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures, sorted by timestamp in descending order.
    pub async fn fetch_market_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/market/recent-trade");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());
        params.insert("symbol".to_string(), market.id.clone());

        // Bybit maximum limit is 1000
        let actual_limit = limit.map_or(60, |l| l.min(1000));
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let trades_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of trades",
            ))
        })?;

        let mut trades = Vec::new();
        for trade_data in trades_array {
            match parser::parse_trade(trade_data, Some(&market)) {
                Ok(trade) => trades.push(trade),
                Err(e) => {
                    warn!(error = %e, "Failed to parse trade");
                }
            }
        }

        // Sort by timestamp descending (newest first)
        trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(trades)
    }

    /// Fetch OHLCV (candlestick) data using the builder pattern.
    ///
    /// This is the preferred method for fetching OHLCV data. It accepts an [`OhlcvRequest`]
    /// built using the builder pattern, which provides validation and a more ergonomic API.
    ///
    /// # Arguments
    ///
    /// * `request` - OHLCV request built via [`OhlcvRequest::builder()`]
    ///
    /// # Returns
    ///
    /// Returns a vector of [`OHLCV`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if the market is not found or the API request fails.
    ///
    /// _Requirements: 2.3, 2.6_
    pub async fn fetch_ohlcv(&self, request: OhlcvRequest) -> Result<Vec<OHLCV>> {
        let market = self.base().market(&request.symbol).await?;

        // Convert timeframe to Bybit format
        let timeframes = self.timeframes();
        let bybit_timeframe = timeframes.get(&request.timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", request.timeframe))
        })?;

        let path = Self::build_api_path("/market/kline");
        let mut params = HashMap::new();
        params.insert("category".to_string(), self.get_category().to_string());
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("interval".to_string(), bybit_timeframe.clone());

        // Bybit maximum limit is 1000
        let actual_limit = request.limit.map_or(200, |l| l.min(1000));
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = request.since {
            params.insert("start".to_string(), start_time.to_string());
        }

        if let Some(end_time) = request.until {
            params.insert("end".to_string(), end_time.to_string());
        }

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let candles_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of candles",
            ))
        })?;

        let mut ohlcv = Vec::new();
        for candle_data in candles_array {
            match parser::parse_ohlcv(candle_data) {
                Ok(candle) => ohlcv.push(candle),
                Err(e) => {
                    warn!(error = %e, "Failed to parse OHLCV");
                }
            }
        }

        // Sort by timestamp ascending (oldest first)
        ohlcv.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(ohlcv)
    }

    // ========================================================================
    // Funding Rates
    // ========================================================================

    /// Fetch current funding rate for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol (e.g., "BTC/USDT:USDT")
    ///
    /// # API Endpoint
    ///
    /// `GET /v5/market/tickers`
    ///
    /// # Reference
    ///
    /// https://bybit-exchange.github.io/docs/zh-TW/v5/market/tickers
    pub async fn fetch_funding_rate(&self, symbol: &str) -> Result<FundingRate> {
        let category = self.category_from_symbol(symbol).await?;
        let market = self.base().market(symbol).await?;

        // Build request parameters according to Bybit API spec
        let mut params = HashMap::new();
        params.insert("category".to_string(), category.clone());
        params.insert("symbol".to_string(), market.id.clone());

        let response = self
            .public_request("GET", "/v5/market/tickers", Some(&params))
            .await?;

        // Check response
        let ret_code = response["retCode"].as_i64().unwrap_or(-1);
        if ret_code != 0 {
            let ret_msg = response["retMsg"].as_str().unwrap_or("Unknown error");
            return Err(Error::invalid_request(format!(
                "Bybit fetch_funding_rate failed: retCode={}, retMsg={}",
                ret_code, ret_msg
            )));
        }

        // Validate category in response
        let result_category = response["result"]["category"].as_str().unwrap_or("");
        debug!(
            "Bybit fetch_funding_rate response category: {}",
            result_category
        );

        // Parse funding rate from ticker data
        let list = response["result"]["list"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of tickers",
            ))
        })?;

        if list.is_empty() {
            return Err(Error::invalid_request(format!(
                "No ticker data found for {} (category: {})",
                symbol, category
            )));
        }

        let ticker = &list[0];

        // Parse all available fields according to Bybit API spec
        let funding_rate = ticker["fundingRate"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());

        let next_funding_time = ticker["nextFundingTime"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok());

        let mark_price = ticker["markPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());

        let index_price = ticker["indexPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok());

        // Get timestamp from response if available
        let timestamp = response["time"].as_i64().or_else(|| next_funding_time);

        Ok(FundingRate {
            info: response.clone(),
            symbol: symbol.to_string(),
            mark_price,
            index_price,
            interest_rate: None,
            estimated_settle_price: None,
            funding_rate,
            funding_timestamp: next_funding_time,
            funding_datetime: next_funding_time.and_then(|ts| {
                chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339())
            }),
            previous_funding_rate: None,
            previous_funding_timestamp: None,
            previous_funding_datetime: None,
            timestamp,
            datetime: timestamp.and_then(|ts| {
                chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339())
            }),
        })
    }

    /// Fetch funding rates for multiple symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of unified symbols
    ///
    /// # Returns
    ///
    /// Returns a map of symbol to funding rate
    pub async fn fetch_funding_rates(
        &self,
        symbols: Option<&[String]>,
    ) -> Result<BTreeMap<String, FundingRate>> {
        let mut rates = BTreeMap::new();

        if let Some(syms) = symbols {
            // Fetch for each symbol individually
            for symbol in syms {
                if let Ok(rate) = self.fetch_funding_rate(symbol).await {
                    rates.insert(symbol.clone(), rate);
                }
            }
        } else {
            // Fetch all rates for linear category
            let mut params = std::collections::HashMap::new();
            params.insert("category".to_string(), "linear".to_string());

            let response = self
                .public_request("GET", "/v5/market/tickers", Some(&params))
                .await?;

            let ret_code = response["retCode"].as_i64().unwrap_or(-1);
            if ret_code != 0 {
                let ret_msg = response["retMsg"].as_str().unwrap_or("Unknown error");
                return Err(Error::invalid_request(format!(
                    "Bybit fetch_funding_rates failed: retCode={}, retMsg={}",
                    ret_code, ret_msg
                )));
            }

            let list = response["result"]["list"].as_array().ok_or_else(|| {
                Error::from(ParseError::invalid_format(
                    "data",
                    "Expected array of tickers",
                ))
            })?;

            for ticker in list {
                if let (Some(bybit_symbol), Some(rate_str)) =
                    (ticker["symbol"].as_str(), ticker["fundingRate"].as_str())
                {
                    if let Ok(market) = self.base().market_by_id(bybit_symbol).await {
                        let funding_rate = rate_str.parse::<f64>().ok();

                        rates.insert(
                            market.symbol.as_str().to_string(),
                            FundingRate {
                                info: ticker.clone(),
                                symbol: market.symbol.as_str().to_string(),
                                funding_rate,
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }

        Ok(rates)
    }

    /// Fetch funding rate history for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Unified symbol (e.g., "BTC/USDT:USDT")
    /// * `since` - Optional start timestamp in milliseconds
    /// * `limit` - Optional maximum number of records
    ///
    /// # API Endpoint
    ///
    /// `GET /v5/market/history-funding-rate`
    pub async fn fetch_funding_rate_history(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<FundingRateHistory>> {
        let category = self.category_from_symbol(symbol).await?;
        let market = self.base().market(symbol).await?;

        let mut params = std::collections::HashMap::new();
        params.insert("category".to_string(), category);
        params.insert("symbol".to_string(), market.id.clone());

        if let Some(s) = since {
            params.insert("startTime".to_string(), s.to_string());
        }

        if let Some(l) = limit {
            params.insert("limit".to_string(), l.to_string());
        }

        let response = self
            .public_request("GET", "/v5/market/history-funding-rate", Some(&params))
            .await?;

        // Check response
        let ret_code = response["retCode"].as_i64().unwrap_or(-1);
        if ret_code != 0 {
            let ret_msg = response["retMsg"].as_str().unwrap_or("Unknown error");
            return Err(Error::invalid_request(format!(
                "Bybit fetch_funding_rate_history failed: retCode={}, retMsg={}",
                ret_code, ret_msg
            )));
        }

        let list = response["result"]["list"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of funding rate history",
            ))
        })?;

        let mut history = Vec::new();
        for item in list {
            let funding_rate = item["fundingRate"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());

            let timestamp = item["fundingRateTimestamp"]
                .as_str()
                .and_then(|s| s.parse::<i64>().ok());

            history.push(FundingRateHistory {
                info: item.clone(),
                symbol: symbol.to_string(),
                funding_rate,
                timestamp,
                datetime: timestamp.and_then(|ts| {
                    chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339())
                }),
            });
        }

        Ok(history)
    }
}

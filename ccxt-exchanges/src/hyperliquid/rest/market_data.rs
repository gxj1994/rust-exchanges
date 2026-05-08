//! HyperLiquid market data operations.
//!
//! This module contains all public market data methods including tickers,
//! order books, trades, and OHLCV data.

use crate::hyperliquid::{HyperLiquid, parser};
use ccxt_core::{
    DefaultType, Error, ParseError, Result,
    types::{Market, OrderBook, Ticker, Trade},
};
use rust_decimal::Decimal;
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

impl HyperLiquid {
    /// Fetch all trading markets (perpetuals only).
    pub async fn fetch_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let response = self
            .info_request("meta", serde_json::Value::Object(serde_json::Map::new()))
            .await?;

        let universe = response["universe"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("universe")))?;

        let mut markets = Vec::new();
        for (index, asset) in universe.iter().enumerate() {
            match parser::parse_market(asset, index) {
                Ok(market) => markets.push(market),
                Err(e) => {
                    warn!(error = %e, "Failed to parse market");
                }
            }
        }

        // Cache the markets and preserve ownership for the caller
        let result = self.base().set_markets(markets, None).await?;

        info!("Loaded {} perpetual markets for HyperLiquid", result.len());
        Ok(result)
    }

    /// Fetch spot trading markets.
    ///
    /// # API Endpoint
    ///
    /// POST https://api.hyperliquid.xyz/info
    /// ```json
    /// { "type": "spotMeta" }
    /// ```
    ///
    /// # Response
    ///
    /// Returns spot metadata including:
    /// - `universe`: list of spot trading pairs
    /// - `tokens`: token information
    pub async fn fetch_spot_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let response = self
            .info_request(
                "spotMeta",
                serde_json::Value::Object(serde_json::Map::new()),
            )
            .await?;

        let universe = response["universe"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("universe")))?;

        let mut markets = Vec::new();
        for (index, asset) in universe.iter().enumerate() {
            match parser::parse_spot_market(asset, index) {
                Ok(market) => markets.push(market),
                Err(e) => {
                    warn!(error = %e, "Failed to parse spot market");
                }
            }
        }

        // Cache the spot markets
        let result = self.base().set_markets(markets, None).await?;

        info!("Loaded {} spot markets for HyperLiquid", result.len());
        Ok(result)
    }

    /// Load and cache market data.
    pub async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        // Acquire the loading lock to serialize concurrent load_markets calls
        let _loading_guard = self.base().market_loading_lock.lock().await;

        // Check cache status while holding the lock
        {
            let cache = self.base().market_cache.read().await;
            if cache.is_loaded() && !reload {
                return Ok(cache.markets());
            }
        }

        // Load both perpetual and spot markets
        if self.options.default_type == DefaultType::Spot {
            let _spot_markets = self.fetch_spot_markets().await?;
        } else if self.options.default_type == DefaultType::Swap {
            let _perp_markets = self.fetch_markets().await?;
        } else {
            return Err(Error::MarketNotFound(std::borrow::Cow::Borrowed(
                "default_type Invalid default type",
            )));
        }

        let cache = self.base().market_cache.read().await;
        Ok(cache.markets())
    }

    /// Fetch ticker for a single trading pair.
    ///
    /// # Note
    ///
    /// Hyperliquid does not have a dedicated ticker endpoint. We use `allMids`
    /// which returns mid prices for all coins but does NOT include a timestamp.
    /// This is an API limitation - the timestamp will be set to local time.
    ///
    /// For tickers with proper timestamps, consider using `watch_ticker` via WebSocket
    /// which uses `l2Book` subscription that includes the `time` field.
    pub async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        let market = self.base().market(symbol).await?;
        let response = self
            .info_request("allMids", serde_json::Value::Object(serde_json::Map::new()))
            .await?;

        // Response is a map of asset name to mid price (no timestamp in allMids)
        let mid_price = response[&market.base]
            .as_str()
            .and_then(|s| s.parse::<Decimal>().ok())
            .ok_or_else(|| Error::bad_symbol(format!("No ticker data for {}", symbol)))?;

        parser::parse_ticker(symbol, mid_price, Some(&market))
    }

    /// Fetch tickers for multiple trading pairs.
    pub async fn fetch_tickers(&self, symbols: Option<Vec<String>>) -> Result<Vec<Ticker>> {
        let response = self
            .info_request("allMids", serde_json::Value::Object(serde_json::Map::new()))
            .await?;

        let cache = self.base().market_cache.read().await;
        if !cache.is_loaded() {
            drop(cache);
            return Err(Error::exchange(
                "-1",
                "Markets not loaded. Call load_markets() first.",
            ));
        }
        drop(cache);

        let mut tickers = Vec::new();

        if let Some(obj) = response.as_object() {
            for (asset, price) in obj {
                if let Some(mid_price) = price.as_str().and_then(|s| s.parse::<Decimal>().ok()) {
                    let symbol = format!("{}/USDC:USDC", asset);

                    // Filter by requested symbols if provided
                    if let Some(ref syms) = symbols {
                        if !syms.contains(&symbol) {
                            continue;
                        }
                    }

                    if let Ok(ticker) = parser::parse_ticker(&symbol, mid_price, None) {
                        tickers.push(ticker);
                    }
                }
            }
        }

        Ok(tickers)
    }

    /// Fetch order book for a trading pair.
    pub async fn fetch_order_book(&self, symbol: &str, _limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        let response = self
            .info_request("l2Book", {
                let mut map = serde_json::Map::new();
                map.insert(
                    "coin".to_string(),
                    serde_json::Value::String(market.base.clone()),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        parser::parse_orderbook(&response, symbol.to_string())
    }

    /// Fetch recent public trades.
    pub async fn fetch_market_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;
        let limit = limit.unwrap_or(100).min(1000);

        let response = self
            .info_request("recentTrades", {
                let mut map = serde_json::Map::new();
                map.insert(
                    "coin".to_string(),
                    serde_json::Value::String(market.base.clone()),
                );
                map.insert("n".to_string(), serde_json::Value::Number(limit.into()));
                serde_json::Value::Object(map)
            })
            .await?;

        let trades_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut trades = Vec::new();
        for trade_data in trades_array {
            match parser::parse_trade(trade_data, Some(&market)) {
                Ok(trade) => trades.push(trade),
                Err(e) => {
                    warn!(error = %e, "Failed to parse trade");
                }
            }
        }

        Ok(trades)
    }

    /// Fetch OHLCV (candlestick) data.
    pub async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<ccxt_core::types::Ohlcv>> {
        let market = self.base().market(symbol).await?;
        let limit_val = limit.unwrap_or(500).min(5000) as usize;

        // Convert timeframe to HyperLiquid interval format
        let interval = match timeframe {
            "1m" => "1m",
            "3m" => "3m",
            "5m" => "5m",
            "15m" => "15m",
            "30m" => "30m",
            "1h" => "1h",
            "2h" => "2h",
            "4h" => "4h",
            "8h" => "8h",
            "12h" => "12h",
            "1d" => "1d",
            "3d" => "3d",
            "1w" => "1w",
            "1M" => "1M",
            _ => "1h",
        };

        let now = chrono::Utc::now().timestamp_millis() as u64;
        let start_time = since.map_or(now - limit_val as u64 * 3600000, |s| s as u64);
        let end_time = now;

        // Build the request with proper format: req object contains the parameters
        let req_body = {
            let mut map = serde_json::Map::new();
            map.insert(
                "coin".to_string(),
                serde_json::Value::String(market.base.clone()),
            );
            map.insert(
                "interval".to_string(),
                serde_json::Value::String(interval.to_string()),
            );
            map.insert(
                "startTime".to_string(),
                serde_json::Value::Number(start_time.into()),
            );
            map.insert(
                "endTime".to_string(),
                serde_json::Value::Number(end_time.into()),
            );
            serde_json::Value::Object(map)
        };

        // candleSnapshot expects { "type": "candleSnapshot", "req": {...} }
        let response = self
            .info_request_with_req("candleSnapshot", req_body)
            .await?;

        let candles_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut ohlcv_list = Vec::new();
        for candle_data in candles_array {
            match parser::parse_ohlcv(candle_data) {
                Ok(ohlcv) => ohlcv_list.push(ohlcv),
                Err(e) => {
                    warn!(error = %e, "Failed to parse OHLCV");
                }
            }
        }

        // Sort by timestamp descending and limit the results
        ohlcv_list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        ohlcv_list.truncate(limit_val);
        // Re-sort ascending for consistent output
        ohlcv_list.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(ohlcv_list)
    }

    /// Fetch current funding rate for a symbol.
    pub async fn fetch_funding_rate(&self, symbol: &str) -> Result<ccxt_core::types::FundingRate> {
        let market = self.base().market(symbol).await?;

        let response = self
            .info_request("meta", serde_json::Value::Object(serde_json::Map::new()))
            .await?;

        // Find the asset in the universe
        let universe = response["universe"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("universe")))?;

        let asset_index: usize = market.id.parse().unwrap_or(0);
        let asset_data = universe
            .get(asset_index)
            .ok_or_else(|| Error::bad_symbol(format!("Asset not found: {}", symbol)))?;

        let funding_rate = asset_data["funding"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let timestamp = chrono::Utc::now().timestamp_millis();

        Ok(ccxt_core::types::FundingRate {
            info: serde_json::json!({}),
            symbol: symbol.to_string(),
            mark_price: None,
            index_price: None,
            interest_rate: None,
            estimated_settle_price: None,
            funding_rate: Some(funding_rate),
            funding_timestamp: None,
            funding_datetime: None,
            previous_funding_rate: None,
            previous_funding_timestamp: None,
            previous_funding_datetime: None,
            timestamp: Some(timestamp),
            datetime: parser::timestamp_to_datetime(timestamp),
        })
    }
}

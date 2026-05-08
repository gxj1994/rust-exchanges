//! Market data endpoints for Bitget REST API.

use super::super::{Bitget, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Market, OHLCV, OhlcvRequest, OrderBook, Ticker, Trade},
};
use std::{collections::HashMap, sync::Arc};
use tracing::{info, warn};

impl Bitget {
    /// 将内部 product_type 转换为 V3 API 的 category 参数
    pub(super) fn product_type_to_category(product_type: &str) -> &'static str {
        match product_type {
            // V3 API category values
            "USDT-FUTURES" | "umcbl" => "USDT-FUTURES",
            "COIN-FUTURES" | "dmcbl" => "COIN-FUTURES",
            "USDC-FUTURES" | "sumcbl" => "USDC-FUTURES",
            "SPOT" | "spot" => "SPOT",
            _ => "SPOT", // Default fallback
        }
    }

    /// Fetch all trading markets.
    pub async fn fetch_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        // 使用统一的 /api/v3/market/instruments 端点
        let product_type = self.options().effective_product_type();
        let category = Self::product_type_to_category(product_type);

        let path = "/api/v3/market/instruments".to_string();
        let mut params = HashMap::new();
        params.insert("category".to_string(), category.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let instruments = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of instruments",
            ))
        })?;

        let mut markets = Vec::new();
        for instrument in instruments.iter() {
            match parser::parse_market(instrument) {
                Ok(market) => markets.push(market),
                Err(e) => {
                    warn!(error = %e, "Failed to parse market");
                }
            }
        }

        let result = self.base().set_markets(markets, None).await?;
        Ok(result)
    }

    /// Load and cache market data.
    pub async fn load_markets(&self, reload: bool) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let _loading_guard = self.base().market_loading_lock.lock().await;

        {
            let cache = self.base().market_cache.read().await;
            if cache.is_loaded() && !reload {
                return Ok(cache.markets());
            }
        }

        info!("Loading markets for Bitget (reload: {})", reload);
        let _markets = self.fetch_markets().await?;

        let cache = self.base().market_cache.read().await;
        Ok(cache.markets())
    }

    /// Fetch ticker for a single trading pair.
    pub async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        let market = self.base().market(symbol).await?;

        // 使用 V3 API: /api/v3/market/tickers
        let product_type = self.options().effective_product_type();
        let category = Self::product_type_to_category(product_type);

        let path = "/api/v3/market/tickers".to_string();
        let mut params = HashMap::new();
        params.insert("category".to_string(), category.to_string());
        params.insert("symbol".to_string(), market.id.clone());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let tickers = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of tickers",
            ))
        })?;

        if tickers.is_empty() {
            return Err(Error::bad_symbol(format!("No ticker data for {}", symbol)));
        }

        parser::parse_ticker(&tickers[0], Some(&market))
    }

    /// Fetch tickers for multiple trading pairs.
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

        // 使用 V3 API: /api/v3/market/tickers
        let product_type = self.options().effective_product_type();
        let category = Self::product_type_to_category(product_type);

        let path = "/api/v3/market/tickers".to_string();
        let mut params = HashMap::new();
        params.insert("category".to_string(), category.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let tickers_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of tickers",
            ))
        })?;

        let mut tickers = Vec::new();
        for ticker_data in tickers_array {
            if let Some(bitget_symbol) = ticker_data["symbol"].as_str() {
                if let Some(market) = markets_snapshot.get(bitget_symbol) {
                    match parser::parse_ticker(ticker_data, Some(market)) {
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
                                symbol = %bitget_symbol,
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
    pub async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        let market = self.base().market(symbol).await?;

        // 使用 V3 API: /api/v3/market/orderbook
        let path = "/api/v3/market/orderbook".to_string();
        let mut params = HashMap::new();

        // V3 API 需要 category 参数
        let category = Self::product_type_to_category(&self.options().effective_product_type());
        params.insert("category".to_string(), category.to_string());

        // V3 API 使用 symbol 参数（交易对名称，如 BTCUSDT）
        params.insert("symbol".to_string(), market.id.clone());

        // V3 API limit 参数：默认5，最大200
        let actual_limit = limit.map_or(20, |l| l.min(200));
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        parser::parse_orderbook(data, market.symbol.as_str().to_string())
    }

    /// Fetch OHLCV (candlestick) data.
    pub async fn fetch_ohlcv(&self, request: OhlcvRequest) -> Result<Vec<OHLCV>> {
        let market = self.base().market(&request.symbol).await?;

        let timeframes = self.timeframes();
        let bitget_interval = timeframes.get(&request.timeframe).ok_or_else(|| {
            Error::invalid_request(format!("Unsupported timeframe: {}", request.timeframe))
        })?;

        // 使用 V3 API: /api/v3/market/candles
        let product_type = self.options().effective_product_type();
        let category = Self::product_type_to_category(product_type);

        let path = "/api/v3/market/candles".to_string();
        let mut params = HashMap::new();
        params.insert("category".to_string(), category.to_string());
        params.insert("symbol".to_string(), market.id.clone());
        params.insert("interval".to_string(), bitget_interval.clone());
        params.insert("kLineType".to_string(), "MARKET".to_string());

        let actual_limit = request.limit.map_or(100, |l| l.min(1000));
        params.insert("limit".to_string(), actual_limit.to_string());

        if let Some(start_time) = request.since {
            params.insert("startTime".to_string(), start_time.to_string());
        }

        if let Some(end_time) = request.until {
            params.insert("endTime".to_string(), end_time.to_string());
        }

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let candles_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
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

        ohlcv.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(ohlcv)
    }

    /// Fetch recent trades for a trading pair.
    ///
    /// # API Endpoint
    /// - GET /api/v3/market/fills
    /// - Rate limit: 20 requests/second/IP
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `limit` - Number of trades to fetch (default: 100, max: 100)
    ///
    /// # Returns
    /// Returns a vector of recent trades sorted by timestamp (newest first).
    pub async fn fetch_market_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        // 使用 V3 API: /api/v3/market/fills
        let product_type = self.options().effective_product_type();
        let category = Self::product_type_to_category(product_type);

        let path = "/api/v3/market/fills".to_string();
        let mut params = HashMap::new();
        params.insert("category".to_string(), category.to_string());
        params.insert("symbol".to_string(), market.id.clone());

        // V3 API limit 参数: 默认100, 最大100
        let actual_limit = limit.map_or(100, |l| l.min(100));
        params.insert("limit".to_string(), actual_limit.to_string());

        let response = self.public_request("GET", &path, Some(&params)).await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let trades_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
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

        // API 返回的是按时间倒序 (最新在前), 保持这个顺序
        Ok(trades)
    }
}

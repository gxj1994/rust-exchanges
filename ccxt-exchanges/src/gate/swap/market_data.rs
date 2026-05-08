//! Gate.io contract (swap/futures) market data operations.
//!
//! This module contains all contract market data methods including markets, tickers, and order books.

use crate::gate::parser;
use ccxt_core::{
    Error, ParseError, Result,
    traits::MarketData,
    types::{Market, Ohlcv, OrderBook, Ticker, Trade},
};
use std::collections::HashMap;
use std::sync::Arc;

use super::Gate;

impl Gate {
    /// Fetch all contract markets from Gate.io.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/contracts` (USDT) or `/api/v4/contracts/{settle}` (USDC/BTC)
    pub async fn fetch_contract_markets(&self) -> Result<Arc<HashMap<String, Arc<Market>>>> {
        let settle = self.get_contract_settle();

        let url = format!("/api/v4/futures/{}/contracts", settle);
        let response = self.signed_request(&url).execute().await?;

        let markets_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        let mut markets = HashMap::new();

        for market_data in markets_array {
            let market = parser::parse_contract_market(market_data, settle)?;
            let symbol = market.symbol.as_str().to_string();
            let market_arc = Arc::new(market);
            markets.insert(symbol, market_arc);
        }

        Ok(Arc::new(markets))
    }

    /// Fetch ticker for a specific contract symbol.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/tickers?contract={contract}`
    pub async fn fetch_contract_ticker(&self, symbol: &str) -> Result<Ticker> {
        // Load contract markets if not already loaded
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", symbol)))?;

        let settle = market.settle.as_deref().unwrap_or("usdt").to_lowercase();
        let contract = &market.id;

        // Use query parameter (public endpoint)
        let response = self
            .signed_request(&format!("/api/v4/futures/{}/tickers", settle))
            .param("contract", contract)
            .execute()
            .await?;

        // Gate returns array of tickers
        let tickers = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        if tickers.is_empty() {
            return Err(Error::bad_symbol(format!("No ticker data for {}", symbol)));
        }

        parser::parse_contract_ticker(&tickers[0], Some(&market))
    }

    /// Fetch order book for a specific contract symbol.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/order_book?contract={contract}&limit={limit}`
    pub async fn fetch_contract_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<OrderBook> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", symbol)))?;

        let settle = market.settle.as_deref().unwrap_or("usdt").to_lowercase();
        let contract = &market.id;

        let base_url = self.get_contract_rest_url();
        let url = if let Some(l) = limit {
            format!(
                "{}/api/v4/futures/{}/order_book?contract={}&limit={}",
                base_url, settle, contract, l
            )
        } else {
            format!(
                "{}/api/v4/futures/{}/order_book?contract={}",
                base_url, settle, contract
            )
        };

        self.rate_limiter.wait().await;
        let response = self.base.http_client.get(&url, None).await?;

        let symbol_obj = ccxt_core::types::Symbol::new_unchecked(symbol);
        parser::parse_order_book(&response, symbol_obj)
    }

    /// Fetch recent contract market trades.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/trades?contract={contract}&limit={limit}`
    pub async fn fetch_contract_market_trades(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", symbol)))?;

        let settle = market.settle.as_deref().unwrap_or("usdt").to_lowercase();
        let contract = &market.id;

        let base_url = self.get_contract_rest_url();
        let url = if let Some(l) = limit {
            format!(
                "{}/api/v4/futures/{}/trades?contract={}&limit={}",
                base_url, settle, contract, l
            )
        } else {
            format!(
                "{}/api/v4/futures/{}/trades?contract={}",
                base_url, settle, contract
            )
        };

        self.rate_limiter.wait().await;
        let response = self.base.http_client.get(&url, None).await?;

        let trades_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        trades_array
            .iter()
            .map(|trade_data| parser::parse_trade(trade_data, Some(&market)))
            .collect()
    }

    /// Fetch contract OHLCV (candlestick) data.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/candlesticks?contract={contract}&interval=1h`
    pub async fn fetch_contract_ohlcv(
        &self,
        symbol: &str,
        timeframe: ccxt_core::types::Timeframe,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Ohlcv>> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", symbol)))?;

        let settle = market.settle.as_deref().unwrap_or("usdt").to_lowercase();
        let contract = &market.id;

        // Map timeframe to Gate interval format
        // Gate 合约官方支持: 10s, 1m, 5m, 15m, 30m, 1h, 4h, 8h, 1d, 7d
        // 参考: https://www.gate.com/docs/developers/apiv4/zh_CN/#合约市场-k-线图
        // 注意:
        // - 合约不支持 1s (最小是 10s)
        // - 单次请求最大返回 2000 个点
        // - 支持标记价格和指数价格 K 线
        let interval = match timeframe {
            ccxt_core::types::Timeframe::M1 => "1m",
            ccxt_core::types::Timeframe::M5 => "5m",
            ccxt_core::types::Timeframe::M15 => "15m",
            ccxt_core::types::Timeframe::M30 => "30m",
            ccxt_core::types::Timeframe::H1 => "1h",
            ccxt_core::types::Timeframe::H4 => "4h",
            ccxt_core::types::Timeframe::H8 => "8h",
            ccxt_core::types::Timeframe::D1 => "1d",
            ccxt_core::types::Timeframe::W1 => "7d",
            _ => {
                // 明确指出不支持的 timeframe
                let unsupported_hint = match timeframe {
                    ccxt_core::types::Timeframe::S1 => {
                        " (1s not supported in futures, minimum is 10s)"
                    }
                    ccxt_core::types::Timeframe::Mon1 => " (30d/month not supported in futures)",
                    _ => "",
                };
                return Err(Error::invalid_request(format!(
                    "Unsupported timeframe: {:?}{}. Gate futures supports: 10s, 1m, 5m, 15m, 30m, 1h, 4h, 8h, 1d, 7d",
                    timeframe, unsupported_hint
                )));
            }
        };

        let base_url = self.get_contract_rest_url();
        let mut url = format!(
            "{}/api/v4/futures/{}/candlesticks?contract={}&interval={}",
            base_url, settle, contract, interval
        );

        if let Some(s) = since {
            url.push_str(&format!("&from={}", s / 1000)); // Convert ms to seconds
        }

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }

        self.rate_limiter.wait().await;
        let response = self.base.http_client.get(&url, None).await?;

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
    use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};

    #[test]
    fn test_get_contract_rest_url_swap() {
        let gate = GateBuilder::default()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Linear)
            .build()
            .unwrap();
        let url = gate.get_contract_rest_url();
        assert!(url.contains("fx-api.gateio.ws"), "URL: {}", url);
    }

    #[test]
    fn test_get_contract_rest_url_usdc() {
        let gate = GateBuilder::default()
            .default_type(DefaultType::Swap)
            .default_sub_type(DefaultSubType::Usdc)
            .build()
            .unwrap();
        let url = gate.get_contract_rest_url();
        assert!(url.contains("fx-api.gateio.ws"), "URL: {}", url);
    }
}

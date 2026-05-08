//! Bybit REST API implementation organized by functionality.
//!
//! This module provides a modularized structure for Bybit REST API operations,
//! organized into logical sub-modules by functionality:
//!
//! - `market_data`: Public market data operations (ticker, orderbook, trades, OHLCV)
//! - `account`: Account-related operations (balance, trade history)
//! - `trading`: Trading operations (orders, cancellations)
//! - `margin`: Margin and leverage operations (positions, leverage, margin mode, funding rates)
//!
//! # Usage
//!
//! All methods are implemented directly on the `Bybit` struct, so you can call them
//! directly on any `Bybit` instance:
//!
//! ```no_run
//! # use ccxt_exchanges::bybit::Bybit;
//! # use ccxt_core::ExchangeConfig;
//! # async fn example() -> ccxt_core::Result<()> {
//! let bybit = Bybit::builder().build()?;
//!
//! // Market data methods (from market_data.rs)
//! let ticker = bybit.fetch_ticker("BTC/USDT").await?;
//!
//! // Account methods (from account.rs)
//! // let balance = bybit.fetch_balance().await?;
//!
//! // Trading methods (from trading.rs)
//! // let order = bybit.create_order(request).await?;
//! # Ok(())
//! # }
//! ```

use crate::bybit::{Bybit, BybitAuth};
use ccxt_core::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

pub mod account;
pub mod builder;
pub mod margin;
pub mod market_data;
pub mod position;
pub mod trading;

impl Bybit {
    /// Get the authentication instance if credentials are configured.
    pub fn get_auth(&self) -> Result<BybitAuth> {
        let config = &self.base().config;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| Error::authentication("API key is required"))?;
        let secret = config
            .secret
            .as_ref()
            .ok_or_else(|| Error::authentication("API secret is required"))?;

        Ok(BybitAuth::new(
            api_key.expose_secret().to_string(),
            secret.expose_secret().to_string(),
        ))
    }

    /// Check that required credentials are configured.
    pub fn check_required_credentials(&self) -> Result<()> {
        self.base().check_required_credentials()
    }

    /// Build the API path for Bybit V5 API.
    fn build_api_path(endpoint: &str) -> String {
        format!("/v5{}", endpoint)
    }

    /// Get the category for API requests based on default_type and default_sub_type.
    ///
    /// This method delegates to the public `category()` method to ensure consistency.
    /// Bybit V5 API uses category parameter for filtering:
    /// - `Spot` -> "spot"
    /// - `Swap` + Linear -> "linear"
    /// - `Swap` + Inverse -> "inverse"
    /// - `Futures` + Linear -> "linear"
    /// - `Futures` + Inverse -> "inverse"
    /// - `Option` -> "option"
    fn get_category(&self) -> &str {
        self.category()
    }

    /// Make a public API request (no authentication required).
    async fn public_request(
        &self,
        method: &str,
        path: &str,
        params: Option<&HashMap<String, String>>,
    ) -> Result<Value> {
        let base_url =
            self.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        let mut url = format!("{}{}", base_url, path);

        if let Some(p) = params {
            if !p.is_empty() {
                let query: Vec<String> = p
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect();
                url = format!("{}?{}", url, query.join("&"));
            }
        }

        debug!("Bybit public request: {} {}", method, url);

        let response = match method.to_uppercase().as_str() {
            "GET" => self.base().http_client.get(&url, None).await?,
            "POST" => self.base().http_client.post(&url, None, None).await?,
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        // Check for Bybit error response
        if crate::bybit::core::error::is_error_response(&response) {
            return Err(crate::bybit::core::error::parse_error(&response));
        }

        Ok(response)
    }
}

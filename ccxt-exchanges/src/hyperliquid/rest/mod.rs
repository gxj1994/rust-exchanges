//! HyperLiquid REST API implementation organized by functionality.
//!
//! This module provides a modularized structure for HyperLiquid REST API operations,
//! organized into logical sub-modules by functionality:
//!
//! - `market_data`: Public market data operations (ticker, orderbook, trades, OHLCV)
//! - `account`: Account-related operations (balance, positions)
//! - `trading`: Trading operations (orders, cancellations)
//!
//! # Usage
//!
//! All methods are implemented directly on the `HyperLiquid` struct, so you can call them
//! directly on any `HyperLiquid` instance:
//!
//! ```no_run
//! # use ccxt_exchanges::hyperliquid::HyperLiquid;
//! # async fn example() -> ccxt_core::Result<()> {
//! let exchange = HyperLiquid::builder().build()?;
//!
//! // Market data methods (from market_data.rs)
//! let ticker = exchange.fetch_ticker("BTC/USDC:USDC").await?;
//!
//! // Account methods (from account.rs)
//! // let balance = exchange.fetch_balance().await?;
//!
//! // Trading methods (from trading.rs)
//! // let order = exchange.create_order("BTC/USDC:USDC", OrderType::Market, OrderSide::Buy, amount, None).await?;
//! # Ok(())
//! # }
//! ```

use crate::hyperliquid::HyperLiquid;
use ccxt_core::Result;
use serde_json::Value;
use tracing::debug;

pub mod account;
pub mod builder;
pub mod margin;
pub mod market_data;
pub mod trading;

impl HyperLiquid {
    /// Make a public info API request.
    pub(crate) async fn info_request(&self, request_type: &str, payload: Value) -> Result<Value> {
        let base_url =
            self.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        let url = format!("{}/info", base_url);

        // Build the request body by merging type with payload
        let body = if let Value::Object(map) = payload {
            let mut obj = serde_json::Map::new();
            obj.insert("type".to_string(), Value::String(request_type.to_string()));
            for (k, v) in map {
                obj.insert(k, v);
            }
            Value::Object(obj)
        } else {
            let mut map = serde_json::Map::new();
            map.insert(
                "type".to_string(),
                serde_json::Value::String(request_type.to_string()),
            );
            serde_json::Value::Object(map)
        };

        debug!("HyperLiquid info request: {} {:?}", request_type, body);

        let response = self.base().http_client.post(&url, None, Some(body)).await?;

        if crate::hyperliquid::core::error::is_error_response(&response) {
            return Err(crate::hyperliquid::core::error::parse_error(&response));
        }

        Ok(response)
    }

    /// Make a public info API request with `req` parameter.
    /// Used for endpoints like candleSnapshot that expect { "type": "...", "req": {...} }
    pub(crate) async fn info_request_with_req(
        &self,
        request_type: &str,
        req: Value,
    ) -> Result<Value> {
        let base_url =
            self.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public);
        let url = format!("{}/info", base_url);

        let mut obj = serde_json::Map::new();
        obj.insert("type".to_string(), Value::String(request_type.to_string()));
        obj.insert("req".to_string(), req);
        let body = Value::Object(obj);

        debug!(
            "HyperLiquid info request with req: {} {:?}",
            request_type, body
        );

        let response = self.base().http_client.post(&url, None, Some(body)).await?;

        if crate::hyperliquid::core::error::is_error_response(&response) {
            return Err(crate::hyperliquid::core::error::parse_error(&response));
        }

        Ok(response)
    }
}

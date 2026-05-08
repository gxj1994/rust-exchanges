//! Bitget REST API implementation.
//!
//! Implements all REST API endpoint operations for the Bitget exchange.

mod account;
pub mod builder;
pub mod futures;
mod market_data;
pub mod trading;

use crate::bitget::core::error;
use crate::bitget::{Bitget, BitgetAuth};
use ccxt_core::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

impl Bitget {
    /// Get the authentication instance if credentials are configured.
    pub fn get_auth(&self) -> Result<BitgetAuth> {
        let config = &self.base().config;

        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| Error::authentication("API key is required"))?;
        let secret = config
            .secret
            .as_ref()
            .ok_or_else(|| Error::authentication("API secret is required"))?;
        let passphrase = config
            .password
            .as_ref()
            .ok_or_else(|| Error::authentication("Passphrase is required"))?;

        Ok(BitgetAuth::new(
            api_key.expose_secret().to_string(),
            secret.expose_secret().to_string(),
            passphrase.expose_secret().to_string(),
        ))
    }

    /// Check that required credentials are configured.
    pub fn check_required_credentials(&self) -> Result<()> {
        self.base().check_required_credentials()?;
        if self.base().config.password.is_none() {
            return Err(Error::authentication("Passphrase is required for Bitget"));
        }
        Ok(())
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

        debug!("Bitget public request: {} {}", method, url);

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

        if error::is_error_response(&response) {
            return Err(error::parse_error(&response));
        }

        Ok(response)
    }
}

//! Gate.io signed request builder for authenticated API calls.
//!
//! This module provides a builder pattern for creating authenticated Gate.io API requests,
//! encapsulating the common signing workflow used across all authenticated endpoints.
//!
//! # Overview
//!
//! The `GateSignedRequestBuilder` eliminates code duplication by centralizing:
//! - Credential validation
//! - Timestamp generation
//! - Request signing with HMAC-SHA512
//! - Authentication header injection (KEY, Timestamp, SIGN)
//! - HTTP request execution
//!
//! # Example
//!
//! ```ignore
//! # use ccxt_exchanges::gate::Gate;
//! # use ccxt_exchanges::gate::rest::signed_request::HttpMethod;
//! # use ccxt_core::ExchangeConfig;
//! # async fn example() -> ccxt_core::Result<()> {
//! let gate = Gate::builder().build()?;
//!
//! // GET request with authentication
//! let data = gate.signed_request("/api/v4/spot/accounts")
//!     .execute()
//!     .await?;
//!
//! // POST request with body
//! let data = gate.signed_request("/api/v4/spot/orders")
//!     .method(HttpMethod::Post)
//!     .body(serde_json::json!({
//!         "currency_pair": "BTC_USDT",
//!         "side": "buy",
//!         "amount": "0.001",
//!         "type": "limit",
//!         "price": "50000"
//!     }))
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use super::super::Gate;
use super::core::GateAuth;
use ccxt_core::{Error, ParseError, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;

/// HTTP request methods supported by the signed request builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpMethod {
    /// GET request - parameters in query string
    #[default]
    Get,
    /// POST request - parameters in JSON body
    Post,
    /// DELETE request - for canceling orders
    Delete,
    /// PUT request - for updates
    Put,
}

/// Builder for creating authenticated Gate.io API requests.
///
/// This builder encapsulates the common signing workflow:
/// 1. Credential validation
/// 2. Timestamp generation
/// 3. Request signing with HMAC-SHA512
/// 4. Authentication header injection (KEY, Timestamp, SIGN)
/// 5. HTTP request execution
///
/// # Gate.io Signature Format
///
/// Gate.io uses HMAC-SHA512 for request signing:
/// ```text
/// SIGNATURE = HMAC-SHA512(
///     secret_key,
///     HTTP_METHOD + "\n" +
///     REQUEST_URI + "\n" +
///     REQUEST_BODY + "\n" +
///     TIMESTAMP + "\n" +
///     "sha512" + "\n" +
///     hex(SHA512(REQUEST_BODY))
/// )
/// ```
///
/// # Example
///
/// ```ignore
/// # use ccxt_exchanges::gate::Gate;
/// # use ccxt_exchanges::gate::rest::signed_request::HttpMethod;
/// # use ccxt_core::ExchangeConfig;
/// # async fn example() -> ccxt_core::Result<()> {
/// let gate = Gate::builder().build()?;
///
/// let data = gate.signed_request("/api/v4/spot/orders")
///     .method(HttpMethod::Post)
///     .body(serde_json::json!({
///         "currency_pair": "BTC_USDT",
///         "side": "buy",
///         "type": "limit",
///         "amount": "0.001",
///         "price": "50000"
///     }))
///     .execute()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct GateSignedRequestBuilder<'a> {
    /// Reference to the Gate exchange instance
    gate: &'a Gate,
    /// Request parameters (for GET requests)
    params: BTreeMap<String, String>,
    /// Request body for POST requests
    body: Option<Value>,
    /// API endpoint path
    endpoint: String,
    /// HTTP method for the request
    method: HttpMethod,
}

impl<'a> GateSignedRequestBuilder<'a> {
    /// Creates a new signed request builder.
    ///
    /// # Arguments
    ///
    /// * `gate` - Reference to the Gate exchange instance
    /// * `endpoint` - API endpoint path (e.g., "/api/v4/spot/accounts")
    pub fn new(gate: &'a Gate, endpoint: impl Into<String>) -> Self {
        Self {
            gate,
            params: BTreeMap::new(),
            body: None,
            endpoint: endpoint.into(),
            method: HttpMethod::default(),
        }
    }

    /// Sets the HTTP method for the request.
    ///
    /// Default is GET.
    pub fn method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    /// Adds a query parameter (for GET requests).
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Sets the request body (for POST requests).
    pub fn body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Executes the signed request and returns the response.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Credentials are not configured
    /// - Request signing fails
    /// - HTTP request fails
    /// - Response parsing fails
    pub async fn execute(self) -> Result<Value> {
        // Validate credentials
        let api_key = self
            .gate
            .base
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| Error::authentication("API key not configured"))?;
        let secret = self
            .gate
            .base
            .config
            .secret
            .as_ref()
            .ok_or_else(|| Error::authentication("Secret key not configured"))?;

        let api_key_str = api_key.expose_secret();
        let secret_str = secret.expose_secret();

        #[cfg(test)]
        println!(
            "[DEBUG Gate HTTP] API Key (first 10 chars): {}",
            &api_key_str[..10.min(api_key_str.len())]
        );
        #[cfg(test)]
        println!("[DEBUG Gate HTTP] Endpoint: {}", self.endpoint);
        #[cfg(test)]
        println!("[DEBUG Gate HTTP] Method: {:?}", self.method);

        // Build request body string
        let body_str = match &self.body {
            Some(body) => serde_json::to_string(body)
                .map_err(|e| Error::invalid_request(format!("Failed to serialize body: {}", e)))?,
            None => String::new(),
        };

        #[cfg(test)]
        if !body_str.is_empty() {
            println!("[DEBUG Gate HTTP] Body: {}", body_str);
        }

        // Calculate timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Error::from(ParseError::invalid_format("timestamp", e.to_string())))?
            .as_secs();

        // Build signature string according to Gate API v4 docs:
        // SIGN_STRING = Request Method + "\n" + Request URL + "\n" + Query String + "\n" +
        //               HexEncode(SHA512(Request Payload)) + "\n" + Timestamp
        //
        // For GET requests: Query String contains parameters (e.g., "status=finished&limit=50")
        // For POST/PUT requests: Query String is empty, Payload Hash contains body hash

        let body_hash = hex::encode(Sha512::digest(body_str.as_bytes()));

        #[cfg(test)]
        println!(
            "[DEBUG Gate HTTP] Body SHA512: {}",
            &body_hash[..20.min(body_hash.len())]
        );

        // Build query string for ALL requests (GET/POST/PUT/DELETE)
        let query_string = if self.params.is_empty() {
            String::new()
        } else {
            self.params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        };

        #[cfg(test)]
        let sign_string = format!(
            "{}\n{}\n{}\n{}\n{}",
            match self.method {
                HttpMethod::Get => "GET",
                HttpMethod::Post => "POST",
                HttpMethod::Delete => "DELETE",
                HttpMethod::Put => "PUT",
            },
            self.endpoint,
            query_string,
            body_hash,
            timestamp
        );

        #[cfg(test)]
        println!(
            "[DEBUG Gate HTTP] Sign string (first 100 chars): {}",
            &sign_string[..100.min(sign_string.len())]
        );

        // Sign the request
        let auth = GateAuth::new(api_key_str, secret_str);
        let signature = auth.sign_request(
            match self.method {
                HttpMethod::Get => "GET",
                HttpMethod::Post => "POST",
                HttpMethod::Delete => "DELETE",
                HttpMethod::Put => "PUT",
            },
            &self.endpoint,
            // For GET/DELETE: query string
            // For POST/PUT: empty string (body is hashed separately)
            &query_string,
            &body_hash,
            timestamp,
        )?;

        #[cfg(test)]
        println!(
            "[DEBUG Gate HTTP] Signature: {}",
            &signature[..30.min(signature.len())]
        );

        // Build headers
        let mut headers = HeaderMap::new();
        headers.insert(
            "KEY",
            HeaderValue::from_str(api_key_str)
                .map_err(|_| Error::authentication("Invalid API key"))?,
        );
        headers.insert(
            "Timestamp",
            HeaderValue::from_str(&timestamp.to_string())
                .map_err(|_| Error::authentication("Invalid timestamp"))?,
        );
        headers.insert(
            "SIGN",
            HeaderValue::from_str(&signature)
                .map_err(|_| Error::authentication("Invalid signature"))?,
        );
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        // Build URL - use contract base URL for futures endpoints
        let base_url = if self.endpoint.contains("/futures/") {
            self.gate.get_contract_rest_url()
        } else {
            self.gate.get_rest_url()
        };

        // Append query parameters to URL for ALL methods if present
        let url = if !self.params.is_empty() {
            let query_string = self
                .params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}{}?{}", base_url, self.endpoint, query_string)
        } else {
            format!("{}{}", base_url, self.endpoint)
        };

        #[cfg(test)]
        println!("[DEBUG Gate HTTP] Base URL: {}", base_url);
        #[cfg(test)]
        println!("[DEBUG Gate HTTP] Full URL: {}", url);
        #[cfg(test)]
        println!("[DEBUG Gate HTTP] Timestamp: {}", timestamp);
        #[cfg(test)]
        println!(
            "[DEBUG Gate HTTP] Signature (first 20 chars): {}",
            &signature[..20.min(signature.len())]
        );

        // Execute request
        let http_client = &self.gate.base.http_client;
        #[cfg(test)]
        println!("[DEBUG Gate HTTP] Sending request...");

        let response = match self.method {
            HttpMethod::Get => http_client.get(&url, Some(headers)).await,
            HttpMethod::Post => http_client.post(&url, Some(headers), self.body).await,
            HttpMethod::Delete => http_client.delete(&url, Some(headers), self.body).await,
            HttpMethod::Put => http_client.put(&url, Some(headers), self.body).await,
        };

        #[cfg(test)]
        if let Err(ref e) = response {
            println!("[DEBUG Gate HTTP] Request failed with error: {:?}", e);
        }

        response
    }
}

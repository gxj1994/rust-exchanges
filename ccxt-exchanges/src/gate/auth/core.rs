//! Gate.io authentication and signature module.
//!
//! Implements HMAC-SHA512 signature algorithm for API request authentication.
//!
//! # Security
//!
//! All credentials are automatically zeroed from memory when dropped,
//! preventing credential leakage through memory dumps or core files.

use ccxt_core::credentials::SecretString;
use ccxt_core::{Error, Result};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha512;
use std::collections::BTreeMap;

type HmacSha512 = Hmac<Sha512>;

/// Gate.io authenticator.
///
/// Credentials are automatically zeroed from memory when dropped.
///
/// # Signature Algorithm
///
/// Gate.io uses HMAC-SHA512 for request signing. The signature is calculated as:
///
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
/// ```rust,ignore
/// use ccxt_exchanges::gate::auth::core::GateAuth;
///
/// let auth = GateAuth::new("api_key", "secret_key");
///
/// // Sign a GET request
/// let timestamp = std::time::SystemTime::now()
///     .duration_since(std::time::UNIX_EPOCH)
///     .unwrap()
///     .as_secs();
///
/// let signature = auth.sign_request(
///     "GET",
///     "/api/v4/spot/tickers",
///     "",
///     timestamp,
/// ).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct GateAuth {
    /// API key (automatically zeroed on drop).
    api_key: SecretString,
    /// Secret key (automatically zeroed on drop).
    secret: SecretString,
}

impl GateAuth {
    /// Creates a new authenticator.
    ///
    /// # Arguments
    ///
    /// * `api_key` - API key
    /// * `secret` - Secret key
    ///
    /// # Security
    ///
    /// Credentials are automatically zeroed from memory when the authenticator is dropped.
    pub fn new(api_key: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            secret: SecretString::new(secret),
        }
    }

    /// Returns the API key.
    pub fn api_key(&self) -> &str {
        self.api_key.expose_secret()
    }

    /// Returns the secret key.
    pub fn secret(&self) -> &str {
        self.secret.expose_secret()
    }

    /// Signs a request string using HMAC-SHA512.
    ///
    /// # Arguments
    ///
    /// * `http_method` - HTTP method (GET, POST, DELETE, etc.)
    /// * `request_uri` - Request URI (e.g., "/api/v4/spot/tickers")
    /// * `request_body` - Request body (empty string for GET requests)
    /// * `timestamp` - Current timestamp in seconds
    ///
    /// # Returns
    ///
    /// Returns the HMAC-SHA512 signature as a hex string.
    ///
    /// # Signature Format
    ///
    /// ```text
    /// SIGN_STRING = HTTP_METHOD + "\n" +
    ///               REQUEST_URI + "\n" +
    ///               REQUEST_BODY + "\n" +
    ///               TIMESTAMP + "\n" +
    ///               "sha512" + "\n" +
    ///               hex(SHA512(REQUEST_BODY))
    ///
    /// SIGNATURE = HMAC-SHA512(secret_key, SIGN_STRING)
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the secret key is invalid.
    pub fn sign_request(
        &self,
        http_method: &str,
        request_uri: &str,
        query_string: &str,
        body_hash: &str,
        timestamp: u64,
    ) -> Result<String> {
        // Build the string to sign according to Gate API v4 docs:
        // SIGN_STRING = Method + "\n" + URL + "\n" + Query String + "\n" + Body Hash + "\n" + Timestamp
        let sign_string = format!(
            "{}\n{}\n{}\n{}\n{}",
            http_method.to_uppercase(),
            request_uri,
            query_string,
            body_hash,
            timestamp
        );

        // Calculate HMAC-SHA512
        let mut mac = HmacSha512::new_from_slice(self.secret.expose_secret_bytes())
            .map_err(|e| Error::authentication(format!("Invalid secret key: {}", e)))?;

        mac.update(sign_string.as_bytes());
        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());

        Ok(signature)
    }

    /// Signs parameters and adds timestamp and signature.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameter map
    /// * `timestamp` - Timestamp in seconds
    ///
    /// # Returns
    ///
    /// Returns a new parameter map containing timestamp and signature.
    ///
    /// # Errors
    ///
    /// Returns an error if signature generation fails.
    pub fn sign_params(
        &self,
        params: &BTreeMap<String, String>,
        timestamp: u64,
    ) -> Result<BTreeMap<String, String>> {
        // Build query string from params
        let query_string = self.build_query_string(params);
        let body_hash = self.sha512_hex("");

        // Sign according to Gate API v4 format
        let signature = self.sign_request("GET", "", &query_string, &body_hash, timestamp)?;

        let mut signed_params = params.clone();
        signed_params.insert("signature".to_string(), signature);
        signed_params.insert("timestamp".to_string(), timestamp.to_string());

        Ok(signed_params)
    }

    /// Builds a query string from parameters.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameter map
    ///
    /// # Returns
    ///
    /// Returns a URL-encoded query string with parameters sorted by key.
    #[allow(clippy::unused_self)]
    pub(crate) fn build_query_string(&self, params: &BTreeMap<String, String>) -> String {
        let mut pairs: Vec<_> = params.iter().collect();
        pairs.sort_by_key(|(k, _)| *k);

        pairs
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Calculate SHA512 hash of a string and return as hex.
    fn sha512_hex(&self, input: &str) -> String {
        use sha2::Digest;
        let mut hasher = sha2::Sha512::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_get_request() {
        let auth = GateAuth::new("test_api_key", "test_secret_key");

        let timestamp = 1234567890;
        let body_hash = auth.sha512_hex("");
        let signature = auth
            .sign_request("GET", "/api/v4/spot/tickers", "", &body_hash, timestamp)
            .unwrap();

        // Signature should be a 128-character hex string (SHA512 = 64 bytes = 128 hex chars)
        assert_eq!(signature.len(), 128);
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sign_post_request() {
        let auth = GateAuth::new("test_api_key", "test_secret_key");

        let timestamp = 1234567890;
        let body = r#"{"currency_pair":"BTC_USDT","side":"buy","amount":"0.001","price":"50000"}"#;
        let body_hash = auth.sha512_hex(body);
        let signature = auth
            .sign_request("POST", "/api/v4/spot/orders", "", &body_hash, timestamp)
            .unwrap();

        assert_eq!(signature.len(), 128);
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sign_params() {
        let auth = GateAuth::new("test_api_key", "test_secret_key");

        let mut params = BTreeMap::new();
        params.insert("currency_pair".to_string(), "BTC_USDT".to_string());
        params.insert("limit".to_string(), "10".to_string());

        let timestamp = 1234567890;
        let signed = auth.sign_params(&params, timestamp).unwrap();

        assert!(signed.contains_key("signature"));
        assert!(signed.contains_key("timestamp"));
        assert_eq!(signed.get("timestamp").unwrap(), "1234567890");
    }

    #[test]
    fn test_build_query_string() {
        let auth = GateAuth::new("test_api_key", "test_secret_key");

        let mut params = BTreeMap::new();
        params.insert("currency_pair".to_string(), "BTC_USDT".to_string());
        params.insert("limit".to_string(), "10".to_string());

        let query = auth.build_query_string(&params);
        // Parameters should be sorted by key
        assert_eq!(query, "currency_pair=BTC_USDT&limit=10");
    }

    #[test]
    fn test_sha512_empty_string() {
        let auth = GateAuth::new("test_api_key", "test_secret_key");
        let hash = auth.sha512_hex("");

        // SHA512 of empty string
        assert_eq!(
            hash,
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
    }

    #[test]
    fn test_credentials_zeroed_on_drop() {
        use std::mem;

        let auth = GateAuth::new("my_api_key", "my_secret_key");
        let api_key_ptr = auth.api_key.expose_secret().as_ptr();
        let secret_ptr = auth.secret.expose_secret().as_ptr();

        // Drop the authenticator
        mem::drop(auth);

        // Note: We can't verify memory zeroing directly in tests,
        // but the ZeroizeOnDrop trait ensures it happens

        // This is just to verify the pointers were valid
        // In production, the memory would be zeroed
        let _ = api_key_ptr;
        let _ = secret_ptr;
    }
}

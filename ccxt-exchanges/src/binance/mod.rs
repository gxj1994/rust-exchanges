//! Binance exchange implementation.
//!
//! Supports spot trading, futures trading, and options trading with complete REST API and WebSocket support.

use ccxt_core::types::MarketType;
use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use std::sync::{Arc, Weak};
use std::time::Duration;

pub mod auth;
pub mod core;
mod impls;

pub mod network;
pub mod parser;
pub mod rest;
pub mod ws;

// Backward compatibility aliases
pub use auth as signed_request;
pub use core::constants;
pub use core::options;
pub use core::symbol;
pub use network::endpoint_router;
pub use network::rate_limiter;
pub use network::time_sync;

pub use auth::{BinanceWsAuth, BinanceWsMarket, HttpMethod, SignedRequestBuilder};
pub use core::builder::BinanceBuilder;
pub use core::error::BinanceApiError;
pub use core::options::BinanceOptions;
pub use network::time_sync::{TimeSyncConfig, TimeSyncManager};

use network::rate_limiter::WeightRateLimiter;

/// Binance exchange structure.
#[derive(Debug, Clone)]
pub struct Binance {
    /// Base exchange instance.
    base: BaseExchange,
    /// Binance-specific options.
    options: BinanceOptions,
    /// Time synchronization manager for caching server time offset.
    time_sync: Arc<TimeSyncManager>,
    /// Rate limiter for API requests.
    rate_limiter: Arc<WeightRateLimiter>,
    /// WebSocket client for public channels (new architecture, lazily initialized)
    ws_client: std::sync::OnceLock<ws::BinanceWsClient>,
    /// Weak reference to self for private channels (set by init_for_private_channels)
    self_weak: std::sync::OnceLock<Weak<Self>>,
    /// WebSocket client for private channels (new architecture, lazily initialized)
    ws_client_auth: std::sync::OnceLock<ws::BinanceWsClientAuth>,
}

impl Binance {
    /// Creates a new Binance instance using the builder pattern.
    ///
    /// This is the recommended way to create a Binance instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    ///
    /// let binance = Binance::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .sandbox(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> BinanceBuilder {
        BinanceBuilder::new()
    }

    /// Creates a new Binance instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    /// use ccxt_core::credentials::SecretString;
    ///
    /// let config = ExchangeConfig {
    ///     id: "binance".to_string(),
    ///     name: "Binance".to_string(),
    ///     api_key: Some(SecretString::new("your-api-key")),
    ///     secret: Some(SecretString::new("your-secret")),
    ///     ..Default::default()
    /// };
    ///
    /// let binance = Binance::new(config).unwrap();
    /// ```
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = BinanceOptions::default();
        let time_sync = Arc::new(TimeSyncManager::new());
        let rate_limiter = Arc::new(WeightRateLimiter::new());

        Ok(Self {
            base,
            options,
            time_sync,
            rate_limiter,
            ws_client: std::sync::OnceLock::new(),
            self_weak: std::sync::OnceLock::new(),
            ws_client_auth: std::sync::OnceLock::new(),
        })
    }

    /// Creates a new Binance instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - Binance-specific options.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceOptions};
    /// use ccxt_core::ExchangeConfig;
    /// use ccxt_core::types::common::default_type::DefaultType;
    ///
    /// let config = ExchangeConfig::default();
    /// let options = BinanceOptions {
    ///     default_type: DefaultType::Swap,
    ///     ..Default::default()
    /// };
    ///
    /// let binance = Binance::new_with_options(config, options).unwrap();
    /// ```
    pub fn new_with_options(config: ExchangeConfig, options: BinanceOptions) -> Result<Self> {
        let base = BaseExchange::new(config)?;

        // Create TimeSyncManager with configuration from options
        let time_sync_config = TimeSyncConfig {
            sync_interval: Duration::from_secs(options.time_sync_interval_secs),
            auto_sync: options.auto_time_sync,
            max_offset_drift: options.recv_window as i64,
        };
        let time_sync = Arc::new(TimeSyncManager::with_config(time_sync_config));
        let rate_limiter = Arc::new(WeightRateLimiter::new());

        Ok(Self {
            base,
            options,
            time_sync,
            rate_limiter,
            ws_client: std::sync::OnceLock::new(),
            self_weak: std::sync::OnceLock::new(),
            ws_client_auth: std::sync::OnceLock::new(),
        })
    }

    /// Creates a new Binance futures instance for perpetual contracts.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig::default();
    /// let futures = Binance::new_swap(config).unwrap();
    /// ```
    pub fn new_swap(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = BinanceOptions {
            default_type: DefaultType::Swap,                // Perpetual futures
            default_sub_type: Some(DefaultSubType::Linear), // USDT-margined (U本位)
            ..Default::default()
        };

        // Create TimeSyncManager with configuration from options
        let time_sync_config = TimeSyncConfig {
            sync_interval: Duration::from_secs(options.time_sync_interval_secs),
            auto_sync: options.auto_time_sync,
            max_offset_drift: options.recv_window as i64,
        };
        let time_sync = Arc::new(TimeSyncManager::with_config(time_sync_config));
        let rate_limiter = Arc::new(WeightRateLimiter::new());

        Ok(Self {
            base,
            options,
            time_sync,
            rate_limiter,
            ws_client: std::sync::OnceLock::new(),
            self_weak: std::sync::OnceLock::new(),
            ws_client_auth: std::sync::OnceLock::new(),
        })
    }

    /// Returns a reference to the base exchange.
    pub fn base(&self) -> &BaseExchange {
        &self.base
    }

    /// Returns a mutable reference to the base exchange.
    pub fn base_mut(&mut self) -> &mut BaseExchange {
        &mut self.base
    }

    /// Returns the Binance options.
    pub fn options(&self) -> &BinanceOptions {
        &self.options
    }

    /// Sets the Binance options.
    pub fn set_options(&mut self, options: BinanceOptions) {
        self.options = options;
    }

    /// Returns a reference to the time synchronization manager.
    ///
    /// The `TimeSyncManager` caches the time offset between local system time
    /// and Binance server time, reducing network round-trips for signed requests.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    /// let time_sync = binance.time_sync();
    /// println!("Time sync initialized: {}", time_sync.is_initialized());
    /// ```
    pub fn time_sync(&self) -> &Arc<TimeSyncManager> {
        &self.time_sync
    }

    /// Returns a reference to the rate limiter.
    ///
    /// The `WeightRateLimiter` tracks API usage based on response headers
    /// and provides throttling recommendations to avoid hitting rate limits.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let binance = Binance::new(ExchangeConfig::default()).unwrap();
    /// let rate_limiter = binance.rate_limiter();
    /// println!("Current weight usage: {}%", rate_limiter.weight_usage_ratio() * 100.0);
    /// ```
    pub fn rate_limiter(&self) -> &Arc<WeightRateLimiter> {
        &self.rate_limiter
    }

    /// Creates a new signed request builder for the given endpoint.
    ///
    /// This is the recommended way to make authenticated API requests.
    /// The builder handles credential validation, timestamp generation,
    /// parameter signing, and request execution.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Full API endpoint URL
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::{Binance, HttpMethod};
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Simple GET request
    /// let response = binance.signed_request("https://api.binance.com/api/v3/account")
    ///     .execute()
    ///     .await?;
    ///
    /// // POST request with parameters
    /// let response = binance.signed_request("https://api.binance.com/api/v3/order")
    ///     .method(HttpMethod::Post)
    ///     .param("symbol", "BTCUSDT")
    ///     .param("side", "BUY")
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn signed_request(&self, endpoint: impl Into<String>) -> SignedRequestBuilder<'_> {
        SignedRequestBuilder::new(self, endpoint)
    }

    /// Creates an API-key-only request builder (no signature).
    ///
    /// Used for endpoints that require only the `X-MBX-APIKEY` header,
    /// such as listen key operations.
    pub(crate) fn api_key_request(
        &self,
        endpoint: impl Into<String>,
    ) -> auth::signed_request::ApiKeyRequestBuilder<'_> {
        auth::signed_request::ApiKeyRequestBuilder::new(self, endpoint)
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &'static str {
        "binance"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &'static str {
        "Binance"
    }

    /// Returns the API version.
    pub fn version(&self) -> &'static str {
        "v3"
    }

    /// Returns `true` if the exchange has passed CCXT verification.
    pub fn is_verified(&self) -> bool {
        true
    }

    /// Returns `true` if Pro version (WebSocket) is supported.
    pub fn pro(&self) -> bool {
        true
    }

    /// Returns the rate limit in requests per second.
    pub fn requests_per_second(&self) -> u32 {
        50
    }

    /// Returns `true` if sandbox/testnet mode is enabled.
    ///
    /// Sandbox mode is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.test` is set to `true`
    ///
    /// # Returns
    ///
    /// `true` if sandbox mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new(config).unwrap();
    /// assert!(binance.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.test
    }

    /// Returns the WebSocket client (new architecture).
    ///
    /// Uses the unified GenericWsClient architecture with:
    /// - Message broadcasting (single message loop + multiple subscribers)
    /// - Reference counting for same channel subscriptions
    /// - Automatic reconnection with subscription recovery
    ///
    /// The client is lazily initialized on first access and reused for subsequent calls.
    /// Connection state is queried directly from the underlying client.
    pub fn ws_client(&self) -> &ws::BinanceWsClient {
        self.ws_client
            .get_or_init(|| ws::create_binance_ws_client(self.is_sandbox()))
    }

    /// Initialize for private channels (required for watch_balance, watch_orders, etc.)
    ///
    /// This method must be called before using private channel methods.
    /// It stores a weak reference to self to avoid circular references.
    ///
    /// # Arguments
    ///
    /// - `arc_self`: Arc reference to this Binance instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::sync::Arc;
    ///
    /// let binance = Arc::new(Binance::new(config)?);
    /// Arc::get_mut(&mut binance.clone()).unwrap().init_for_private_channels(binance.clone());
    /// // or use the helper:
    /// let binance = Binance::new_arc(config)?; // creates Arc and initializes
    /// ```
    pub fn init_for_private_channels(self: &Arc<Self>) {
        // Store weak reference to self
        let _ = self.self_weak.set(Arc::downgrade(self));
    }

    /// Check if private channels are initialized
    pub fn is_private_channels_initialized(&self) -> bool {
        self.self_weak.get().is_some()
    }

    /// Returns the WebSocket client for private channels (new architecture).
    ///
    /// Uses the unified GenericWsClient architecture with TokenProvider authentication.
    /// Requires `init_for_private_channels()` to be called first.
    ///
    /// # Returns
    ///
    /// Returns the private WebSocket client, or an error if not initialized.
    pub fn ws_client_auth(&self) -> Result<&ws::BinanceWsClientAuth> {
        self.ws_client_auth.get_or_init(|| {
            // Get weak reference
            let weak = self
                .self_weak
                .get()
                .expect("init_for_private_channels() must be called first");

            // Create auth strategy with weak reference
            let market = BinanceWsMarket::from(MarketType::from(self.options.default_type));
            let auth = auth::BinanceWsAuth::new(weak.clone(), market);

            // Create authenticated client
            ws::BinanceWsClientAuth::new(
                ws::BinanceSubscriptionBuilder,
                ws::BinanceStreamParser,
                ws::BinanceWsEndpointProvider::new(self.is_sandbox()),
                auth,
            )
        });
        Ok(self.ws_client_auth.get().unwrap())
    }

    /// Create an Arc<Binance> and initialize for private channels.
    ///
    /// This is a convenience method that combines:
    /// 1. Creating a Binance instance
    /// 2. Wrapping it in Arc
    /// 3. Initializing for private channels
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let binance = Binance::new_arc(config)?;
    /// // Now you can use watch_balance, watch_orders, etc.
    /// ```
    pub fn new_arc(config: ExchangeConfig) -> Result<Arc<Self>> {
        let binance = Self::new(config)?;
        let arc = Arc::new(binance);
        arc.init_for_private_channels();
        Ok(arc)
    }

    /// Create an Arc<Binance> with options and initialize for private channels.
    pub fn new_arc_with_options(
        config: ExchangeConfig,
        options: BinanceOptions,
    ) -> Result<Arc<Self>> {
        let binance = Self::new_with_options(config, options)?;
        let arc = Arc::new(binance);
        arc.init_for_private_channels();
        Ok(arc)
    }

    /// Returns the supported timeframes.
    pub fn timeframes(&self) -> std::collections::HashMap<String, String> {
        constants::timeframes()
    }

    /// Determines the WebSocket URL based on default_type and default_sub_type.
    ///
    /// This method implements the endpoint routing logic according to:
    /// - Spot/Margin: Uses the standard WebSocket endpoint
    /// - Swap/Futures with Linear sub-type: Uses FAPI WebSocket endpoint
    /// - Swap/Futures with Inverse sub-type: Uses DAPI WebSocket endpoint
    /// - Option: Uses EAPI WebSocket endpoint
    ///
    /// # Returns
    ///
    /// The appropriate WebSocket URL string.
    ///
    /// # Note
    ///
    /// This method uses `default_ws_endpoint_by_options()` internally.
    /// The routing logic is based on exchange options (default_type, default_sub_type).
    pub fn get_ws_url(&self) -> String {
        self.default_ws_endpoint_by_options()
    }

    /// Returns the public REST API base URL based on default_type and default_sub_type.
    ///
    /// This method implements the endpoint routing logic for public REST API calls:
    /// - Spot: Uses the public API endpoint (api.binance.com)
    /// - Margin: Uses the SAPI endpoint (api.binance.com/sapi)
    /// - Swap/Futures with Linear sub-type: Uses FAPI endpoint (fapi.binance.com)
    /// - Swap/Futures with Inverse sub-type: Uses DAPI endpoint (dapi.binance.com)
    /// - Option: Uses EAPI endpoint (eapi.binance.com)
    ///
    /// # Returns
    ///
    /// The appropriate REST API base URL string.
    ///
    /// # Note
    ///
    /// This method uses `default_rest_endpoint_by_options()` internally.
    /// The routing logic is based on exchange options (default_type, default_sub_type).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceOptions};
    /// use ccxt_core::ExchangeConfig;
    /// use ccxt_core::types::common::default_type::{DefaultType, DefaultSubType};
    ///
    /// let options = BinanceOptions {
    ///     default_type: DefaultType::Swap,
    ///     default_sub_type: Some(DefaultSubType::Linear),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new_with_options(ExchangeConfig::default(), options).unwrap();
    /// let url = binance.get_rest_url_public();
    /// assert!(url.contains("fapi.binance.com"));
    /// ```
    pub fn get_rest_url_public(&self) -> String {
        self.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Public)
    }

    /// Returns the private REST API base URL based on default_type and default_sub_type.
    ///
    /// This method implements the endpoint routing logic for private REST API calls:
    /// - Spot: Uses the private API endpoint (api.binance.com)
    /// - Margin: Uses the SAPI endpoint (api.binance.com/sapi)
    /// - Swap/Futures with Linear sub-type: Uses FAPI endpoint (fapi.binance.com)
    /// - Swap/Futures with Inverse sub-type: Uses DAPI endpoint (dapi.binance.com)
    /// - Option: Uses EAPI endpoint (eapi.binance.com)
    ///
    /// # Returns
    ///
    /// The appropriate REST API base URL string.
    ///
    /// # Note
    ///
    /// This method uses `default_rest_endpoint_by_options()` internally.
    /// The routing logic is based on exchange options (default_type, default_sub_type).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::{Binance, BinanceOptions};
    /// use ccxt_core::ExchangeConfig;
    /// use ccxt_core::types::common::default_type::{DefaultType, DefaultSubType};
    ///
    /// let options = BinanceOptions {
    ///     default_type: DefaultType::Swap,
    ///     default_sub_type: Some(DefaultSubType::Inverse),
    ///     ..Default::default()
    /// };
    /// let binance = Binance::new_with_options(ExchangeConfig::default(), options).unwrap();
    /// let url = binance.get_rest_url_private();
    /// assert!(url.contains("dapi.binance.com"));
    /// ```
    pub fn get_rest_url_private(&self) -> String {
        self.default_rest_endpoint_by_options(ccxt_core::types::EndpointType::Private)
    }

    /// Checks if the current default_type is a contract type (Swap, Futures, or Option).
    ///
    /// This is useful for determining whether contract-specific API endpoints should be used.
    ///
    /// # Returns
    ///
    /// `true` if the default_type is Swap, Futures, or Option; `false` otherwise.
    pub fn is_contract_type(&self) -> bool {
        self.options.default_type.is_contract()
    }

    /// Checks if the current configuration uses inverse (coin-margined) contracts.
    ///
    /// # Returns
    ///
    /// `true` if default_sub_type is Inverse; `false` otherwise.
    pub fn is_inverse(&self) -> bool {
        matches!(self.options.default_sub_type, Some(DefaultSubType::Inverse))
    }

    /// Checks if the current configuration uses linear (USDT-margined) contracts.
    ///
    /// # Returns
    ///
    /// `true` if default_sub_type is Linear or not specified (defaults to Linear); `false` otherwise.
    pub fn is_linear(&self) -> bool {
        !self.is_inverse()
    }

    /// Checks a JSON response for Binance API errors and converts them to `Result`.
    ///
    /// Binance API may return errors in the response body even with HTTP 200 status.
    /// The error format is: `{"code": -1121, "msg": "Invalid symbol."}`
    ///
    /// This method should be called after receiving a response from public API endpoints
    /// to ensure proper error handling.
    ///
    /// # Arguments
    ///
    /// * `response` - The JSON response from Binance API
    ///
    /// # Returns
    ///
    /// Returns `Ok(response)` if no error is found, or `Err(CoreError)` if the response
    /// contains a Binance API error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    /// let response = binance.base().http_client.get("https://api.binance.com/api/v3/ticker/price", None).await?;
    /// let validated = binance.check_response(response)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn check_response(&self, response: serde_json::Value) -> Result<serde_json::Value> {
        if let Some(api_error) = BinanceApiError::from_json(&response) {
            return Err(api_error.into());
        }
        Ok(response)
    }

    /// Executes a public GET request with unified error checking and rate limiter updates.
    ///
    /// This method wraps `http_client.get()` to provide:
    /// - Rate limiter updates from response headers
    /// - API error checking (Binance may return errors in the body with HTTP 200)
    ///
    /// All public REST API methods should use this instead of calling `http_client.get()` directly.
    pub(crate) async fn public_get(
        &self,
        url: &str,
        headers: Option<reqwest::header::HeaderMap>,
    ) -> Result<serde_json::Value> {
        // Apply rate limiting before making the request
        if let Some(wait) = self.rate_limiter().wait_duration() {
            tokio::time::sleep(wait).await;
        }

        let data = self.base().http_client.get(url, headers).await?;

        // Update rate limiter from response headers
        if let Some(resp_headers) = data.get("responseHeaders") {
            let rate_info = network::rate_limiter::RateLimitInfo::from_headers(resp_headers);
            if rate_info.has_data() {
                self.rate_limiter().update(rate_info);
            }
        }

        // Check for API errors in response body
        self.check_response(data)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]
    use super::*;

    #[test]
    fn test_binance_creation() {
        let config = ExchangeConfig {
            id: "binance".to_string(),
            name: "Binance".to_string(),
            ..Default::default()
        };

        let binance = Binance::new(config);
        assert!(binance.is_ok());

        let binance = binance.unwrap();
        assert_eq!(binance.id(), "binance");
        assert_eq!(binance.name(), "Binance");
        assert_eq!(binance.version(), "v3");
        assert!(binance.is_verified());
        assert!(binance.pro());
    }

    #[test]
    fn test_timeframes() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();
        let timeframes = binance.timeframes();

        assert!(timeframes.contains_key("1m"));
        assert!(timeframes.contains_key("1h"));
        assert!(timeframes.contains_key("1d"));
        assert_eq!(timeframes.len(), 16);
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let binance = Binance::new(config).unwrap();
        assert!(binance.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_options_test() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            test: true,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        assert!(binance.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();
        assert!(!binance.is_sandbox());
    }

    #[test]
    fn test_binance_options_default() {
        let options = BinanceOptions::default();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
        assert!(!options.adjust_for_time_difference);
        assert_eq!(options.recv_window, 5000);
        assert!(!options.test);
        assert_eq!(options.time_sync_interval_secs, 30);
        assert!(options.auto_time_sync);
    }

    #[test]
    fn test_binance_options_with_default_type() {
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
    }

    #[test]
    fn test_binance_options_serialization() {
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"default_type\":\"swap\""));
        assert!(json.contains("\"default_sub_type\":\"linear\""));
        assert!(json.contains("\"time_sync_interval_secs\":30"));
        assert!(json.contains("\"auto_time_sync\":true"));
    }

    #[test]
    fn test_binance_options_deserialization_with_enum() {
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "swap",
            "default_sub_type": "linear",
            "test": false,
            "time_sync_interval_secs": 60,
            "auto_time_sync": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
        assert_eq!(options.time_sync_interval_secs, 60);
        assert!(!options.auto_time_sync);
    }

    #[test]
    fn test_binance_options_deserialization_legacy_future() {
        // Test backward compatibility with legacy "future" value
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "future",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Swap);
        // Verify defaults are applied for missing fields
        assert_eq!(options.time_sync_interval_secs, 30);
        assert!(options.auto_time_sync);
    }

    #[test]
    fn test_binance_options_deserialization_legacy_delivery() {
        // Test backward compatibility with legacy "delivery" value
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "delivery",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Futures);
    }

    #[test]
    fn test_binance_options_deserialization_without_sub_type() {
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "spot",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
    }

    #[test]
    fn test_binance_options_deserialization_case_insensitive() {
        // Test case-insensitive deserialization
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "SWAP",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Swap);

        // Test mixed case
        let json = r#"{
            "adjust_for_time_difference": false,
            "recv_window": 5000,
            "default_type": "FuTuReS",
            "test": false
        }"#;
        let options: BinanceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Futures);
    }

    #[test]
    fn test_new_futures_uses_swap_type() {
        let config = ExchangeConfig::default();
        let binance = Binance::new_swap(config).unwrap();
        assert_eq!(binance.options().default_type, DefaultType::Swap);
    }

    #[test]
    fn test_get_ws_url_spot() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("stream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_margin() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Margin,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        // Margin uses the same WebSocket as Spot
        assert!(ws_url.contains("stream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_swap_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("fstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_swap_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("dstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_swap_default_sub_type() {
        // When sub_type is not specified, should default to Linear (FAPI)
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: None,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("fstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_futures_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("fstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_futures_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("dstream.binance.com"));
    }

    #[test]
    fn test_get_ws_url_option() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let ws_url = binance.get_ws_url();
        assert!(ws_url.contains("nbstream.binance.com") || ws_url.contains("eoptions"));
    }

    #[test]
    fn test_production_endpoints_has_all_ws_endpoints() {
        use crate::binance::core::endpoints::PRODUCTION_ENDPOINTS;

        let endpoints = &*PRODUCTION_ENDPOINTS;
        assert!(!endpoints.websocket.public.is_empty());
        if let Some(by_market) = &endpoints.websocket.by_market {
            assert!(!by_market.is_empty());
        }
    }

    #[test]
    fn test_testnet_endpoints_has_all_ws_endpoints() {
        use crate::binance::core::endpoints::TESTNET_ENDPOINTS;

        let endpoints = &*TESTNET_ENDPOINTS;
        assert!(!endpoints.websocket.public.is_empty());
        if let Some(by_market) = &endpoints.websocket.by_market {
            assert!(!by_market.is_empty());
        }
    }

    #[test]
    fn test_get_rest_url_public_spot() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("api.binance.com"));
        assert!(url.contains("/api/v3"));
    }

    #[test]
    fn test_get_rest_url_public_margin() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Margin,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("api.binance.com"));
        assert!(url.contains("/sapi/"));
    }

    #[test]
    fn test_get_rest_url_public_swap_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_public_swap_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_public_futures_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_public_futures_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_public_option() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_public();
        assert!(url.contains("eapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_private_swap_linear() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_private();
        assert!(url.contains("fapi.binance.com"));
    }

    #[test]
    fn test_get_rest_url_private_swap_inverse() {
        let config = ExchangeConfig::default();
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        let url = binance.get_rest_url_private();
        assert!(url.contains("dapi.binance.com"));
    }

    #[test]
    fn test_is_contract_type() {
        let config = ExchangeConfig::default();

        // Spot is not a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(!binance.is_contract_type());

        // Margin is not a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Margin,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(!binance.is_contract_type());

        // Swap is a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(binance.is_contract_type());

        // Futures is a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Futures,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(binance.is_contract_type());

        // Option is a contract type
        let options = BinanceOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        assert!(binance.is_contract_type());
    }

    #[test]
    fn test_is_linear_and_is_inverse() {
        let config = ExchangeConfig::default();

        // No sub-type specified defaults to linear
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: None,
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(binance.is_linear());
        assert!(!binance.is_inverse());

        // Explicit linear
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config.clone(), options).unwrap();
        assert!(binance.is_linear());
        assert!(!binance.is_inverse());

        // Explicit inverse
        let options = BinanceOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let binance = Binance::new_with_options(config, options).unwrap();
        assert!(!binance.is_linear());
        assert!(binance.is_inverse());
    }

    // ============================================================
    // Sandbox Mode Market Type URL Selection Tests
    // ============================================================

    // ============================================================
    // check_response Tests
    // ============================================================

    #[test]
    fn test_check_response_success() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Valid response without error
        let response = serde_json::json!({
            "symbol": "BTCUSDT",
            "price": "50000.00"
        });

        let result = binance.check_response(response.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), response);
    }

    #[test]
    fn test_check_response_with_binance_error() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Response with Binance API error
        let response = serde_json::json!({
            "code": -1121,
            "msg": "Invalid symbol."
        });

        let result = binance.check_response(response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid symbol"));
    }

    #[test]
    fn test_check_response_with_rate_limit_error() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Response with rate limit error
        let response = serde_json::json!({
            "code": -1003,
            "msg": "Too many requests; please use the websocket for live updates."
        });

        let result = binance.check_response(response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ccxt_core::error::Error::RateLimit { .. }));
    }

    #[test]
    fn test_check_response_with_auth_error() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Response with authentication error
        let response = serde_json::json!({
            "code": -2015,
            "msg": "Invalid API-key, IP, or permissions for action."
        });

        let result = binance.check_response(response);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ccxt_core::error::Error::Authentication(_)));
    }

    #[test]
    fn test_check_response_array_response() {
        let config = ExchangeConfig::default();
        let binance = Binance::new(config).unwrap();

        // Array response (common for list endpoints)
        let response = serde_json::json!([
            {"symbol": "BTCUSDT", "price": "50000.00"},
            {"symbol": "ETHUSDT", "price": "3000.00"}
        ]);

        let result = binance.check_response(response.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), response);
    }
}

// ============================================================================
// Binance 特有的 WebSocket 方法 (不在 WsExchange trait 中)
// ============================================================================

impl Binance {
    /// Watches the mark price stream for a futures symbol.
    ///
    /// This is a Binance-specific method for futures markets.
    /// Returns the mark price, index price, and funding rate.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading symbol (e.g., "BTC/USDT:USDT")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let binance = Binance::new(config)?;
    /// let mut stream = binance.watch_mark_price("BTC/USDT:USDT").await?;
    /// while let Some(mp) = stream.recv().await {
    ///     println!("Mark price: {}", mp.mark_price);
    /// }
    /// ```
    pub async fn watch_mark_price(
        &self,
        symbol: &str,
    ) -> ccxt_core::error::Result<ccxt_core::ws_exchange::MessageStream<ccxt_core::types::MarkPrice>>
    {
        self.ws_client()
            .watch::<ccxt_core::types::MarkPrice>(symbol, None)
            .await
    }

    /// Watches multiple mark prices for futures symbols.
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of trading symbols (e.g., ["BTC/USDT:USDT", "ETH/USDT:USDT"])
    pub async fn watch_mark_prices(
        &self,
        symbols: &[String],
    ) -> ccxt_core::error::Result<
        ccxt_core::ws_exchange::MessageStream<Vec<ccxt_core::types::MarkPrice>>,
    > {
        use futures::StreamExt;

        if symbols.is_empty() {
            return Err(ccxt_core::error::Error::invalid_request(
                "symbols cannot be empty",
            ));
        }

        // Subscribe to all mark price channels
        let channels: Vec<ccxt_core::ws::SubscriptionChannel> = symbols
            .iter()
            .map(|s| ccxt_core::ws::SubscriptionChannel::mark_price(s))
            .collect();

        let client = self.ws_client();
        client.subscribe(&channels).await?;

        // Create individual streams and merge them
        let mut streams = Vec::new();
        for symbol in symbols {
            let stream = client
                .watch::<ccxt_core::types::MarkPrice>(symbol, None)
                .await?;
            streams.push(stream.map(|r| r.map(|mp| vec![mp])));
        }

        let merged = futures::stream::select_all(streams);
        Ok(Box::pin(merged))
    }

    /// Watches the best bid/ask prices for a symbol.
    ///
    /// This is a Binance-specific method that returns the current best bid and ask.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading symbol (e.g., "BTC/USDT")
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let binance = Binance::new(config)?;
    /// let mut stream = binance.watch_bids_asks("BTC/USDT").await?;
    /// while let Some(ba) = stream.recv().await {
    ///     println!("Best bid: {} @ {}, Best ask: {} @ {}",
    ///              ba.bid_quantity, ba.bid_price, ba.ask_quantity, ba.ask_price);
    /// }
    /// ```
    pub async fn watch_bids_asks(
        &self,
        symbol: &str,
    ) -> ccxt_core::error::Result<ccxt_core::ws_exchange::MessageStream<ccxt_core::types::BidAsk>>
    {
        self.ws_client()
            .watch::<ccxt_core::types::BidAsk>(symbol, None)
            .await
    }

    /// Watches position updates for futures trading.
    ///
    /// This is a private method that requires API credentials.
    /// Make sure to call `init_for_private_channels()` or use `Binance::new_arc()`.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Optional list of symbols to filter (None = all symbols)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let binance = Binance::new_arc(config)?;
    /// let mut stream = binance.watch_positions(None).await?;
    /// while let Some(positions) = stream.recv().await {
    ///     for pos in positions {
    ///         println!("{}: {} contracts @ {}", pos.symbol, pos.contracts.unwrap_or(0.0), pos.entry_price.unwrap_or(0.0));
    ///     }
    /// }
    /// ```
    pub async fn watch_positions(
        &self,
        symbols: Option<&[String]>,
    ) -> ccxt_core::error::Result<
        ccxt_core::ws_exchange::MessageStream<Vec<ccxt_core::types::Position>>,
    > {
        self.base.check_required_credentials().map_err(|_| {
            ccxt_core::error::Error::authentication("API credentials required for watch_positions")
        })?;

        if !self.is_private_channels_initialized() {
            return Err(ccxt_core::error::Error::invalid_request(
                "Private channels not initialized. Use Binance::new_arc() or call init_for_private_channels() first.",
            ));
        }

        let client = self.ws_client_auth()?;

        // Connect with token (automatically gets listenKey)
        let context = ccxt_core::ws::WsContext::new().with_private();
        client.connect_with_token(&context).await?;

        // Watch positions
        client
            .watch::<Vec<ccxt_core::types::Position>>(
                symbols
                    .and_then(|s| s.first().map(|s| s.as_str()))
                    .unwrap_or(""),
                None,
            )
            .await
    }
}

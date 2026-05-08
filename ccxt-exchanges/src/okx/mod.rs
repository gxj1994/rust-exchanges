//! OKX exchange implementation.
//!
//! Supports spot trading and futures trading (USDT-M and Coin-M) with REST API and WebSocket support.
//! OKX uses V5 unified API with HMAC-SHA256 + Base64 authentication.

use ccxt_core::types::Timeframe;
use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod auth;
pub mod core;
mod impls;

pub mod network;
pub mod parser;
pub mod rest;
pub mod ws;

// Backward compatibility alias
pub use auth as signed_request;

pub use auth::OkxAuth;
pub use auth::OkxWsAuth;
pub use auth::signed_request::{HttpMethod, OkxSignedRequestBuilder};
pub use core::error::{OkxErrorCode, is_error_response, parse_error};
pub use core::{OkxBuilder, OkxSymbolConverter};

pub(crate) fn okx_capabilities() -> ccxt_core::ExchangeCapabilities {
    use ccxt_core::exchange::{Capability, ExchangeCapabilities};

    ExchangeCapabilities::builder()
        .market_data()
        .without_capability(Capability::FetchCurrencies)
        .without_capability(Capability::FetchStatus)
        .without_capability(Capability::FetchTime)
        .trading()
        .without_capability(Capability::FetchOrders)
        .without_capability(Capability::FetchCanceledOrders)
        .capability(Capability::FetchBalance)
        .capability(Capability::FetchMyTrades)
        .capability(Capability::FetchPositions)
        .capability(Capability::SetLeverage)
        .capability(Capability::SetMarginMode)
        .capability(Capability::FetchFundingRate)
        .capability(Capability::FetchFundingRates)
        .capability(Capability::Websocket)
        .capability(Capability::WatchTicker)
        .capability(Capability::WatchOrderBook)
        .capability(Capability::WatchTrades)
        .capability(Capability::WatchOhlcv)
        .capability(Capability::WatchBalance)
        .capability(Capability::WatchOrders)
        .capability(Capability::WatchMyTrades)
        .build()
}

pub(crate) fn okx_timeframes() -> &'static [Timeframe] {
    &[
        Timeframe::M1,
        Timeframe::M3,
        Timeframe::M5,
        Timeframe::M15,
        Timeframe::M30,
        Timeframe::H1,
        Timeframe::H2,
        Timeframe::H4,
        Timeframe::H6,
        Timeframe::H12,
        Timeframe::D1,
        Timeframe::W1,
        Timeframe::Mon1,
    ]
}

/// OKX exchange structure.
#[derive(Debug, Clone)]
pub struct Okx {
    /// Base exchange instance.
    base: BaseExchange,
    /// OKX-specific options.
    options: OkxOptions,
    /// WebSocket client (lazily initialized).
    ws_client: std::sync::OnceLock<ws::OkxWsClient>,
}

/// OKX-specific options.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::okx::OkxOptions;
/// use ccxt_core::types::common::default_type::{DefaultType, DefaultSubType};
///
/// let options = OkxOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Linear),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxOptions {
    /// Account mode: cash (spot), cross (cross margin), isolated (isolated margin).
    ///
    /// This is kept for backward compatibility with existing configurations.
    pub account_mode: String,
    /// Default trading type (spot/margin/swap/futures/option).
    ///
    /// This determines which instrument type (instType) to use for API calls.
    /// OKX uses a unified V5 API, so this primarily affects market filtering
    /// rather than endpoint selection.
    #[serde(default)]
    pub default_type: DefaultType,
    /// Default sub-type for contract settlement (linear/inverse).
    ///
    /// - `Linear`: USDT-margined contracts
    /// - `Inverse`: Coin-margined contracts
    ///
    /// Only applicable when `default_type` is `Swap`, `Futures`, or `Option`.
    /// Ignored for `Spot` and `Margin` types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,
    /// Enables demo trading environment.
    pub testnet: bool,
}

impl Default for OkxOptions {
    fn default() -> Self {
        Self {
            account_mode: "cash".to_string(),
            default_type: DefaultType::default(), // Defaults to Spot
            default_sub_type: None,
            testnet: false,
        }
    }
}

impl Okx {
    /// Creates a new OKX instance using the builder pattern.
    ///
    /// This is the recommended way to create an OKX instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::Okx;
    ///
    /// let okx = Okx::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .passphrase("your-passphrase")
    ///     .sandbox(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> OkxBuilder {
        OkxBuilder::new()
    }

    /// Creates a new OKX instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = OkxOptions::default();

        Ok(Self {
            base,
            options,
            ws_client: std::sync::OnceLock::new(),
        })
    }

    /// Creates a new OKX instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - OKX-specific options.
    pub fn new_with_options(config: ExchangeConfig, options: OkxOptions) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        Ok(Self {
            base,
            options,
            ws_client: std::sync::OnceLock::new(),
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

    /// Returns the OKX options.
    pub fn options(&self) -> &OkxOptions {
        &self.options
    }

    /// Sets the OKX options.
    pub fn set_options(&mut self, options: OkxOptions) {
        self.options = options;
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &'static str {
        "okx"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &'static str {
        "OKX"
    }

    /// Returns the API version.
    pub fn version(&self) -> &'static str {
        "v5"
    }

    /// Returns `true` if the exchange has passed CCXT verification.
    pub fn is_verified(&self) -> bool {
        false
    }

    /// Returns `true` if Pro version (WebSocket) is supported.
    pub fn pro(&self) -> bool {
        true
    }

    /// Returns the rate limit in requests per second.
    pub fn requests_per_second(&self) -> u32 {
        20
    }

    /// Returns `true` if sandbox/demo mode is enabled.
    ///
    /// Sandbox mode is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.demo` is set to `true`
    ///
    /// # Returns
    ///
    /// `true` if sandbox mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::Okx;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let okx = Okx::new(config).unwrap();
    /// assert!(okx.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns `true` if demo trading mode is enabled.
    ///
    /// This is an OKX-specific alias for `is_sandbox()`. Demo trading mode
    /// is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.demo` is set to `true`
    ///
    /// When demo trading is enabled, the client will:
    /// - Add the `x-simulated-trading: 1` header to all REST API requests
    /// - Use demo WebSocket URLs (`wss://wspap.okx.com:8443/ws/v5/*?brokerId=9999`)
    /// - Continue using the production REST domain (`https://www.okx.com`)
    ///
    /// # Returns
    ///
    /// `true` if demo trading mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::Okx;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let okx = Okx::new(config).unwrap();
    /// assert!(okx.is_testnet_trading());
    /// ```
    pub fn is_testnet_trading(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns the supported timeframes.
    pub fn timeframes(&self) -> HashMap<String, String> {
        let mut timeframes = HashMap::new();
        timeframes.insert("1m".to_string(), "1m".to_string());
        timeframes.insert("3m".to_string(), "3m".to_string());
        timeframes.insert("5m".to_string(), "5m".to_string());
        timeframes.insert("15m".to_string(), "15m".to_string());
        timeframes.insert("30m".to_string(), "30m".to_string());
        timeframes.insert("1h".to_string(), "1H".to_string());
        timeframes.insert("2h".to_string(), "2H".to_string());
        timeframes.insert("4h".to_string(), "4H".to_string());
        timeframes.insert("6h".to_string(), "6Hutc".to_string());
        timeframes.insert("12h".to_string(), "12Hutc".to_string());
        timeframes.insert("1d".to_string(), "1Dutc".to_string());
        timeframes.insert("1w".to_string(), "1Wutc".to_string());
        timeframes.insert("1M".to_string(), "1Mutc".to_string());
        timeframes
    }

    /// Returns the default type configuration.
    pub fn default_type(&self) -> DefaultType {
        self.options.default_type
    }

    /// Returns the default sub-type configuration.
    pub fn default_sub_type(&self) -> Option<DefaultSubType> {
        self.options.default_sub_type
    }

    /// Checks if the current default_type is a contract type (Swap, Futures, or Option).
    ///
    /// This is useful for determining whether contract-specific API parameters should be used.
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

    /// Returns the WebSocket client (unified architecture), initializing it if necessary.
    ///
    /// This provides a shared WebSocket client that supports:
    /// - Message broadcasting (single message loop + multiple subscribers)
    /// - Reference counting for same channel subscriptions
    /// - Automatic reconnection with subscription recovery
    ///
    /// The client is lazily initialized on first access and reused for subsequent calls.
    /// Connection state is queried directly from the underlying client.
    pub fn ws_client(&self) -> &ws::OkxWsClient {
        self.ws_client
            .get_or_init(|| ws::create_okx_ws_client(self.is_sandbox()))
    }

    /// Creates a new signed request builder for authenticated API requests.
    ///
    /// This builder encapsulates the common signing workflow for OKX API requests,
    /// including credential validation, timestamp generation, HMAC-SHA256 signing,
    /// and authentication header injection.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - API endpoint path (e.g., "/api/v5/account/balance")
    ///
    /// # Returns
    ///
    /// A `OkxSignedRequestBuilder` instance for fluent API construction.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::okx::Okx;
    /// use ccxt_exchanges::okx::signed_request::HttpMethod;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let okx = Okx::new(ExchangeConfig::default())?;
    ///
    /// // Simple GET request
    /// let data = okx.signed_request("/api/v5/account/balance")
    ///     .execute()
    ///     .await?;
    ///
    /// // POST request with parameters
    /// let data = okx.signed_request("/api/v5/trade/order")
    ///     .method(HttpMethod::Post)
    ///     .param("instId", "BTC-USDT")
    ///     .param("tdMode", "cash")
    ///     .param("side", "buy")
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn signed_request(
        &self,
        endpoint: impl Into<String>,
    ) -> signed_request::OkxSignedRequestBuilder<'_> {
        signed_request::OkxSignedRequestBuilder::new(self, endpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okx_creation() {
        let config = ExchangeConfig {
            id: "okx".to_string(),
            name: "OKX".to_string(),
            ..Default::default()
        };

        let okx = Okx::new(config);
        assert!(okx.is_ok());

        let okx = okx.unwrap();
        assert_eq!(okx.id(), "okx");
        assert_eq!(okx.name(), "OKX");
        assert_eq!(okx.version(), "v5");
        assert!(!okx.is_verified());
        assert!(okx.pro());
    }

    #[test]
    fn test_timeframes() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        let timeframes = okx.timeframes();

        assert!(timeframes.contains_key("1m"));
        assert!(timeframes.contains_key("1h"));
        assert!(timeframes.contains_key("1d"));
        assert_eq!(timeframes.len(), 13);
    }

    #[test]
    fn test_is_sandbox_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let okx = Okx::new(config).unwrap();
        assert!(okx.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_options_demo() {
        let config = ExchangeConfig::default();
        let options = OkxOptions {
            testnet: true,
            ..Default::default()
        };
        let okx = Okx::new_with_options(config, options).unwrap();
        assert!(okx.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        assert!(!okx.is_sandbox());
    }

    #[test]
    fn test_is_demo_trading_with_config_sandbox() {
        let config = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let okx = Okx::new(config).unwrap();
        assert!(okx.is_testnet_trading());
    }

    #[test]
    fn test_is_demo_trading_with_options_demo() {
        let config = ExchangeConfig::default();
        let options = OkxOptions {
            testnet: true,
            ..Default::default()
        };
        let okx = Okx::new_with_options(config, options).unwrap();
        assert!(okx.is_testnet_trading());
    }

    #[test]
    fn test_is_demo_trading_false_by_default() {
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        assert!(!okx.is_testnet_trading());
    }

    #[test]
    fn test_is_demo_trading_equals_is_sandbox() {
        // Test that is_demo_trading() and is_sandbox() return the same value
        let config = ExchangeConfig::default();
        let okx = Okx::new(config).unwrap();
        assert_eq!(okx.is_testnet_trading(), okx.is_sandbox());

        let config_sandbox = ExchangeConfig {
            sandbox: true,
            ..Default::default()
        };
        let okx_sandbox = Okx::new(config_sandbox).unwrap();
        assert_eq!(okx_sandbox.is_testnet_trading(), okx_sandbox.is_sandbox());
    }

    #[test]
    fn test_default_options() {
        let options = OkxOptions::default();
        assert_eq!(options.account_mode, "cash");
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
        assert!(!options.testnet);
    }

    #[test]
    fn test_okx_options_with_default_type() {
        let options = OkxOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
    }

    #[test]
    fn test_okx_options_serialization() {
        let options = OkxOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"default_type\":\"swap\""));
        assert!(json.contains("\"default_sub_type\":\"linear\""));
    }

    #[test]
    fn test_okx_options_deserialization() {
        let json = r#"{
            "account_mode": "cross",
            "default_type": "swap",
            "default_sub_type": "inverse",
            "testnet": true
        }"#;
        let options: OkxOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.account_mode, "cross");
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Inverse));
        assert!(options.testnet);
    }

    #[test]
    fn test_okx_options_deserialization_without_default_type() {
        // Test backward compatibility - default_type should default to Spot
        let json = r#"{
            "account_mode": "cash",
            "testnet": false
        }"#;
        let options: OkxOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
    }
}

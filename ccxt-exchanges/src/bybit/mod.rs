//! Bybit exchange implementation.
//!
//! Supports spot trading and futures trading (USDT-M and Coin-M) with REST API and WebSocket support.
//! Bybit uses V5 unified account API with HMAC-SHA256 authentication.

use ccxt_core::types::Timeframe;
use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod auth;
pub mod core;
pub(crate) mod impls;
pub mod network;
pub mod parser;
pub mod rest;
pub mod ws;

// Backward compatibility alias
pub use auth as signed_request;

pub use auth::{BybitAuth, BybitWsAuth};
pub use core::error::{BybitErrorCode, is_error_response, parse_error};
pub use core::{BybitBuilder, BybitSymbolConverter};

pub(crate) fn bybit_capabilities() -> ccxt_core::ExchangeCapabilities {
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
        .capability(Capability::SetLeverage)
        .capability(Capability::SetMarginMode)
        .capability(Capability::FetchPositions)
        .capability(Capability::FetchFundingRate)
        .capability(Capability::FetchFundingRates)
        .capability(Capability::Websocket)
        .capability(Capability::WatchTicker)
        .capability(Capability::WatchTickers)
        .capability(Capability::WatchOrderBook)
        .capability(Capability::WatchTrades)
        .capability(Capability::WatchOhlcv)
        .build()
}

pub(crate) fn bybit_timeframes() -> &'static [Timeframe] {
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

/// 将 CCXT 统一 timeframe 格式转换为 Bybit 专用 interval 格式。
///
/// Bybit V5 API（REST 和 WebSocket）使用纯数字或单字母的 interval 格式：
/// - `"1"`, `"3"`, `"5"`, `"15"`, `"30"`（分钟，数字）
/// - `"60"`, `"120"`, `"240"`, `"360"`, `"720"`（小时，换算为分钟）
/// - `"D"`（天）, `"W"`（周）, `"M"`（月）
///
/// CCXT 统一格式（如 `"1m"`, `"1h"`）需要转换为 Bybit 格式后方可用于 API 请求。
///
/// 参考：https://bybit-exchange.github.io/docs/v5/websocket/public/kline
pub(crate) fn to_bybit_interval(ccxt_interval: &str) -> Option<&'static str> {
    match ccxt_interval {
        "1m" => Some("1"),
        "3m" => Some("3"),
        "5m" => Some("5"),
        "15m" => Some("15"),
        "30m" => Some("30"),
        "1h" => Some("60"),
        "2h" => Some("120"),
        "4h" => Some("240"),
        "6h" => Some("360"),
        "8h" => Some("480"),
        "12h" => Some("720"),
        "1d" => Some("D"),
        "3d" => Some("D"), // Bybit 不支持 3 天，回退到日线
        "1w" => Some("W"),
        "1M" => Some("M"),
        _ => None,
    }
}

/// Bybit exchange structure.
#[derive(Debug, Clone)]
pub struct Bybit {
    /// Base exchange instance.
    base: BaseExchange,
    /// Bybit-specific options.
    options: BybitOptions,
    /// WebSocket client (lazily initialized).
    ws_client: std::sync::OnceLock<ws::BybitWsClient>,
}

/// Bybit-specific options.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::bybit::BybitOptions;
/// use ccxt_core::types::common::default_type::{DefaultType, DefaultSubType};
///
/// let options = BybitOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Linear),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BybitOptions {
    /// Account type: UNIFIED, CONTRACT, SPOT.
    ///
    /// This is kept for backward compatibility with existing configurations.
    pub account_type: String,
    /// Default trading type (spot/swap/futures/option).
    ///
    /// This determines which category to use for API calls.
    /// Bybit uses a unified V5 API with category-based filtering:
    /// - `Spot` -> category=spot
    /// - `Swap` + Linear -> category=linear
    /// - `Swap` + Inverse -> category=inverse
    /// - `Option` -> category=option
    #[serde(default)]
    pub default_type: DefaultType,
    /// Default sub-type for contract settlement (linear/inverse).
    ///
    /// - `Linear`: USDT-margined contracts (category=linear)
    /// - `Inverse`: Coin-margined contracts (category=inverse)
    ///
    /// Only applicable when `default_type` is `Swap` or `Futures`.
    /// Ignored for `Spot` and `Option` types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,
    /// Enables testnet environment.
    pub testnet: bool,
    /// Receive window in milliseconds.
    pub recv_window: u64,
}

impl Default for BybitOptions {
    fn default() -> Self {
        Self {
            account_type: "UNIFIED".to_string(),
            default_type: DefaultType::default(), // Defaults to Spot
            default_sub_type: None,
            testnet: false,
            recv_window: 5000,
        }
    }
}

impl Bybit {
    /// Creates a new Bybit instance using the builder pattern.
    ///
    /// This is the recommended way to create a Bybit instance.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::Bybit;
    ///
    /// let bybit = Bybit::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .testnet(true)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> BybitBuilder {
        BybitBuilder::new()
    }

    /// Creates a new Bybit instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = BybitOptions::default();

        Ok(Self {
            base,
            options,
            ws_client: std::sync::OnceLock::new(),
        })
    }

    /// Creates a new Bybit instance with custom options.
    ///
    /// This is used internally by the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `config` - Exchange configuration.
    /// * `options` - Bybit-specific options.
    pub fn new_with_options(config: ExchangeConfig, options: BybitOptions) -> Result<Self> {
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

    /// Returns the Bybit options.
    pub fn options(&self) -> &BybitOptions {
        &self.options
    }

    /// Sets the Bybit options.
    pub fn set_options(&mut self, options: BybitOptions) {
        self.options = options;
    }

    /// Returns the WebSocket client (unified architecture), initializing it if necessary.
    ///
    /// This provides a shared WebSocket client that supports:
    /// - Message broadcasting (single message loop + multiple subscribers)
    /// - Reference counting for same channel subscriptions
    /// - Automatic reconnection with subscription recovery
    /// - Multi-URL support for different market types
    ///
    /// The client is lazily initialized on first access and reused for subsequent calls.
    pub fn ws_client(&self) -> &ws::BybitWsClient {
        self.ws_client
            .get_or_init(|| ws::create_bybit_ws_client(self.is_sandbox()))
    }

    /// Returns the exchange ID.
    pub fn id(&self) -> &'static str {
        "bybit"
    }

    /// Returns the exchange name.
    pub fn name(&self) -> &'static str {
        "Bybit"
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

    /// Returns `true` if sandbox/testnet mode is enabled.
    ///
    /// Sandbox mode is enabled when either:
    /// - `config.sandbox` is set to `true`
    /// - `options.testnet` is set to `true`
    ///
    /// # Returns
    ///
    /// `true` if sandbox mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::Bybit;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// let config = ExchangeConfig {
    ///     sandbox: true,
    ///     ..Default::default()
    /// };
    /// let bybit = Bybit::new(config).unwrap();
    /// assert!(bybit.is_sandbox());
    /// ```
    pub fn is_sandbox(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }

    /// Returns the supported timeframes.
    pub fn timeframes(&self) -> HashMap<String, String> {
        bybit_timeframes()
            .into_iter()
            .filter_map(|timeframe| {
                let ccxt = timeframe.to_string();
                to_bybit_interval(&ccxt).map(|bybit| (ccxt, bybit.to_string()))
            })
            .collect()
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

    /// Returns the Bybit category string based on the current default_type and default_sub_type.
    ///
    /// Bybit V5 API uses category parameter for filtering:
    /// - `Spot` -> "spot"
    /// - `Swap` + Linear -> "linear"
    /// - `Swap` + Inverse -> "inverse"
    /// - `Futures` + Linear -> "linear"
    /// - `Futures` + Inverse -> "inverse"
    /// - `Option` -> "option"
    /// - `Margin` -> "spot" (margin trading uses spot category)
    ///
    /// # Returns
    ///
    /// The category string to use for Bybit API calls.
    pub fn category(&self) -> &'static str {
        match self.options.default_type {
            DefaultType::Spot | DefaultType::Margin => "spot",
            DefaultType::Swap | DefaultType::Futures => {
                if self.is_inverse() {
                    "inverse"
                } else {
                    "linear"
                }
            }
            DefaultType::Option => "option",
        }
    }

    /// Creates a signed request builder for authenticated API calls.
    ///
    /// This method provides a fluent API for constructing authenticated requests
    /// to Bybit's private endpoints. The builder handles:
    /// - Credential validation
    /// - Millisecond timestamp generation
    /// - HMAC-SHA256 signature generation (hex encoded)
    /// - Authentication header injection (X-BAPI-* headers)
    ///
    /// # Arguments
    ///
    /// * `endpoint` - API endpoint path (e.g., "/v5/account/wallet-balance")
    ///
    /// # Returns
    ///
    /// Returns a `BybitSignedRequestBuilder` for method chaining.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::bybit::Bybit;
    /// use ccxt_exchanges::bybit::signed_request::HttpMethod;
    /// use ccxt_core::ExchangeConfig;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let bybit = Bybit::builder()
    ///     .api_key("your-api-key")
    ///     .secret("your-secret")
    ///     .build()?;
    ///
    /// // GET request
    /// let balance = bybit.signed_request("/v5/account/wallet-balance")
    ///     .param("accountType", "UNIFIED")
    ///     .execute()
    ///     .await?;
    ///
    /// // POST request
    /// let order = bybit.signed_request("/v5/order/create")
    ///     .method(HttpMethod::Post)
    ///     .param("category", "spot")
    ///     .param("symbol", "BTCUSDT")
    ///     .param("side", "Buy")
    ///     .param("orderType", "Limit")
    ///     .param("qty", "0.001")
    ///     .param("price", "50000")
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn signed_request(
        &self,
        endpoint: impl Into<String>,
    ) -> signed_request::BybitSignedRequestBuilder<'_> {
        signed_request::BybitSignedRequestBuilder::new(self, endpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bybit_creation() {
        let config = ExchangeConfig {
            id: "bybit".to_string(),
            name: "Bybit".to_string(),
            ..Default::default()
        };

        let bybit = Bybit::new(config);
        assert!(bybit.is_ok());

        let bybit = bybit.unwrap();
        assert_eq!(bybit.id(), "bybit");
        assert_eq!(bybit.name(), "Bybit");
        assert_eq!(bybit.version(), "v5");
        assert!(!bybit.is_verified());
        assert!(bybit.pro());
    }

    #[test]
    fn test_timeframes() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();
        let timeframes = bybit.timeframes();

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
        let bybit = Bybit::new(config).unwrap();
        assert!(bybit.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_with_options_testnet() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            testnet: true,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert!(bybit.is_sandbox());
    }

    #[test]
    fn test_is_sandbox_false_by_default() {
        let config = ExchangeConfig::default();
        let bybit = Bybit::new(config).unwrap();
        assert!(!bybit.is_sandbox());
    }

    #[test]
    fn test_default_options() {
        let options = BybitOptions::default();
        assert_eq!(options.account_type, "UNIFIED");
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
        assert!(!options.testnet);
        assert_eq!(options.recv_window, 5000);
    }

    #[test]
    fn test_bybit_options_with_default_type() {
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
    }

    #[test]
    fn test_bybit_options_serialization() {
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"default_type\":\"swap\""));
        assert!(json.contains("\"default_sub_type\":\"linear\""));
    }

    #[test]
    fn test_bybit_options_deserialization() {
        let json = r#"{
            "account_type": "CONTRACT",
            "default_type": "swap",
            "default_sub_type": "inverse",
            "testnet": true,
            "recv_window": 10000
        }"#;
        let options: BybitOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.account_type, "CONTRACT");
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Inverse));
        assert!(options.testnet);
        assert_eq!(options.recv_window, 10000);
    }

    #[test]
    fn test_bybit_options_deserialization_without_default_type() {
        // Test backward compatibility - default_type should default to Spot
        let json = r#"{
            "account_type": "UNIFIED",
            "testnet": false,
            "recv_window": 5000
        }"#;
        let options: BybitOptions = serde_json::from_str(json).unwrap();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert_eq!(options.default_sub_type, None);
    }

    #[test]
    fn test_bybit_category_spot() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert_eq!(bybit.category(), "spot");
    }

    #[test]
    fn test_bybit_category_linear() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert_eq!(bybit.category(), "linear");
    }

    #[test]
    fn test_bybit_category_inverse() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert_eq!(bybit.category(), "inverse");
    }

    #[test]
    fn test_bybit_category_option() {
        let config = ExchangeConfig::default();
        let options = BybitOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert_eq!(bybit.category(), "option");
    }

    #[test]
    fn test_bybit_is_contract_type() {
        let config = ExchangeConfig::default();

        // Spot is not a contract type
        let options = BybitOptions {
            default_type: DefaultType::Spot,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config.clone(), options).unwrap();
        assert!(!bybit.is_contract_type());

        // Swap is a contract type
        let options = BybitOptions {
            default_type: DefaultType::Swap,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config.clone(), options).unwrap();
        assert!(bybit.is_contract_type());

        // Futures is a contract type
        let options = BybitOptions {
            default_type: DefaultType::Futures,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config.clone(), options).unwrap();
        assert!(bybit.is_contract_type());

        // Option is a contract type
        let options = BybitOptions {
            default_type: DefaultType::Option,
            ..Default::default()
        };
        let bybit = Bybit::new_with_options(config, options).unwrap();
        assert!(bybit.is_contract_type());
    }

    // ============================================================
    // Sandbox Mode Market Type URL Selection Tests
    // ============================================================
}

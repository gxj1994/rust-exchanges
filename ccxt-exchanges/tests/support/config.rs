//! Test configuration management utilities.
//!
//! Provides test environment configuration loading and management, supporting:
//! - Configuration loading from environment variables
//! - Configuration loading from `.env` files
//! - Multi-exchange API credential management
//! - Performance benchmark configuration

use serde::Deserialize;
use std::env;

/// Configuration loading error types.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvError(#[from] env::VarError),

    /// Configuration parsing error
    #[error("Configuration parsing error: {0}")]
    ParseError(String),

    /// File not found error
    #[error("File not found: {0}")]
    FileNotFound(String),
}

/// Main test configuration structure.
#[derive(Debug, Clone, Deserialize)]
pub struct TestConfig {
    /// Whether to skip private tests
    #[serde(default)]
    pub skip_private_tests: bool,

    /// Test timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub test_timeout_ms: u64,

    /// Binance exchange configuration
    #[serde(default)]
    pub binance: ExchangeConfig,

    /// OKX exchange configuration
    #[serde(default)]
    pub okx: ExchangeConfig,

    /// Bybit exchange configuration
    #[serde(default)]
    pub bybit: ExchangeConfig,

    /// Kraken exchange configuration
    #[serde(default)]
    pub kraken: ExchangeConfig,

    /// `KuCoin` exchange configuration
    #[serde(default)]
    pub kucoin: ExchangeConfig,

    /// Hyperliquid exchange configuration
    #[serde(default)]
    pub hyperliquid: ExchangeConfig,

    /// Bitget exchange configuration
    #[serde(default)]
    pub bitget: ExchangeConfig,

    /// Gate exchange configuration
    #[serde(default)]
    pub gate: ExchangeConfig,

    /// Performance benchmark configuration
    #[serde(default)]
    pub benchmark: BenchmarkConfig,
}

fn default_timeout() -> u64 {
    30000
}

/// Exchange-specific configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExchangeConfig {
    /// API key for production environment
    pub api_key: Option<String>,
    /// API secret for production environment
    pub api_secret: Option<String>,
    /// Whether to use testnet
    #[serde(default)]
    pub use_testnet: bool,
}

/// Performance benchmark configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkConfig {
    /// Benchmark sample size
    #[serde(default = "default_sample_size")]
    pub sample_size: usize,

    /// Number of warmup iterations
    #[serde(default = "default_warmup_iterations")]
    pub warmup_iterations: usize,
}

fn default_sample_size() -> usize {
    100
}

fn default_warmup_iterations() -> usize {
    10
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            sample_size: default_sample_size(),
            warmup_iterations: default_warmup_iterations(),
        }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            skip_private_tests: false,
            test_timeout_ms: default_timeout(),
            binance: ExchangeConfig::default(),
            okx: ExchangeConfig::default(),
            bybit: ExchangeConfig::default(),
            kraken: ExchangeConfig::default(),
            kucoin: ExchangeConfig::default(),
            hyperliquid: ExchangeConfig::default(),
            bitget: ExchangeConfig::default(),
            gate: ExchangeConfig::default(),
            benchmark: BenchmarkConfig::default(),
        }
    }
}

impl TestConfig {
    /// Loads configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if exchange configuration loading fails.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = TestConfig::default();

        if let Ok(val) = env::var("SKIP_PRIVATE_TESTS") {
            config.skip_private_tests = val.parse().unwrap_or(false);
        }
        if let Ok(val) = env::var("TEST_TIMEOUT_MS") {
            config.test_timeout_ms = val.parse().unwrap_or(default_timeout());
        }

        config.binance = Self::load_exchange_config("BINANCE");
        config.okx = Self::load_exchange_config("OKX");
        config.bybit = Self::load_exchange_config("BYBIT");
        config.kraken = Self::load_exchange_config("KRAKEN");
        config.kucoin = Self::load_exchange_config("KUCOIN");
        config.hyperliquid = Self::load_exchange_config("HYPERLIQUID");
        config.bitget = Self::load_exchange_config("BITGET");
        config.gate = Self::load_exchange_config("GATE");

        // Apply global USE_TESTNET setting to all exchanges
        if let Ok(val) = env::var("USE_TESTNET") {
            let use_testnet = val.parse().unwrap_or(false);
            config.binance.use_testnet = use_testnet;
            config.okx.use_testnet = use_testnet;
            config.bybit.use_testnet = use_testnet;
            config.kraken.use_testnet = use_testnet;
            config.kucoin.use_testnet = use_testnet;
            config.hyperliquid.use_testnet = use_testnet;
            config.bitget.use_testnet = use_testnet;
            config.gate.use_testnet = use_testnet;
        }

        if let Ok(val) = env::var("BENCHMARK_SAMPLES") {
            config.benchmark.sample_size = val.parse().unwrap_or(default_sample_size());
        }
        if let Ok(val) = env::var("BENCHMARK_WARMUP") {
            config.benchmark.warmup_iterations = val.parse().unwrap_or(default_warmup_iterations());
        }

        Ok(config)
    }

    /// Loads configuration from a specified `.env` file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the `.env` file
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::FileNotFound`] if the file does not exist.
    pub fn from_dotenv(path: &str) -> Result<Self, ConfigError> {
        dotenvy::from_filename(path)
            .map_err(|e| ConfigError::FileNotFound(format!("{path}: {e}")))?;
        Self::from_env()
    }

    /// Loads configuration from the default `.env` file.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if environment variable loading fails.
    pub fn from_default_dotenv() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        Self::from_env()
    }

    /// Loads configuration for a single exchange from environment variables.
    fn load_exchange_config(exchange: &str) -> ExchangeConfig {
        let mut config = ExchangeConfig::default();

        let api_key_var = format!("{exchange}_API_KEY");
        let api_secret_var = format!("{exchange}_API_SECRET");

        config.api_key = env::var(&api_key_var).ok();
        config.api_secret = env::var(&api_secret_var).ok();

        config
    }

    /// Checks whether private tests should be skipped.
    #[must_use]
    pub fn should_skip_private_tests(&self) -> bool {
        self.skip_private_tests
    }

    /// Checks whether Binance credentials are available.
    #[must_use]
    pub fn has_binance_credentials(&self) -> bool {
        self.binance.has_credentials()
    }

    /// Checks whether OKX credentials are available.
    #[must_use]
    pub fn has_okx_credentials(&self) -> bool {
        self.okx.has_credentials()
    }

    /// Checks whether Bybit credentials are available.
    #[must_use]
    pub fn has_bybit_credentials(&self) -> bool {
        self.bybit.has_credentials()
    }

    /// Checks whether Kraken credentials are available.
    #[must_use]
    pub fn has_kraken_credentials(&self) -> bool {
        self.kraken.has_credentials()
    }

    /// Checks whether `KuCoin` credentials are available.
    #[must_use]
    pub fn has_kucoin_credentials(&self) -> bool {
        self.kucoin.has_credentials()
    }

    /// Checks whether Hyperliquid credentials are available.
    #[must_use]
    pub fn has_hyperliquid_credentials(&self) -> bool {
        self.hyperliquid.has_credentials()
    }

    /// Checks whether Bitget credentials are available.
    #[must_use]
    pub fn has_bitget_credentials(&self) -> bool {
        self.bitget.has_credentials()
    }

    /// Checks if Gate credentials are configured.
    pub fn has_gate_credentials(&self) -> bool {
        self.gate.has_credentials()
    }

    /// Gets the active API credentials for the specified exchange.
    ///
    /// # Arguments
    ///
    /// * `exchange` - Exchange name (case-insensitive)
    ///
    /// # Returns
    ///
    /// Returns `Some((api_key, api_secret))` if credentials exist, `None` otherwise.
    #[must_use]
    pub fn get_active_api_key(&self, exchange: &str) -> Option<(String, String)> {
        let config = match exchange.to_lowercase().as_str() {
            "binance" => &self.binance,
            "okx" => &self.okx,
            "bybit" => &self.bybit,
            "kraken" => &self.kraken,
            "kucoin" => &self.kucoin,
            "hyperliquid" => &self.hyperliquid,
            "bitget" => &self.bitget,
            "gate" => &self.gate,
            _ => return None,
        };

        config.get_active_credentials()
    }
}

impl ExchangeConfig {
    /// Checks whether any credentials are available.
    #[must_use]
    pub fn has_credentials(&self) -> bool {
        self.api_key.is_some() && self.api_secret.is_some()
    }

    /// Gets the active credentials.
    ///
    /// # Returns
    ///
    /// Returns `Some((api_key, api_secret))` if credentials exist, `None` otherwise.
    #[must_use]
    pub fn get_active_credentials(&self) -> Option<(String, String)> {
        match (&self.api_key, &self.api_secret) {
            (Some(key), Some(secret)) => Some((key.clone(), secret.clone())),
            _ => None,
        }
    }
}

/// Conditionally skips a test based on a condition.
///
/// # Examples
///
/// ```ignore
/// // Version 1: With explicit config and condition
/// skip_if!(config, config.skip_private_tests, "Private tests disabled");
///
/// // Version 2: Simplified version for private tests
/// skip_if!(private_tests);
/// ```
#[macro_export]
macro_rules! skip_if {
    ($config:expr, $condition:expr, $reason:expr) => {
        if $condition {
            println!("SKIPPED: {}", $reason);
            return;
        }
    };

    (private_tests) => {{
        let config =
            $crate::support::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
        if config.should_skip_private_tests() {
            println!("SKIPPED: Private tests are disabled");
            return;
        }
    }};
}

/// Requires exchange credentials to run a test.
///
/// Skips the test if credentials are not available for the specified exchange.
///
/// # Examples
///
/// ```ignore
/// // Version 1: With explicit config
/// require_credentials!(config, binance);
///
/// // Version 2: Simplified version with auto-loading
/// require_credentials!(binance);
/// ```
#[macro_export]
macro_rules! require_credentials {
    ($config:expr, binance) => {
        if !$config.has_binance_credentials() {
            println!("SKIPPED: No binance credentials");
            return;
        }
    };
    ($config:expr, okx) => {
        if !$config.has_okx_credentials() {
            println!("SKIPPED: No okx credentials");
            return;
        }
    };
    ($config:expr, bybit) => {
        if !$config.has_bybit_credentials() {
            println!("SKIPPED: No bybit credentials");
            return;
        }
    };
    ($config:expr, kraken) => {
        if !$config.has_kraken_credentials() {
            println!("SKIPPED: No kraken credentials");
            return;
        }
    };
    ($config:expr, kucoin) => {
        if !$config.has_kucoin_credentials() {
            println!("SKIPPED: No kucoin credentials");
            return;
        }
    };
    ($config:expr, hyperliquid) => {
        if !$config.has_hyperliquid_credentials() {
            println!("SKIPPED: No hyperliquid credentials");
            return;
        }
    };

    (binance) => {{
        let config =
            $crate::support::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
        if !config.has_binance_credentials() {
            println!("SKIPPED: No binance credentials");
            return;
        }
    }};
    (okx) => {{
        let config =
            $crate::support::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
        if !config.has_okx_credentials() {
            println!("SKIPPED: No okx credentials");
            return;
        }
    }};
    (bybit) => {{
        let config =
            $crate::support::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
        if !config.has_bybit_credentials() {
            println!("SKIPPED: No bybit credentials");
            return;
        }
    }};
    (kraken) => {{
        let config =
            $crate::support::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
        if !config.has_kraken_credentials() {
            println!("SKIPPED: No kraken credentials");
            return;
        }
    }};
    (kucoin) => {{
        let config =
            $crate::support::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
        if !config.has_kucoin_credentials() {
            println!("SKIPPED: No kucoin credentials");
            return;
        }
    }};
    (hyperliquid) => {{
        let config =
            $crate::support::test_config::TestConfig::from_default_dotenv().unwrap_or_default();
        if !config.has_hyperliquid_credentials() {
            println!("SKIPPED: No hyperliquid credentials");
            return;
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TestConfig::default();
        assert!(!config.skip_private_tests);
        assert_eq!(config.test_timeout_ms, 30000);
        assert_eq!(config.benchmark.sample_size, 100);
    }

    #[test]
    fn test_exchange_config_no_credentials() {
        let config = ExchangeConfig::default();
        assert!(!config.has_credentials());
        assert!(config.get_active_credentials().is_none());
    }

    #[test]
    fn test_exchange_config_with_credentials() {
        let config = ExchangeConfig {
            api_key: Some("test_key".to_string()),
            api_secret: Some("test_secret".to_string()),
            use_testnet: false,
        };

        assert!(config.has_credentials());
        let (key, secret) = config.get_active_credentials().unwrap();
        assert_eq!(key, "test_key");
        assert_eq!(secret, "test_secret");
    }

    #[test]
    fn test_exchange_config_use_testnet_flag() {
        let config = ExchangeConfig {
            api_key: Some("test_key".to_string()),
            api_secret: Some("test_secret".to_string()),
            use_testnet: true,
        };

        assert!(config.has_credentials());
        let (key, secret) = config.get_active_credentials().unwrap();
        assert_eq!(key, "test_key");
        assert_eq!(secret, "test_secret");
    }

    #[test]
    fn test_from_env_with_defaults() {
        // test using defaults when no env vars are set
        let config = TestConfig::from_env().unwrap();
        assert_eq!(config.test_timeout_ms, 30000);
    }
}

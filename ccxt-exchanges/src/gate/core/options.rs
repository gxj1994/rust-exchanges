//! Gate.io exchange options configuration
//!
//! This module provides configuration options for the Gate.io exchange,
//! including default market types, settlement currencies, and testnet mode.

use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};
use serde::{Deserialize, Serialize};

/// Gate.io exchange configuration options
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::gate::core::options::GateOptions;
/// use ccxt_core::types::common::default_type::{DefaultType, DefaultSubType};
///
/// // Default configuration (spot trading)
/// let options = GateOptions::default();
///
/// // Configure for USDT-margined perpetual swaps
/// let options = GateOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Linear),
///     testnet: false,
///     sandbox: false,
/// };
///
/// // Configure for USDC-margined perpetual swaps
/// let options = GateOptions {
///     default_type: DefaultType::Swap,
///     default_sub_type: Some(DefaultSubType::Usdc),
///     testnet: false,
///     sandbox: false,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateOptions {
    /// Default trading type (spot/swap/futures)
    ///
    /// Controls which market type is used when no explicit type is specified.
    ///
    /// # Default
    /// `DefaultType::Spot`
    #[serde(default)]
    pub default_type: DefaultType,

    /// Default sub-type for contract settlement currency
    ///
    /// - `Linear`: USDT-margined contracts (e.g., BTC_USDT)
    /// - `Usdc`: USDC-margined contracts (e.g., BTC_USDC)
    /// - `Inverse`: BTC-margined contracts (e.g., BTC_USD)
    ///
    /// # Default
    /// `None` (uses exchange default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,

    /// Enables testnet mode for contract trading
    ///
    /// When `true`, all requests will be sent to the testnet environment.
    /// Note: Only contract trading has testnet support.
    ///
    /// # Default
    /// `false`
    #[serde(default)]
    pub testnet: bool,

    /// Enables sandbox mode (alias for testnet)
    ///
    /// # Default
    /// `false`
    #[serde(default)]
    pub sandbox: bool,
}

impl Default for GateOptions {
    fn default() -> Self {
        Self {
            default_type: DefaultType::Spot,
            default_sub_type: None,
            testnet: false,
            sandbox: false,
        }
    }
}

impl GateOptions {
    /// Create a new GateOptions with spot trading (default)
    pub fn spot() -> Self {
        Self {
            default_type: DefaultType::Spot,
            ..Default::default()
        }
    }

    /// Create a new GateOptions with USDT-margined perpetual swaps
    pub fn swap_usdt() -> Self {
        Self {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Linear),
            ..Default::default()
        }
    }

    /// Create a new GateOptions with USDC-margined perpetual swaps
    pub fn swap_usdc() -> Self {
        Self {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Usdc),
            ..Default::default()
        }
    }

    /// Create a new GateOptions with BTC-margined perpetual swaps (inverse)
    pub fn swap_inverse() -> Self {
        Self {
            default_type: DefaultType::Swap,
            default_sub_type: Some(DefaultSubType::Inverse),
            ..Default::default()
        }
    }

    /// Check if testnet mode is enabled
    pub fn is_testnet(&self) -> bool {
        self.testnet || self.sandbox
    }

    /// Get the default settle currency based on sub-type configuration
    pub fn default_settle(&self) -> &'static str {
        match self.default_sub_type {
            Some(DefaultSubType::Linear) => "usdt",
            Some(DefaultSubType::Usdc) => "usdc",
            Some(DefaultSubType::Inverse) => "btc",
            None => "usdt", // Default to USDT
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = GateOptions::default();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert!(options.default_sub_type.is_none());
        assert!(!options.is_testnet());
        assert_eq!(options.default_settle(), "usdt");
    }

    #[test]
    fn test_spot_options() {
        let options = GateOptions::spot();
        assert_eq!(options.default_type, DefaultType::Spot);
        assert!(!options.is_testnet());
    }

    #[test]
    fn test_swap_usdt_options() {
        let options = GateOptions::swap_usdt();
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Linear));
        assert_eq!(options.default_settle(), "usdt");
    }

    #[test]
    fn test_swap_usdc_options() {
        let options = GateOptions::swap_usdc();
        assert_eq!(options.default_type, DefaultType::Swap);
        assert_eq!(options.default_sub_type, Some(DefaultSubType::Usdc));
        assert_eq!(options.default_settle(), "usdc");
    }

    #[test]
    fn test_testnet_mode() {
        let options = GateOptions {
            testnet: true,
            ..Default::default()
        };
        assert!(options.is_testnet());

        let options = GateOptions {
            sandbox: true,
            ..Default::default()
        };
        assert!(options.is_testnet());
    }
}

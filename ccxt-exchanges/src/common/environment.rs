//! Environment URL resolver for exchanges.
//!
//! This module provides a reusable resolver for environment-specific URL resolution:
//! - Production URLs
//! - Sandbox/testnet/demo URLs
//! - Optional URL overrides
//!
//! Purpose:
//! - Stop repeating environment-selection logic across exchanges
//! - Reduce URL drift bugs
//! - Provide a single source of truth for URL management
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::common::EnvironmentUrlResolver;
//!
//! // Create a resolver with production and testnet URLs
//! let resolver = EnvironmentUrlResolver::new(
//!     "https://api.binance.com",
//!     "https://testnet.binance.vision",
//! );
//!
//! // Get URL based on environment
//! let prod_url = resolver.resolve(false);  // production URL
//! let test_url = resolver.resolve(true);   // testnet URL
//!
//! assert_eq!(prod_url, "https://api.binance.com");
//! assert_eq!(test_url, "https://testnet.binance.vision");
//! ```

/// Environment URL resolver.
///
/// Manages environment-specific URLs for an exchange, supporting:
/// - Production environment
/// - Testnet/sandbox environment
/// - Custom URL overrides
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::common::EnvironmentUrlResolver;
///
/// let resolver = EnvironmentUrlResolver::new(
///     "https://api.example.com",
///     "https://testnet-api.example.com",
/// );
///
/// // Production URL
/// assert_eq!(resolver.production_url(), "https://api.example.com");
///
/// // Testnet URL
/// assert_eq!(resolver.testnet_url(), "https://testnet-api.example.com");
///
/// // Resolve based on environment
/// assert_eq!(resolver.resolve(false), "https://api.example.com");
/// assert_eq!(resolver.resolve(true), "https://testnet-api.example.com");
/// ```
#[derive(Debug, Clone)]
pub struct EnvironmentUrlResolver {
    /// Production API URL.
    production_url: String,
    /// Testnet/sandbox API URL.
    testnet_url: String,
    /// Optional WebSocket URL for production.
    production_ws_url: Option<String>,
    /// Optional WebSocket URL for testnet.
    testnet_ws_url: Option<String>,
    /// Optional custom URL override (takes precedence).
    custom_override: Option<String>,
    /// Optional custom WebSocket URL override.
    custom_ws_override: Option<String>,
}

impl EnvironmentUrlResolver {
    /// Create a new URL resolver with production and testnet URLs.
    ///
    /// # Arguments
    ///
    /// * `production_url` - The production API URL
    /// * `testnet_url` - The testnet/sandbox API URL
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::common::EnvironmentUrlResolver;
    ///
    /// let resolver = EnvironmentUrlResolver::new(
    ///     "https://api.binance.com",
    ///     "https://testnet.binance.vision",
    /// );
    /// ```
    pub fn new(production_url: &str, testnet_url: &str) -> Self {
        Self {
            production_url: production_url.to_string(),
            testnet_url: testnet_url.to_string(),
            production_ws_url: None,
            testnet_ws_url: None,
            custom_override: None,
            custom_ws_override: None,
        }
    }

    /// Create a new URL resolver with WebSocket support.
    ///
    /// # Arguments
    ///
    /// * `production_url` - The production REST API URL
    /// * `testnet_url` - The testnet REST API URL
    /// * `production_ws_url` - The production WebSocket URL
    /// * `testnet_ws_url` - The testnet WebSocket URL
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::common::EnvironmentUrlResolver;
    ///
    /// let resolver = EnvironmentUrlResolver::with_websocket(
    ///     "https://api.binance.com",
    ///     "https://testnet.binance.vision",
    ///     "wss://stream.binance.com:9443/ws",
    ///     "wss://testnet.binance.vision/ws",
    /// );
    /// ```
    pub fn with_websocket(
        production_url: &str,
        testnet_url: &str,
        production_ws_url: &str,
        testnet_ws_url: &str,
    ) -> Self {
        Self {
            production_url: production_url.to_string(),
            testnet_url: testnet_url.to_string(),
            production_ws_url: Some(production_ws_url.to_string()),
            testnet_ws_url: Some(testnet_ws_url.to_string()),
            custom_override: None,
            custom_ws_override: None,
        }
    }

    /// Get the production REST API URL.
    pub fn production_url(&self) -> &str {
        &self.production_url
    }

    /// Get the testnet REST API URL.
    pub fn testnet_url(&self) -> &str {
        &self.testnet_url
    }

    /// Get the production WebSocket URL (if configured).
    pub fn production_ws_url(&self) -> Option<&str> {
        self.production_ws_url.as_deref()
    }

    /// Get the testnet WebSocket URL (if configured).
    pub fn testnet_ws_url(&self) -> Option<&str> {
        self.testnet_ws_url.as_deref()
    }

    /// Set a custom URL override (takes precedence over environment selection).
    pub fn with_custom_override(mut self, url: &str) -> Self {
        self.custom_override = Some(url.to_string());
        self
    }

    /// Set a custom WebSocket URL override.
    pub fn with_custom_ws_override(mut self, url: &str) -> Self {
        self.custom_ws_override = Some(url.to_string());
        self
    }

    /// Resolve the REST API URL based on environment.
    ///
    /// # Arguments
    ///
    /// * `testnet` - If true, returns testnet URL; otherwise returns production URL
    ///
    /// # Returns
    ///
    /// The resolved URL string. Custom override takes precedence if set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::common::EnvironmentUrlResolver;
    ///
    /// let resolver = EnvironmentUrlResolver::new(
    ///     "https://api.example.com",
    ///     "https://testnet.example.com",
    /// );
    ///
    /// assert_eq!(resolver.resolve(false), "https://api.example.com");
    /// assert_eq!(resolver.resolve(true), "https://testnet.example.com");
    /// ```
    pub fn resolve(&self, testnet: bool) -> &str {
        // Custom override takes precedence
        if let Some(ref custom) = self.custom_override {
            return custom;
        }

        if testnet {
            &self.testnet_url
        } else {
            &self.production_url
        }
    }

    /// Resolve the WebSocket URL based on environment.
    ///
    /// # Arguments
    ///
    /// * `testnet` - If true, returns testnet WebSocket URL; otherwise returns production
    ///
    /// # Returns
    ///
    /// The resolved WebSocket URL, or None if not configured.
    /// Custom override takes precedence if set.
    pub fn resolve_ws(&self, testnet: bool) -> Option<&str> {
        // Custom override takes precedence
        if let Some(ref custom) = self.custom_ws_override {
            return Some(custom);
        }

        if testnet {
            self.testnet_ws_url.as_deref()
        } else {
            self.production_ws_url.as_deref()
        }
    }

    /// Check if WebSocket URLs are configured.
    pub fn has_websocket(&self) -> bool {
        self.production_ws_url.is_some() || self.testnet_ws_url.is_some()
    }
}

// ============================================================================
// Predefined Resolvers for Common Exchanges
// ============================================================================

/// Get the URL resolver for Binance.
pub fn binance_resolver() -> EnvironmentUrlResolver {
    EnvironmentUrlResolver::with_websocket(
        "https://api.binance.com",
        "https://testnet.binance.vision",
        "wss://stream.binance.com:9443/ws",
        "wss://testnet.binance.vision/ws",
    )
}

/// Get the URL resolver for OKX.
pub fn okx_resolver() -> EnvironmentUrlResolver {
    EnvironmentUrlResolver::with_websocket(
        "https://www.okx.com",
        "https://www.okx.com", // OKX uses same URL, different headers
        "wss://ws.okx.com:8443/ws/v5/public",
        "wss://ws.okx.com:8443/ws/v5/public",
    )
}

/// Get the URL resolver for Bybit.
pub fn bybit_resolver() -> EnvironmentUrlResolver {
    EnvironmentUrlResolver::with_websocket(
        "https://api.bybit.com",
        "https://api-testnet.bybit.com",
        "wss://stream.bybit.com/v5/public",
        "wss://stream-testnet.bybit.com/v5/public",
    )
}

/// Get the URL resolver for Bitget.
pub fn bitget_resolver() -> EnvironmentUrlResolver {
    EnvironmentUrlResolver::with_websocket(
        "https://api.bitget.com",
        "https://api.bitget.com", // Bitget uses same URL with different headers
        "wss://ws.bitget.com/v2/ws/public",
        "wss://ws.bitget.com/v2/ws/public",
    )
}

/// Get the URL resolver for HyperLiquid.
pub fn hyperliquid_resolver() -> EnvironmentUrlResolver {
    EnvironmentUrlResolver::with_websocket(
        "https://api.hyperliquid.xyz",
        "https://api.hyperliquid-testnet.xyz",
        "wss://api.hyperliquid.xyz/ws",
        "wss://api.hyperliquid-testnet.xyz/ws",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_resolver() {
        let resolver =
            EnvironmentUrlResolver::new("https://api.example.com", "https://testnet.example.com");

        assert_eq!(resolver.production_url(), "https://api.example.com");
        assert_eq!(resolver.testnet_url(), "https://testnet.example.com");
        assert_eq!(resolver.resolve(false), "https://api.example.com");
        assert_eq!(resolver.resolve(true), "https://testnet.example.com");
        assert!(!resolver.has_websocket());
    }

    #[test]
    fn test_websocket_resolver() {
        let resolver = EnvironmentUrlResolver::with_websocket(
            "https://api.example.com",
            "https://testnet.example.com",
            "wss://ws.example.com",
            "wss://ws-testnet.example.com",
        );

        assert_eq!(resolver.production_ws_url(), Some("wss://ws.example.com"));
        assert_eq!(
            resolver.testnet_ws_url(),
            Some("wss://ws-testnet.example.com")
        );
        assert!(resolver.has_websocket());
        assert_eq!(resolver.resolve_ws(false), Some("wss://ws.example.com"));
        assert_eq!(
            resolver.resolve_ws(true),
            Some("wss://ws-testnet.example.com")
        );
    }

    #[test]
    fn test_custom_override() {
        let resolver =
            EnvironmentUrlResolver::new("https://api.example.com", "https://testnet.example.com")
                .with_custom_override("https://custom.example.com");

        // Override takes precedence
        assert_eq!(resolver.resolve(false), "https://custom.example.com");
        assert_eq!(resolver.resolve(true), "https://custom.example.com");
    }

    #[test]
    fn test_custom_ws_override() {
        let resolver = EnvironmentUrlResolver::with_websocket(
            "https://api.example.com",
            "https://testnet.example.com",
            "wss://ws.example.com",
            "wss://ws-testnet.example.com",
        )
        .with_custom_ws_override("wss://custom.example.com");

        // Override takes precedence
        assert_eq!(resolver.resolve_ws(false), Some("wss://custom.example.com"));
        assert_eq!(resolver.resolve_ws(true), Some("wss://custom.example.com"));
    }

    #[test]
    fn test_preset_resolvers() {
        // Test that preset resolvers don't panic and have reasonable values
        let binance = binance_resolver();
        assert!(binance.has_websocket());
        assert_eq!(binance.resolve(false), "https://api.binance.com");

        let okx = okx_resolver();
        assert!(okx.has_websocket());

        let bybit = bybit_resolver();
        assert!(bybit.has_websocket());
        assert_eq!(bybit.resolve(true), "https://api-testnet.bybit.com");

        let bitget = bitget_resolver();
        assert!(bitget.has_websocket());

        let hyperliquid = hyperliquid_resolver();
        assert!(hyperliquid.has_websocket());
        assert_eq!(
            hyperliquid.resolve(true),
            "https://api.hyperliquid-testnet.xyz"
        );
    }
}

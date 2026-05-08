//! Gate.io rate limiter implementation.
//!
//! This module provides rate limiting for Gate.io API requests
//! using the token bucket algorithm from `ccxt_core`.
//!
//! # Rate Limits
//!
//! Gate.io API rate limits:
//! - Public endpoints: ~200 requests/second per IP
//! - Private endpoints: ~200 requests/10 seconds per API key
//!
//! We configure a conservative default of 10 requests/second.
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::gate::network::rate_limiter::GateRateLimiter;
//! use std::time::Duration;
//!
//! # async fn example() {
//! let limiter = GateRateLimiter::new();
//! limiter.wait().await;
//! // Make API request...
//! # }
//! ```

use ccxt_core::rate_limiter::{RateLimiter, RateLimiterConfig};
use std::sync::Arc;
use std::time::Duration;

/// Default requests per second for Gate.io public API.
pub const DEFAULT_RATE_LIMIT_RPS: u32 = 10;

/// Gate.io rate limiter wrapper.
///
/// Uses ccxt-core's token bucket RateLimiter.
pub struct GateRateLimiter {
    inner: Arc<RateLimiter>,
}

impl GateRateLimiter {
    /// Create a new Gate rate limiter with default settings (10 RPS).
    pub fn new() -> Self {
        Self::with_config(DEFAULT_RATE_LIMIT_RPS)
    }

    /// Create a rate limiter with custom requests per second.
    pub fn with_config(rps: u32) -> Self {
        let config = RateLimiterConfig::new(rps, Duration::from_secs(1));
        Self {
            inner: Arc::new(RateLimiter::new(config)),
        }
    }

    /// Wait for permission to make a request.
    ///
    /// This is the main entry point for rate limiting.
    /// Call this before making an API request.
    pub async fn wait(&self) {
        self.inner.wait().await;
    }

    /// Get a cloned Arc reference to the inner rate limiter.
    pub fn inner(&self) -> Arc<RateLimiter> {
        Arc::clone(&self.inner)
    }
}

impl Default for GateRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GateRateLimiter {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_rate_limiter_creation() {
        let limiter = GateRateLimiter::new();
        assert!(Arc::strong_count(&limiter.inner) == 1);
    }

    #[test]
    fn test_gate_rate_limiter_clone() {
        let limiter = GateRateLimiter::new();
        assert_eq!(Arc::strong_count(&limiter.inner), 1); // 初始应该是 1

        let cloned = limiter.clone();
        assert_eq!(Arc::strong_count(&limiter.inner), 2); // clone 后应该是 2

        drop(cloned);
        assert_eq!(Arc::strong_count(&limiter.inner), 1); // drop 后应该回到 1
    }

    #[tokio::test]
    async fn test_gate_rate_limiter_wait() {
        let limiter = GateRateLimiter::new();
        // Should not block with default capacity
        limiter.wait().await;
    }

    #[test]
    fn test_gate_rate_limiter_custom_config() {
        let limiter = GateRateLimiter::with_config(5);
        assert!(Arc::strong_count(&limiter.inner) == 1);
    }
}

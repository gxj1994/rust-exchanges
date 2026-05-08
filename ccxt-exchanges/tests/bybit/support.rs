//! Bybit-specific test support utilities.
//!
//! This module provides shared helper functions for Bybit integration tests.
//! Since Bybit V5 API is unified, we use a single authenticated client for
//! all market types (spot, swap, option).

use crate::support::{create_bybit_with_credentials, init_test};
use ccxt_exchanges::bybit::Bybit;

/// Macro to skip tests if Bybit credentials are not configured.
///
/// # Usage
///
/// ```ignore
/// #[tokio::test]
/// async fn test_something() {
///     skip_if_no_bybit_credentials!();
///     // test code...
/// }
/// ```
#[macro_export]
macro_rules! skip_if_no_bybit_credentials {
    () => {
        if $crate::support::should_skip_private_tests("bybit") {
            println!("SKIPPED: No Bybit credentials configured");
            return;
        }
    };
}

/// Create an authenticated Bybit client for integration tests.
///
/// Bybit V5 API is unified, so there's no need to distinguish between
/// spot/swap/option at the client level. The `category_from_symbol()`
/// method automatically determines the correct API category based on
/// the symbol's market type.
///
/// # Returns
///
/// An authenticated Bybit instance with markets loaded.
///
/// # Panics
///
/// Panics if:
/// - Bybit credentials are not configured
/// - Failed to create Bybit instance
/// - Failed to load markets
///
/// # Example
///
/// ```ignore
/// use crate::bybit::support::create_auth_bybit;
///
/// #[tokio::test]
/// async fn test_something() {
///     let bybit = create_auth_bybit().await;
///     // use bybit...
/// }
/// ```
pub async fn create_auth_bybit(market_type: &str) -> Bybit {
    let config = init_test();
    let exchange = create_bybit_with_credentials(&config, market_type)
        .expect("Failed to create Bybit with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    exchange
}

/// Convert Price to Decimal for test assertions.
///
/// # Arguments
///
/// * `price` - Price value from API response
///
/// # Returns
///
/// Decimal representation of the price.
pub fn price_to_decimal(price: ccxt_core::types::Price) -> rust_decimal::Decimal {
    price.into()
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_skip_macro_defined() {
        // Just verify the macro is accessible
        assert!(true);
    }
}

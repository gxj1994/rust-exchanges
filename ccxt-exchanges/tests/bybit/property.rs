#![allow(clippy::disallowed_methods)]
//! Bybit property-based tests using proptest.
//!
//! These tests verify correctness properties for Bybit exchange implementation.

use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::json;

/// For any set of configuration values (api_key, secret), when passed to the BybitBuilder
/// methods and then built, the resulting Bybit instance SHALL contain those exact
/// configuration values accessible through the base exchange config.
mod bybit_builder_configuration_preservation {
    use super::*;
    use ccxt_exchanges::bybit::BybitBuilder;
    use std::time::Duration;

    // Strategy for generating valid API keys (alphanumeric, 16-32 chars)
    fn api_key_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{16,32}"
    }

    // Strategy for generating valid secrets (alphanumeric, 32-64 chars)
    fn secret_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{32,64}"
    }

    // Strategy for generating valid account types
    fn account_type_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("UNIFIED".to_string()),
            Just("CONTRACT".to_string()),
            Just("SPOT".to_string()),
        ]
    }

    // Strategy for generating timeout values
    fn timeout_strategy() -> impl Strategy<Value = u64> {
        1u64..120u64
    }

    // Strategy for generating recv_window values
    fn recv_window_strategy() -> impl Strategy<Value = u64> {
        1000u64..30000u64
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that full configuration is preserved through build()
        /// This tests api_key, secret preservation (Requirements 2.2)
        #[test]
        fn prop_bybit_builder_build_preserves_credentials(
            api_key in api_key_strategy(),
            secret in secret_strategy(),
        ) {
            let bybit = BybitBuilder::new()
                .api_key(api_key.clone())
                .secret(secret.clone())
                .build();

            prop_assert!(bybit.is_ok(), "Bybit builder should build successfully");

            let bybit = bybit.unwrap();

            // Verify exchange identity is correct
            prop_assert_eq!(bybit.id(), "bybit", "Bybit id should be 'bybit'");
            prop_assert_eq!(bybit.name(), "Bybit", "Bybit name should be 'Bybit'");

            // Verify credentials are preserved in base config
            let base = bybit.base();
            prop_assert_eq!(
                base.config.api_key.as_ref().map(|s| s.expose_secret()),
                Some(api_key.as_str()),
                "Bybit builder should preserve api_key after build"
            );
            prop_assert_eq!(
                base.config.secret.as_ref().map(|s| s.expose_secret()),
                Some(secret.as_str()),
                "Bybit builder should preserve secret after build"
            );
        }

        /// Test that testnet mode is preserved through build() (Requirements 2.4)
        #[test]
        fn prop_bybit_builder_build_preserves_testnet(testnet in any::<bool>()) {
            let bybit = BybitBuilder::new()
                .testnet(testnet)
                .build();

            prop_assert!(bybit.is_ok(), "Bybit builder should build successfully");

            let bybit = bybit.unwrap();

            // Verify testnet mode is preserved in base config
            let base = bybit.base();
            prop_assert_eq!(
                base.config.sandbox, testnet,
                "Bybit builder should preserve testnet mode as sandbox after build"
            );

            // Verify testnet mode is preserved in options
            let options = bybit.options();
            prop_assert_eq!(
                options.testnet, testnet,
                "Bybit builder should preserve testnet mode in options after build"
            );
        }

        /// Test that account_type is preserved through build()
        #[test]
        fn prop_bybit_builder_build_preserves_account_type(account_type in account_type_strategy()) {
            let bybit = BybitBuilder::new()
                .account_type(account_type.clone())
                .build();

            prop_assert!(bybit.is_ok(), "Bybit builder should build successfully");

            let bybit = bybit.unwrap();

            // Verify account_type is preserved in options
            let options = bybit.options();
            prop_assert_eq!(
                &options.account_type, &account_type,
                "Bybit builder should preserve account_type after build"
            );
        }

        /// Test that recv_window is preserved through build()
        #[test]
        fn prop_bybit_builder_build_preserves_recv_window(recv_window in recv_window_strategy()) {
            let bybit = BybitBuilder::new()
                .recv_window(recv_window)
                .build();

            prop_assert!(bybit.is_ok(), "Bybit builder should build successfully");

            let bybit = bybit.unwrap();

            // Verify recv_window is preserved in options
            let options = bybit.options();
            prop_assert_eq!(
                options.recv_window, recv_window,
                "Bybit builder should preserve recv_window after build"
            );
        }

        /// Test that timeout is preserved through build()
        #[test]
        fn prop_bybit_builder_build_preserves_timeout(timeout in timeout_strategy()) {
            let bybit = BybitBuilder::new()
                .timeout(Duration::from_secs(timeout))
                .build();

            prop_assert!(bybit.is_ok(), "Bybit builder should build successfully");

            let bybit = bybit.unwrap();

            // Verify timeout is preserved in base config
            let base = bybit.base();
            prop_assert_eq!(
                base.config.timeout, Duration::from_secs(timeout),
                "Bybit builder should preserve timeout after build"
            );
        }

        /// Test that full configuration is preserved through build()
        #[test]
        fn prop_bybit_builder_build_preserves_full_config(
            api_key in api_key_strategy(),
            secret in secret_strategy(),
            testnet in any::<bool>(),
            account_type in account_type_strategy(),
            timeout in timeout_strategy(),
            recv_window in recv_window_strategy(),
        ) {
            let bybit = BybitBuilder::new()
                .api_key(api_key.clone())
                .secret(secret.clone())
                .testnet(testnet)
                .account_type(account_type.clone())
                .timeout(Duration::from_secs(timeout))
                .recv_window(recv_window)
                .build();

            prop_assert!(bybit.is_ok(), "Bybit builder should build successfully");

            let bybit = bybit.unwrap();

            // Verify exchange identity
            prop_assert_eq!(bybit.id(), "bybit", "Bybit id should be 'bybit'");
            prop_assert_eq!(bybit.name(), "Bybit", "Bybit name should be 'Bybit'");

            // Verify credentials are preserved
            let base = bybit.base();
            prop_assert_eq!(
                base.config.api_key.as_ref().map(|s| s.expose_secret()),
                Some(api_key.as_str()),
                "Bybit builder should preserve api_key"
            );
            prop_assert_eq!(
                base.config.secret.as_ref().map(|s| s.expose_secret()),
                Some(secret.as_str()),
                "Bybit builder should preserve secret"
            );
            prop_assert_eq!(
                base.config.sandbox, testnet,
                "Bybit builder should preserve testnet as sandbox"
            );
            prop_assert_eq!(
                base.config.timeout, Duration::from_secs(timeout),
                "Bybit builder should preserve timeout"
            );

            // Verify options are preserved
            let options = bybit.options();
            prop_assert_eq!(
                &options.account_type, &account_type,
                "Bybit options should preserve account_type after build"
            );
            prop_assert_eq!(
                options.testnet, testnet,
                "Bybit options should preserve testnet mode after build"
            );
            prop_assert_eq!(
                options.recv_window, recv_window,
                "Bybit options should preserve recv_window after build"
            );
        }

        /// Test that builder order doesn't matter for final configuration
        #[test]
        fn prop_bybit_builder_order_independent(
            api_key in api_key_strategy(),
            secret in secret_strategy(),
        ) {
            // Build with one order
            let bybit1 = BybitBuilder::new()
                .api_key(api_key.clone())
                .secret(secret.clone())
                .build()
                .expect("Should build bybit1");

            // Build with different order
            let bybit2 = BybitBuilder::new()
                .secret(secret.clone())
                .api_key(api_key.clone())
                .build()
                .expect("Should build bybit2");

            // All should have the same credentials (compare exposed secrets)
            prop_assert_eq!(
                bybit1.base().config.api_key.as_ref().map(|s| s.expose_secret()),
                bybit2.base().config.api_key.as_ref().map(|s| s.expose_secret())
            );
            prop_assert_eq!(
                bybit1.base().config.secret.as_ref().map(|s| s.expose_secret()),
                bybit2.base().config.secret.as_ref().map(|s| s.expose_secret())
            );
        }

        /// Test that overwriting a value preserves the last value
        #[test]
        fn prop_bybit_builder_last_value_wins(
            api_key1 in api_key_strategy(),
            api_key2 in api_key_strategy(),
        ) {
            let bybit = BybitBuilder::new()
                .api_key(api_key1.clone())
                .api_key(api_key2.clone())
                .build()
                .expect("Should build bybit");

            prop_assert_eq!(
                bybit.base().config.api_key.as_ref().map(|s| s.expose_secret()),
                Some(api_key2.as_str()),
                "Bybit builder should use the last api_key value"
            );
        }

        /// Test that default values are correct when not explicitly set
        #[test]
        fn prop_bybit_builder_default_values(_dummy in Just(())) {
            let bybit = BybitBuilder::new()
                .build()
                .expect("Should build bybit with defaults");

            // Verify default exchange identity
            prop_assert_eq!(bybit.id(), "bybit");
            prop_assert_eq!(bybit.name(), "Bybit");

            // Verify default options
            let options = bybit.options();
            prop_assert_eq!(&options.account_type, "UNIFIED");
            prop_assert!(!options.testnet);
            prop_assert_eq!(options.recv_window, 5000);

            // Verify default config
            let base = bybit.base();
            prop_assert!(!base.config.sandbox);
            prop_assert!(base.config.api_key.is_none());
            prop_assert!(base.config.secret.is_none());
        }
    }
}

/// For any parsed OrderBook from either OKX or Bybit, bids SHALL be sorted in
/// descending order by price, and asks SHALL be sorted in ascending order by price.
mod orderbook_sorting_invariant {
    use super::*;
    use ccxt_core::types::Symbol;
    use ccxt_exchanges::bybit::parser::parse_orderbook as bybit_parse_orderbook;
    use ccxt_exchanges::okx::parser::parse_orderbook as okx_parse_orderbook;

    // Strategy for generating valid price strings (positive decimals)
    fn price_strategy() -> impl Strategy<Value = String> {
        (1u64..100000u64, 0u32..8u32).prop_map(|(whole, decimals)| {
            if decimals == 0 {
                whole.to_string()
            } else {
                let divisor = 10u64.pow(decimals);
                let frac = whole % divisor;
                format!(
                    "{}.{:0>width$}",
                    whole / divisor,
                    frac,
                    width = decimals as usize
                )
            }
        })
    }

    // Strategy for generating valid amount strings (positive decimals)
    fn amount_strategy() -> impl Strategy<Value = String> {
        (1u64..10000u64, 0u32..6u32).prop_map(|(whole, decimals)| {
            if decimals == 0 {
                whole.to_string()
            } else {
                let divisor = 10u64.pow(decimals);
                let frac = whole % divisor;
                format!(
                    "{}.{:0>width$}",
                    whole / divisor,
                    frac,
                    width = decimals as usize
                )
            }
        })
    }

    // Strategy for generating orderbook entries (price, amount pairs)
    fn orderbook_entry_strategy() -> impl Strategy<Value = (String, String)> {
        (price_strategy(), amount_strategy())
    }

    // Strategy for generating a list of orderbook entries
    fn orderbook_side_strategy() -> impl Strategy<Value = Vec<(String, String)>> {
        prop::collection::vec(orderbook_entry_strategy(), 1..20)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that OKX bids are sorted in descending order by price after parsing
        #[test]
        fn prop_okx_bids_sorted_descending(entries in orderbook_side_strategy()) {
            let bids_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount, "0", "1"]))
                .collect();

            let data = json!({
                "bids": bids_json,
                "asks": [],
                "ts": "1700000000000"
            });

            let orderbook = okx_parse_orderbook(&data, Symbol::new_unchecked("BTC/USDT"))
                .expect("Failed to parse OKX orderbook");

            // Verify bids are sorted in descending order
            for i in 1..orderbook.bids.len() {
                prop_assert!(
                    orderbook.bids[i - 1].price >= orderbook.bids[i].price,
                    "OKX bids should be sorted in descending order: {:?} >= {:?}",
                    orderbook.bids[i - 1].price,
                    orderbook.bids[i].price
                );
            }
        }

        /// Test that OKX asks are sorted in ascending order by price after parsing
        #[test]
        fn prop_okx_asks_sorted_ascending(entries in orderbook_side_strategy()) {
            let asks_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount, "0", "1"]))
                .collect();

            let data = json!({
                "bids": [],
                "asks": asks_json,
                "ts": "1700000000000"
            });

            let orderbook = okx_parse_orderbook(&data, Symbol::new_unchecked("BTC/USDT"))
                .expect("Failed to parse OKX orderbook");

            // Verify asks are sorted in ascending order
            for i in 1..orderbook.asks.len() {
                prop_assert!(
                    orderbook.asks[i - 1].price <= orderbook.asks[i].price,
                    "OKX asks should be sorted in ascending order: {:?} <= {:?}",
                    orderbook.asks[i - 1].price,
                    orderbook.asks[i].price
                );
            }
        }

        /// Test that Bybit bids are sorted in descending order by price after parsing
        #[test]
        fn prop_bybit_bids_sorted_descending(entries in orderbook_side_strategy()) {
            let bids_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount]))
                .collect();

            let data = json!({
                "b": bids_json,
                "a": [],
                "ts": "1700000000000"
            });

            let orderbook = bybit_parse_orderbook(&data, "BTC/USDT".to_string())
                .expect("Failed to parse Bybit orderbook");

            // Verify bids are sorted in descending order
            for i in 1..orderbook.bids.len() {
                prop_assert!(
                    orderbook.bids[i - 1].price >= orderbook.bids[i].price,
                    "Bybit bids should be sorted in descending order: {:?} >= {:?}",
                    orderbook.bids[i - 1].price,
                    orderbook.bids[i].price
                );
            }
        }

        /// Test that Bybit asks are sorted in ascending order by price after parsing
        #[test]
        fn prop_bybit_asks_sorted_ascending(entries in orderbook_side_strategy()) {
            let asks_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount]))
                .collect();

            let data = json!({
                "b": [],
                "a": asks_json,
                "ts": "1700000000000"
            });

            let orderbook = bybit_parse_orderbook(&data, "BTC/USDT".to_string())
                .expect("Failed to parse Bybit orderbook");

            // Verify asks are sorted in ascending order
            for i in 1..orderbook.asks.len() {
                prop_assert!(
                    orderbook.asks[i - 1].price <= orderbook.asks[i].price,
                    "Bybit asks should be sorted in ascending order: {:?} <= {:?}",
                    orderbook.asks[i - 1].price,
                    orderbook.asks[i].price
                );
            }
        }

        /// Test that all OKX entries are preserved (no data loss)
        #[test]
        fn prop_okx_all_entries_preserved(entries in orderbook_side_strategy()) {
            let bids_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount, "0", "1"]))
                .collect();

            let data = json!({
                "bids": bids_json,
                "asks": [],
                "ts": "1700000000000"
            });

            let orderbook = okx_parse_orderbook(&data, Symbol::new_unchecked("BTC/USDT"))
                .expect("Failed to parse OKX orderbook");

            prop_assert_eq!(
                orderbook.bids.len(),
                entries.len(),
                "OKX should preserve all entries"
            );
        }

        /// Test that all Bybit entries are preserved (no data loss)
        #[test]
        fn prop_bybit_all_entries_preserved(entries in orderbook_side_strategy()) {
            let bids_json: Vec<serde_json::Value> = entries
                .iter()
                .map(|(price, amount)| json!([price, amount]))
                .collect();

            let data = json!({
                "b": bids_json,
                "a": [],
                "ts": "1700000000000"
            });

            let orderbook = bybit_parse_orderbook(&data, "BTC/USDT".to_string())
                .expect("Failed to parse Bybit orderbook");

            prop_assert_eq!(
                orderbook.bids.len(),
                entries.len(),
                "Bybit should preserve all entries"
            );
        }
    }
}

/// For any valid error response JSON from OKX or Bybit, the parser SHALL extract
/// the error code and message into a structured Error type without panicking.
mod error_response_parsing {
    use super::*;
    use ccxt_exchanges::bybit::core::error::{
        BybitErrorCode, extract_error_code as bybit_extract_code,
        extract_error_message as bybit_extract_message, is_error_response as bybit_is_error,
        parse_error as bybit_parse_error,
    };
    use ccxt_exchanges::okx::core::error::{
        OkxErrorCode, extract_error_code as okx_extract_code,
        extract_error_message as okx_extract_message, is_error_response as okx_is_error,
        parse_error as okx_parse_error,
    };

    // Strategy for generating OKX error codes
    fn okx_error_code_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("50000".to_string()),
            Just("50001".to_string()),
            Just("50002".to_string()),
            Just("50004".to_string()),
            Just("50011".to_string()),
            Just("51000".to_string()),
            Just("51001".to_string()),
            Just("51400".to_string()),
            (10000i64..60000i64).prop_map(|n| n.to_string()),
        ]
    }

    // Strategy for generating Bybit error codes
    fn bybit_error_code_strategy() -> impl Strategy<Value = i64> {
        prop_oneof![
            Just(10001i64),
            Just(10003i64),
            Just(10004i64),
            Just(10006i64),
            Just(10016i64),
            Just(110001i64),
            Just(110008i64),
            (1000i64..200000i64),
        ]
    }

    // Strategy for generating error messages
    fn error_message_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Invalid API key".to_string()),
            Just("Invalid signature".to_string()),
            Just("Rate limit exceeded".to_string()),
            Just("Invalid request parameters".to_string()),
            Just("Insufficient balance".to_string()),
            Just("Invalid trading pair".to_string()),
            Just("Order not found".to_string()),
            "[a-zA-Z0-9 ]{5,50}",
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that OKX parse_error always produces a valid Error for any error response
        #[test]
        fn prop_okx_parse_error_produces_valid_error(
            code in okx_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "code": code,
                "msg": msg
            });

            let error = okx_parse_error(&response);

            // Error should have a non-empty display string
            let display = error.to_string();
            prop_assert!(!display.is_empty(), "OKX error display should not be empty");
        }

        /// Test that Bybit parse_error always produces a valid Error for any error response
        #[test]
        fn prop_bybit_parse_error_produces_valid_error(
            code in bybit_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "retCode": code,
                "retMsg": msg
            });

            let error = bybit_parse_error(&response);

            // Error should have a non-empty display string
            let display = error.to_string();
            prop_assert!(!display.is_empty(), "Bybit error display should not be empty");
        }

        /// Test that OKX error code is correctly extracted
        #[test]
        fn prop_okx_extract_error_code_consistent(
            code in okx_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "code": code,
                "msg": msg
            });

            let extracted = okx_extract_code(&response);
            prop_assert_eq!(extracted, code.as_str(), "OKX extracted code should match input");
        }

        /// Test that Bybit error code is correctly extracted
        #[test]
        fn prop_bybit_extract_error_code_consistent(
            code in bybit_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "retCode": code,
                "retMsg": msg
            });

            let extracted = bybit_extract_code(&response);
            prop_assert_eq!(extracted, code, "Bybit extracted code should match input");
        }

        /// Test that OKX error message is correctly extracted
        #[test]
        fn prop_okx_extract_error_message_consistent(
            code in okx_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "code": code,
                "msg": msg
            });

            let extracted = okx_extract_message(&response);
            prop_assert_eq!(extracted, msg.as_str(), "OKX extracted message should match input");
        }

        /// Test that Bybit error message is correctly extracted
        #[test]
        fn prop_bybit_extract_error_message_consistent(
            code in bybit_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "retCode": code,
                "retMsg": msg
            });

            let extracted = bybit_extract_message(&response);
            prop_assert_eq!(extracted, msg.as_str(), "Bybit extracted message should match input");
        }

        /// Test that OKX is_error_response correctly identifies errors
        #[test]
        fn prop_okx_is_error_response_identifies_errors(
            code in okx_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "code": code,
                "msg": msg
            });

            // All non-"0" codes should be identified as errors
            if code != "0" {
                prop_assert!(okx_is_error(&response), "OKX non-0 code should be identified as error");
            }
        }

        /// Test that Bybit is_error_response correctly identifies errors
        #[test]
        fn prop_bybit_is_error_response_identifies_errors(
            code in bybit_error_code_strategy(),
            msg in error_message_strategy()
        ) {
            let response = json!({
                "retCode": code,
                "retMsg": msg
            });

            // All non-0 codes should be identified as errors
            if code != 0 {
                prop_assert!(bybit_is_error(&response), "Bybit non-0 code should be identified as error");
            }
        }

        /// Test that OKX missing fields are handled gracefully
        #[test]
        fn prop_okx_missing_code_handled(msg in error_message_strategy()) {
            let response = json!({
                "msg": msg
            });

            // Should not panic
            let error = okx_parse_error(&response);
            let display = error.to_string();
            prop_assert!(!display.is_empty(), "OKX error should have display even with missing code");

            let code = okx_extract_code(&response);
            prop_assert_eq!(code, "unknown", "OKX missing code should return 'unknown'");
        }

        /// Test that Bybit missing fields are handled gracefully
        #[test]
        fn prop_bybit_missing_code_handled(msg in error_message_strategy()) {
            let response = json!({
                "retMsg": msg
            });

            // Should not panic
            let error = bybit_parse_error(&response);
            let display = error.to_string();
            prop_assert!(!display.is_empty(), "Bybit error should have display even with missing code");

            let code = bybit_extract_code(&response);
            prop_assert_eq!(code, 0, "Bybit missing code should return 0");
        }

        /// Test that OkxErrorCode round-trips correctly
        #[test]
        fn prop_okx_error_code_roundtrip(code in okx_error_code_strategy()) {
            let parsed = OkxErrorCode::from_code(&code);
            let back = parsed.code();

            // For known codes, verify the round-trip
            match code.as_str() {
                "50000" => prop_assert_eq!(back, 50000),
                "50001" => prop_assert_eq!(back, 50001),
                "50002" => prop_assert_eq!(back, 50002),
                "50004" => prop_assert_eq!(back, 50004),
                "50011" => prop_assert_eq!(back, 50011),
                "51000" => prop_assert_eq!(back, 51000),
                "51001" => prop_assert_eq!(back, 51001),
                "51400" => prop_assert_eq!(back, 51400),
                _ => {
                    if let Ok(n) = code.parse::<i64>() {
                        prop_assert_eq!(back, n, "OKX unknown code should preserve numeric value");
                    }
                }
            }
        }

        /// Test that BybitErrorCode round-trips correctly
        #[test]
        fn prop_bybit_error_code_roundtrip(code in bybit_error_code_strategy()) {
            let parsed = BybitErrorCode::from_code(code);
            let back = parsed.code();

            // For known codes, verify the round-trip
            match code {
                10001 => prop_assert_eq!(back, 10001),
                10003 => prop_assert_eq!(back, 10003),
                10004 => prop_assert_eq!(back, 10004),
                10006 => prop_assert_eq!(back, 10006),
                10016 => prop_assert_eq!(back, 10016),
                110001 => prop_assert_eq!(back, 110001),
                110008 => prop_assert_eq!(back, 110008),
                _ => prop_assert_eq!(back, code, "Bybit unknown code should preserve numeric value"),
            }
        }
    }
}

/// For any price or amount string from exchange API, parsing to Decimal and
/// converting back to string SHALL preserve the original precision (no floating-point errors).
mod decimal_precision_preservation {
    use super::*;

    // Strategy for generating valid decimal strings with various precisions
    fn decimal_string_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Integer values
            (1u64..1000000u64).prop_map(|n| n.to_string()),
            // 1 decimal place
            (1u64..1000000u64).prop_map(|n| format!("{}.{}", n / 10, n % 10)),
            // 2 decimal places
            (1u64..1000000u64).prop_map(|n| format!("{}.{:02}", n / 100, n % 100)),
            // 4 decimal places
            (1u64..1000000u64).prop_map(|n| format!("{}.{:04}", n / 10000, n % 10000)),
            // 8 decimal places (common for crypto)
            (1u64..100000000u64).prop_map(|n| format!("{}.{:08}", n / 100000000, n % 100000000)),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that parsing a decimal string and converting back preserves the value
        #[test]
        fn prop_decimal_roundtrip_preserves_value(s in decimal_string_strategy()) {
            let decimal = Decimal::from_str(&s).expect("Should parse valid decimal string");
            let back = decimal.to_string();
            let reparsed = Decimal::from_str(&back).expect("Should reparse decimal string");

            prop_assert_eq!(
                decimal, reparsed,
                "Decimal round-trip should preserve value: {} -> {} -> {}",
                s, back, reparsed
            );
        }

        /// Test that decimal arithmetic doesn't introduce floating-point errors
        #[test]
        fn prop_decimal_arithmetic_no_fp_errors(
            a in decimal_string_strategy(),
            b in decimal_string_strategy()
        ) {
            let dec_a = Decimal::from_str(&a).expect("Should parse a");
            let dec_b = Decimal::from_str(&b).expect("Should parse b");

            // Addition should be exact
            let sum = dec_a + dec_b;
            let diff = sum - dec_b;
            prop_assert_eq!(diff, dec_a, "Addition/subtraction should be exact");

            // Multiplication should be exact for reasonable values
            if dec_a < Decimal::from(1000) && dec_b < Decimal::from(1000) {
                let product = dec_a * dec_b;
                if dec_b != Decimal::ZERO {
                    let quotient = product / dec_b;
                    prop_assert_eq!(quotient, dec_a, "Multiplication/division should be exact");
                }
            }
        }
    }
}

/// For any millisecond timestamp, converting to datetime string and back to timestamp
/// SHALL produce the same value (within millisecond precision).
mod timestamp_conversion_consistency {
    use super::*;
    use ccxt_exchanges::okx::parser::{datetime_to_timestamp, timestamp_to_datetime};

    // Strategy for generating valid timestamps (2020-2030 range)
    fn timestamp_strategy() -> impl Strategy<Value = i64> {
        1577836800000i64..1893456000000i64
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that timestamp to datetime conversion produces valid format
        #[test]
        fn prop_timestamp_to_datetime_valid_format(ts in timestamp_strategy()) {
            let datetime = timestamp_to_datetime(ts);
            prop_assert!(datetime.is_some(), "Should produce datetime for valid timestamp");

            let dt = datetime.unwrap();
            // Should be ISO8601 format
            prop_assert!(dt.contains("T"), "Datetime should contain 'T' separator");
            prop_assert!(dt.ends_with("Z"), "Datetime should end with 'Z'");
        }

        /// Test that timestamp round-trips through datetime conversion
        #[test]
        fn prop_timestamp_roundtrip(ts in timestamp_strategy()) {
            let datetime = timestamp_to_datetime(ts);
            prop_assert!(datetime.is_some(), "Should produce datetime");

            let back = datetime_to_timestamp(&datetime.unwrap());
            prop_assert!(back.is_some(), "Should parse datetime back to timestamp");

            prop_assert_eq!(
                back.unwrap(), ts,
                "Timestamp round-trip should preserve value"
            );
        }

        /// Test that datetime string contains expected components
        #[test]
        fn prop_datetime_contains_components(ts in timestamp_strategy()) {
            let datetime = timestamp_to_datetime(ts).unwrap();

            // Should contain year (2020-2030)
            prop_assert!(
                datetime.starts_with("202") || datetime.starts_with("203"),
                "Datetime should start with valid year"
            );

            // Should contain milliseconds
            prop_assert!(
                datetime.contains("."),
                "Datetime should contain milliseconds"
            );
        }
    }
}

/// For any valid OKX or Bybit order status string, the parser SHALL map it to
/// a valid OrderStatus enum variant without panicking.
mod order_status_mapping {
    use super::*;
    use ccxt_core::types::OrderStatus;
    use ccxt_exchanges::bybit::parser::parse_order_status as bybit_parse_status;
    use ccxt_exchanges::okx::parser::parse_order_status as okx_parse_status;

    // Strategy for generating OKX order status strings
    fn okx_status_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("live".to_string()),
            Just("partially_filled".to_string()),
            Just("filled".to_string()),
            Just("canceled".to_string()),
            Just("cancelled".to_string()),
            Just("mmp_canceled".to_string()),
            Just("expired".to_string()),
            Just("rejected".to_string()),
            // Unknown statuses
            "[a-z_]{3,15}",
        ]
    }

    // Strategy for generating Bybit order status strings
    fn bybit_status_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("New".to_string()),
            Just("Created".to_string()),
            Just("PartiallyFilled".to_string()),
            Just("Filled".to_string()),
            Just("Cancelled".to_string()),
            Just("Canceled".to_string()),
            Just("PartiallyFilledCanceled".to_string()),
            Just("Rejected".to_string()),
            Just("Expired".to_string()),
            Just("Triggered".to_string()),
            Just("Untriggered".to_string()),
            Just("Deactivated".to_string()),
            // Unknown statuses
            "[A-Za-z]{3,20}",
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that OKX parse_order_status never panics and returns valid status
        #[test]
        fn prop_okx_parse_order_status_never_panics(status in okx_status_strategy()) {
            // Should not panic
            let result = okx_parse_status(&status);

            // Should return a valid OrderStatus variant
            prop_assert!(
                matches!(
                    result,
                    OrderStatus::Open
                        | OrderStatus::Closed
                        | OrderStatus::Cancelled
                        | OrderStatus::Expired
                        | OrderStatus::Rejected
                ),
                "OKX should return valid OrderStatus for: {}",
                status
            );
        }

        /// Test that Bybit parse_order_status never panics and returns valid status
        #[test]
        fn prop_bybit_parse_order_status_never_panics(status in bybit_status_strategy()) {
            // Should not panic
            let result = bybit_parse_status(&status);

            // Should return a valid OrderStatus variant
            prop_assert!(
                matches!(
                    result,
                    OrderStatus::Open
                        | OrderStatus::Closed
                        | OrderStatus::Cancelled
                        | OrderStatus::Expired
                        | OrderStatus::Rejected
                ),
                "Bybit should return valid OrderStatus for: {}",
                status
            );
        }

        /// Test that OKX known statuses map correctly
        #[test]
        fn prop_okx_known_status_mapping(
            status in prop_oneof![
                Just("live"),
                Just("partially_filled"),
                Just("filled"),
                Just("canceled"),
                Just("mmp_canceled"),
                Just("expired"),
                Just("rejected"),
            ]
        ) {
            let result = okx_parse_status(status);

            match status {
                "live" | "partially_filled" => prop_assert_eq!(result, OrderStatus::Open),
                "filled" => prop_assert_eq!(result, OrderStatus::Closed),
                "canceled" | "mmp_canceled" => prop_assert_eq!(result, OrderStatus::Cancelled),
                "expired" => prop_assert_eq!(result, OrderStatus::Expired),
                "rejected" => prop_assert_eq!(result, OrderStatus::Rejected),
                _ => {}
            }
        }

        /// Test that Bybit known statuses map correctly
        #[test]
        fn prop_bybit_known_status_mapping(
            status in prop_oneof![
                Just("New"),
                Just("PartiallyFilled"),
                Just("Filled"),
                Just("Cancelled"),
                Just("Rejected"),
                Just("Expired"),
            ]
        ) {
            let result = bybit_parse_status(status);

            match status {
                "New" | "PartiallyFilled" => prop_assert_eq!(result, OrderStatus::Open),
                "Filled" => prop_assert_eq!(result, OrderStatus::Closed),
                "Cancelled" => prop_assert_eq!(result, OrderStatus::Cancelled),
                "Rejected" => prop_assert_eq!(result, OrderStatus::Rejected),
                "Expired" => prop_assert_eq!(result, OrderStatus::Expired),
                _ => {}
            }
        }
    }
}

/// For any valid Market struct, serializing to JSON and deserializing back
/// SHALL produce an equivalent Market struct.
mod market_data_roundtrip {
    use super::*;
    use ccxt_core::types::{Market, MarketLimits, MarketPrecision, MarketType, Symbol};
    use rust_decimal::prelude::FromPrimitive;

    // Strategy for generating valid market types
    fn market_type_strategy() -> impl Strategy<Value = MarketType> {
        prop_oneof![
            Just(MarketType::Spot),
            Just(MarketType::Futures),
            Just(MarketType::Swap),
            Just(MarketType::Option),
        ]
    }

    // Strategy for generating valid currency codes
    fn currency_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("BTC".to_string()),
            Just("ETH".to_string()),
            Just("USDT".to_string()),
            Just("USDC".to_string()),
            "[A-Z]{3,5}",
        ]
    }

    // Strategy for generating optional decimals
    #[allow(unused)]
    fn optional_decimal_strategy() -> impl Strategy<Value = Option<Decimal>> {
        prop_oneof![
            Just(None),
            (1u64..1000000u64)
                .prop_map(|n| Some(Decimal::from_u64(n).unwrap() / Decimal::from(10000))),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that Market serializes and deserializes correctly
        #[test]
        fn prop_market_roundtrip(
            base in currency_strategy(),
            quote in currency_strategy(),
            market_type in market_type_strategy(),
            active in any::<bool>(),
        ) {
            let symbol = format!("{}/{}", base, quote);
            let id = format!("{}{}", base, quote);

            let market = Market {
                id: id.clone(),
                symbol: Symbol::new_unchecked(symbol),
                parsed_symbol: None,
                base: base.clone(),
                quote: quote.clone(),
                settle: None,
                base_id: Some(base.clone()),
                quote_id: Some(quote.clone()),
                settle_id: None,
                market_type,
                active,
                margin: false,
                contract: Some(market_type != MarketType::Spot),
                linear: if market_type != MarketType::Spot { Some(true) } else { None },
                inverse: if market_type != MarketType::Spot { Some(false) } else { None },
                contract_size: None,
                expiry: None,
                expiry_datetime: None,
                strike: None,
                option_type: None,
                precision: MarketPrecision::default(),
                limits: MarketLimits::default(),
                maker: None,
                taker: None,
                percentage: Some(true),
                tier_based: Some(false),
                fee_side: Some("quote".to_string()),
                info: std::collections::HashMap::new(),
            };

            // Serialize to JSON
            let json = serde_json::to_string(&market).expect("Should serialize Market");

            // Deserialize back
            let deserialized: Market = serde_json::from_str(&json).expect("Should deserialize Market");

            // Verify key fields are preserved
            prop_assert_eq!(deserialized.id, market.id);
            prop_assert_eq!(deserialized.symbol, market.symbol);
            prop_assert_eq!(deserialized.base, market.base);
            prop_assert_eq!(deserialized.quote, market.quote);
            prop_assert_eq!(deserialized.market_type, market.market_type);
            prop_assert_eq!(deserialized.active, market.active);
        }
    }
}

/// For any valid Ticker struct, serializing to JSON and deserializing back
/// SHALL produce an equivalent Ticker struct.
mod ticker_data_roundtrip {
    use super::*;
    use ccxt_core::types::financial::Price;
    use ccxt_core::types::{Symbol, Ticker};
    use rust_decimal::prelude::FromPrimitive;

    // Strategy for generating valid symbols
    fn symbol_strategy() -> impl Strategy<Value = String> {
        "[A-Z]{3,5}/[A-Z]{3,5}"
    }

    // Strategy for generating valid timestamps
    fn timestamp_strategy() -> impl Strategy<Value = i64> {
        1577836800000i64..1893456000000i64
    }

    // Strategy for generating optional prices
    fn optional_price_strategy() -> impl Strategy<Value = Option<Price>> {
        prop_oneof![
            Just(None),
            (1u64..1000000u64).prop_map(|n| Some(Price::new(Decimal::from_u64(n).unwrap()))),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that Ticker serializes and deserializes correctly
        #[test]
        fn prop_ticker_roundtrip(
            symbol in symbol_strategy(),
            timestamp in timestamp_strategy(),
            last in optional_price_strategy(),
            high in optional_price_strategy(),
            low in optional_price_strategy(),
        ) {
            let ticker = Ticker {
                symbol: Symbol::new_unchecked(symbol.clone()),
                timestamp,
                datetime: None,
                high,
                low,
                bid: None,
                bid_volume: None,
                ask: None,
                ask_volume: None,
                vwap: None,
                open: None,
                close: last.clone(),
                last,
                previous_close: None,
                change: None,
                percentage: None,
                average: None,
                base_volume: None,
                quote_volume: None,
                funding_rate: None,
                open_interest: None,
                index_price: None,
                mark_price: None,
                info: std::collections::HashMap::new(),
            };

            // Serialize to JSON
            let json = serde_json::to_string(&ticker).expect("Should serialize Ticker");

            // Deserialize back
            let deserialized: Ticker = serde_json::from_str(&json).expect("Should deserialize Ticker");

            // Verify key fields are preserved
            prop_assert_eq!(deserialized.symbol, ticker.symbol);
            prop_assert_eq!(deserialized.timestamp, ticker.timestamp);
            prop_assert_eq!(deserialized.last, ticker.last);
            prop_assert_eq!(deserialized.high, ticker.high);
            prop_assert_eq!(deserialized.low, ticker.low);
        }
    }
}

/// For any valid Order struct, serializing to JSON and deserializing back
/// SHALL produce an equivalent Order struct.
mod order_data_roundtrip {
    use super::*;
    use ccxt_core::types::{Order, OrderSide, OrderStatus, OrderType, Symbol};
    use rust_decimal::prelude::FromPrimitive;

    // Strategy for generating valid order IDs
    fn order_id_strategy() -> impl Strategy<Value = String> {
        "[0-9]{8,20}"
    }

    // Strategy for generating valid symbols
    fn symbol_strategy() -> impl Strategy<Value = String> {
        "[A-Z]{3,5}/[A-Z]{3,5}"
    }

    // Strategy for generating order sides
    fn order_side_strategy() -> impl Strategy<Value = OrderSide> {
        prop_oneof![Just(OrderSide::Buy), Just(OrderSide::Sell),]
    }

    // Strategy for generating order types
    fn order_type_strategy() -> impl Strategy<Value = OrderType> {
        prop_oneof![Just(OrderType::Limit), Just(OrderType::Market),]
    }

    // Strategy for generating order statuses
    fn order_status_strategy() -> impl Strategy<Value = OrderStatus> {
        prop_oneof![
            Just(OrderStatus::Open),
            Just(OrderStatus::Closed),
            Just(OrderStatus::Cancelled),
        ]
    }

    // Strategy for generating amounts
    fn amount_strategy() -> impl Strategy<Value = Decimal> {
        (1u64..1000000u64).prop_map(|n| Decimal::from_u64(n).unwrap() / Decimal::from(10000))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that Order serializes and deserializes correctly
        #[test]
        fn prop_order_roundtrip(
            id in order_id_strategy(),
            symbol in symbol_strategy(),
            side in order_side_strategy(),
            order_type in order_type_strategy(),
            status in order_status_strategy(),
            amount in amount_strategy(),
        ) {
            let order = Order {
                id: id.clone(),
                client_order_id: None,
                timestamp: Some(1700000000000),
                datetime: None,
                last_trade_timestamp: None,
                status,
                symbol: Symbol::new_unchecked(symbol.clone()),
                order_type,
                time_in_force: Some("GTC".to_string()),
                side,
                price: Some(Decimal::from(50000)),
                average: None,
                amount,
                filled: Some(Decimal::ZERO),
                remaining: Some(amount),
                cost: None,
                trades: None,
                fee: None,
                post_only: None,
                reduce_only: None,
                trigger_price: None,
                stop_price: None,
                take_profit_price: None,
                stop_loss_price: None,
                trailing_delta: None,
                trailing_percent: None,
                activation_price: None,
                callback_rate: None,
                working_type: None,
                fees: None,
                info: std::collections::HashMap::new(),
            };

            // Serialize to JSON
            let json = serde_json::to_string(&order).expect("Should serialize Order");

            // Deserialize back
            let deserialized: Order = serde_json::from_str(&json).expect("Should deserialize Order");

            // Verify key fields are preserved
            prop_assert_eq!(deserialized.id, order.id);
            prop_assert_eq!(deserialized.symbol, order.symbol);
            prop_assert_eq!(deserialized.side, order.side);
            prop_assert_eq!(deserialized.order_type, order.order_type);
            prop_assert_eq!(deserialized.status, order.status);
            prop_assert_eq!(deserialized.amount, order.amount);
        }
    }
}

/// For any limit parameter value exceeding the exchange's maximum, the system
/// SHALL cap the limit to the exchange's maximum allowed value without error.
mod limit_parameter_capping {
    use super::*;

    // OKX and Bybit typically have max limits around 100-500 for orderbook depth
    const OKX_MAX_ORDERBOOK_LIMIT: u32 = 400;
    const BYBIT_MAX_ORDERBOOK_LIMIT: u32 = 500;

    // Helper function to cap limit (simulating exchange behavior)
    fn cap_limit(limit: u32, max: u32) -> u32 {
        std::cmp::min(limit, max)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that OKX limit capping works correctly
        #[test]
        fn prop_okx_limit_capping(limit in 1u32..10000u32) {
            let capped = cap_limit(limit, OKX_MAX_ORDERBOOK_LIMIT);

            prop_assert!(
                capped <= OKX_MAX_ORDERBOOK_LIMIT,
                "OKX capped limit should not exceed max: {} <= {}",
                capped,
                OKX_MAX_ORDERBOOK_LIMIT
            );

            if limit <= OKX_MAX_ORDERBOOK_LIMIT {
                prop_assert_eq!(capped, limit, "Limit within range should be unchanged");
            } else {
                prop_assert_eq!(capped, OKX_MAX_ORDERBOOK_LIMIT, "Limit exceeding max should be capped");
            }
        }

        /// Test that Bybit limit capping works correctly
        #[test]
        fn prop_bybit_limit_capping(limit in 1u32..10000u32) {
            let capped = cap_limit(limit, BYBIT_MAX_ORDERBOOK_LIMIT);

            prop_assert!(
                capped <= BYBIT_MAX_ORDERBOOK_LIMIT,
                "Bybit capped limit should not exceed max: {} <= {}",
                capped,
                BYBIT_MAX_ORDERBOOK_LIMIT
            );

            if limit <= BYBIT_MAX_ORDERBOOK_LIMIT {
                prop_assert_eq!(capped, limit, "Limit within range should be unchanged");
            } else {
                prop_assert_eq!(capped, BYBIT_MAX_ORDERBOOK_LIMIT, "Limit exceeding max should be capped");
            }
        }

        /// Test that capping is idempotent
        #[test]
        fn prop_limit_capping_idempotent(limit in 1u32..10000u32) {
            let capped_once = cap_limit(limit, OKX_MAX_ORDERBOOK_LIMIT);
            let capped_twice = cap_limit(capped_once, OKX_MAX_ORDERBOOK_LIMIT);

            prop_assert_eq!(
                capped_once, capped_twice,
                "Capping should be idempotent"
            );
        }
    }
}

/// For any valid request parameters (timestamp, method, path, body), the OkxAuth
/// `sign()` method SHALL produce a deterministic HMAC-SHA256 signature that is
/// identical when called multiple times with the same inputs.

/// For any valid request parameters (timestamp, recv_window, params), the BybitAuth
/// `sign()` method SHALL produce a deterministic HMAC-SHA256 signature that is
/// identical when called multiple times with the same inputs.
mod bybit_signature_determinism {
    use super::*;
    use ccxt_exchanges::bybit::BybitAuth;

    // Strategy for generating valid Unix timestamps in milliseconds
    fn timestamp_strategy() -> impl Strategy<Value = String> {
        (1609459200000u64..1893456000000u64).prop_map(|ts| ts.to_string())
    }

    // Strategy for generating recv_window values (1000-30000 ms)
    fn recv_window_strategy() -> impl Strategy<Value = u64> {
        1000u64..30000u64
    }

    // Strategy for generating query params (for GET requests)
    fn params_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("".to_string()),
            Just("symbol=BTCUSDT".to_string()),
            Just("category=spot&symbol=BTCUSDT".to_string()),
            Just("category=linear&symbol=BTCUSDT&limit=100".to_string()),
            // Random query-like params
            "[a-zA-Z0-9=&]{0,100}",
        ]
    }

    // Strategy for generating JSON body params (for POST requests)
    fn body_params_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("".to_string()),
            Just(r#"{"category":"spot","symbol":"BTCUSDT"}"#.to_string()),
            Just(r#"{"category":"spot","symbol":"BTCUSDT","side":"Buy","orderType":"Limit","qty":"0.001","price":"50000"}"#.to_string()),
            // Random JSON-like bodies
            "\\{[a-zA-Z0-9\":,\\-_]{0,100}\\}",
        ]
    }

    // Strategy for generating API credentials
    fn credentials_strategy() -> impl Strategy<Value = (String, String)> {
        (
            "[a-zA-Z0-9]{16,32}", // api_key
            "[a-zA-Z0-9]{32,64}", // secret
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that Bybit sign() produces identical signatures for identical inputs
        #[test]
        fn prop_bybit_sign_deterministic(
            (api_key, secret) in credentials_strategy(),
            timestamp in timestamp_strategy(),
            recv_window in recv_window_strategy(),
            params in params_strategy(),
        ) {
            let auth = BybitAuth::new(api_key, secret);

            // Call sign() multiple times with the same inputs
            let sig1 = auth.sign(&timestamp, recv_window, &params);
            let sig2 = auth.sign(&timestamp, recv_window, &params);
            let sig3 = auth.sign(&timestamp, recv_window, &params);

            // All signatures should be identical
            prop_assert_eq!(
                &sig1, &sig2,
                "Bybit signatures should be deterministic: first call != second call"
            );
            prop_assert_eq!(
                &sig2, &sig3,
                "Bybit signatures should be deterministic: second call != third call"
            );

            // Signature should not be empty
            prop_assert!(
                !sig1.is_empty(),
                "Bybit signature should not be empty"
            );
        }

        /// Test that Bybit sign() produces valid hex output (HMAC-SHA256)
        #[test]
        fn prop_bybit_sign_produces_valid_hex(
            (api_key, secret) in credentials_strategy(),
            timestamp in timestamp_strategy(),
            recv_window in recv_window_strategy(),
            params in params_strategy(),
        ) {
            let auth = BybitAuth::new(api_key, secret);
            let signature = auth.sign(&timestamp, recv_window, &params);

            // Signature should be valid hex
            prop_assert!(
                signature.chars().all(|c| c.is_ascii_hexdigit()),
                "Bybit signature should be valid hex: {}",
                signature
            );

            // HMAC-SHA256 produces 32 bytes = 64 hex characters
            prop_assert_eq!(
                signature.len(),
                64,
                "Bybit signature should be 64 hex characters (32 bytes HMAC-SHA256)"
            );

            // Should be decodable as hex
            let decoded = hex::decode(&signature);
            prop_assert!(
                decoded.is_ok(),
                "Bybit signature should be decodable as hex"
            );
            prop_assert_eq!(
                decoded.unwrap().len(),
                32,
                "Bybit signature should decode to 32 bytes (HMAC-SHA256)"
            );
        }

        /// Test that different timestamps produce different signatures
        #[test]
        fn prop_bybit_sign_different_timestamps_different_signatures(
            (api_key, secret) in credentials_strategy(),
            timestamp1 in timestamp_strategy(),
            timestamp2 in timestamp_strategy(),
            recv_window in recv_window_strategy(),
            params in params_strategy(),
        ) {
            // Skip if timestamps happen to be the same
            prop_assume!(timestamp1 != timestamp2);

            let auth = BybitAuth::new(api_key, secret);

            let sig1 = auth.sign(&timestamp1, recv_window, &params);
            let sig2 = auth.sign(&timestamp2, recv_window, &params);

            prop_assert_ne!(
                sig1, sig2,
                "Different timestamps should produce different signatures"
            );
        }

        /// Test that different recv_window values produce different signatures
        #[test]
        fn prop_bybit_sign_different_recv_window_different_signatures(
            (api_key, secret) in credentials_strategy(),
            timestamp in timestamp_strategy(),
            recv_window1 in recv_window_strategy(),
            recv_window2 in recv_window_strategy(),
            params in params_strategy(),
        ) {
            // Skip if recv_window values happen to be the same
            prop_assume!(recv_window1 != recv_window2);

            let auth = BybitAuth::new(api_key, secret);

            let sig1 = auth.sign(&timestamp, recv_window1, &params);
            let sig2 = auth.sign(&timestamp, recv_window2, &params);

            prop_assert_ne!(
                sig1, sig2,
                "Different recv_window values should produce different signatures"
            );
        }

        /// Test that different params produce different signatures
        #[test]
        fn prop_bybit_sign_different_params_different_signatures(
            (api_key, secret) in credentials_strategy(),
            timestamp in timestamp_strategy(),
            recv_window in recv_window_strategy(),
            params1 in params_strategy(),
            params2 in params_strategy(),
        ) {
            // Skip if params happen to be the same
            prop_assume!(params1 != params2);

            let auth = BybitAuth::new(api_key, secret);

            let sig1 = auth.sign(&timestamp, recv_window, &params1);
            let sig2 = auth.sign(&timestamp, recv_window, &params2);

            prop_assert_ne!(
                sig1, sig2,
                "Different params should produce different signatures"
            );
        }

        /// Test that different secrets produce different signatures
        #[test]
        fn prop_bybit_sign_different_secrets_different_signatures(
            api_key in "[a-zA-Z0-9]{16,32}",
            secret1 in "[a-zA-Z0-9]{32,64}",
            secret2 in "[a-zA-Z0-9]{32,64}",
            timestamp in timestamp_strategy(),
            recv_window in recv_window_strategy(),
            params in params_strategy(),
        ) {
            // Skip if secrets happen to be the same
            prop_assume!(secret1 != secret2);

            let auth1 = BybitAuth::new(api_key.clone(), secret1);
            let auth2 = BybitAuth::new(api_key, secret2);

            let sig1 = auth1.sign(&timestamp, recv_window, &params);
            let sig2 = auth2.sign(&timestamp, recv_window, &params);

            prop_assert_ne!(
                sig1, sig2,
                "Different secrets should produce different signatures"
            );
        }

        /// Test that sign_string is built correctly
        #[test]
        fn prop_bybit_build_sign_string_format(
            (api_key, secret) in credentials_strategy(),
            timestamp in timestamp_strategy(),
            recv_window in recv_window_strategy(),
            params in params_strategy(),
        ) {
            let auth = BybitAuth::new(api_key.clone(), secret);

            let sign_string = auth.build_sign_string(&timestamp, recv_window, &params);

            // Sign string format: timestamp + api_key + recv_window + params
            let expected = format!("{}{}{}{}", timestamp, api_key, recv_window, params);
            prop_assert_eq!(
                sign_string, expected,
                "Bybit sign string should follow format: timestamp + api_key + recv_window + params"
            );
        }

        /// Test signature with empty params (common for some GET requests)
        #[test]
        fn prop_bybit_sign_with_empty_params(
            (api_key, secret) in credentials_strategy(),
            timestamp in timestamp_strategy(),
            recv_window in recv_window_strategy(),
        ) {
            let auth = BybitAuth::new(api_key, secret);
            let signature = auth.sign(&timestamp, recv_window, "");

            // Signature should still be valid
            prop_assert!(
                !signature.is_empty(),
                "Bybit signature should not be empty even with empty params"
            );
            prop_assert_eq!(
                signature.len(),
                64,
                "Bybit signature should be 64 hex characters"
            );
            prop_assert!(
                signature.chars().all(|c| c.is_ascii_hexdigit()),
                "Bybit signature should be valid hex"
            );
        }

        /// Test signature with JSON body params (POST requests)
        #[test]
        fn prop_bybit_sign_with_json_body(
            (api_key, secret) in credentials_strategy(),
            timestamp in timestamp_strategy(),
            recv_window in recv_window_strategy(),
            body in body_params_strategy(),
        ) {
            let auth = BybitAuth::new(api_key, secret);
            let signature = auth.sign(&timestamp, recv_window, &body);

            // Signature should be valid
            prop_assert!(
                !signature.is_empty(),
                "Bybit signature should not be empty with JSON body"
            );
            prop_assert_eq!(
                signature.len(),
                64,
                "Bybit signature should be 64 hex characters"
            );
            prop_assert!(
                signature.chars().all(|c| c.is_ascii_hexdigit()),
                "Bybit signature should be valid hex"
            );

            // Verify determinism with JSON body
            let sig2 = auth.sign(&timestamp, recv_window, &body);
            prop_assert_eq!(
                signature, sig2,
                "Bybit signature should be deterministic with JSON body"
            );
        }
    }
}

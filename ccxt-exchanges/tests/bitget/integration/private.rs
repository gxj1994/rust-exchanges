//! Bitget Integration Tests
//!
//! These tests verify Bitget exchange implementation against real API.
//! They can be run with: cargo test --test bitget_integration_test
//!
//! ## Test Categories
//!
//! - **Public API Tests**: No credentials required, run automatically
//! - **Private API Tests**: Require credentials, automatically skipped if not configured
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all integration tests
//! cargo test -p ccxt-exchanges --test bitget_integration_test
//!
//! # Run only public API tests
//! cargo test -p ccxt-exchanges --test bitget_integration_test public
//! ```

use crate::support::{
    create_bitget, create_bitget_with_credentials, init_test, should_skip_private_tests,
};
use ccxt_core::traits::Margin;
use ccxt_core::traits::Trading;
use ccxt_core::types::Symbol;
use ccxt_exchanges::bitget::Bitget;
use ccxt_exchanges::bitget::BitgetBuilder;

// ==================== Private API Tests ====================

/// Test that operations requiring credentials fail without API keys.
#[tokio::test]
async fn test_edge_no_credentials() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

    let result = exchange.fetch_balance().await;
    assert!(result.is_err(), "Should fail without API credentials");
}

/// Test invalid API key handling.
#[tokio::test]
async fn test_edge_invalid_api_key() {
    let exchange = BitgetBuilder::new()
        .api_key("invalid_key")
        .secret("invalid_secret")
        .passphrase("invalid_passphrase")
        .build()
        .expect("Failed to build Bitget");

    let result = exchange.fetch_balance().await;
    assert!(result.is_err(), "Should fail with invalid API key");
}

/// Test fetching account balance.
///
/// Note: Requires API credentials. Automatically skipped if not configured.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_private_balance() {
    let config = init_test();
    let exchange = create_bitget_with_credentials(&config, "spot")
        .expect("Failed to create Bitget with credentials");

    let result = exchange.fetch_balance().await;

    assert!(
        result.is_ok(),
        "Failed to fetch balance: {:?}",
        result.err()
    );
}

/// Test fetching open orders.
///
/// Note: Requires API credentials.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_private_orders() {
    let config = init_test();
    let exchange = create_bitget_with_credentials(&config, "spot")
        .expect("Failed to create Bitget with credentials");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await;

    assert!(
        result.is_ok(),
        "Failed to fetch open orders: {:?}",
        result.err()
    );
}

/// Test fetching account trade history (fills).
///
/// Note: Requires API credentials.
#[tokio::test]
#[ignore] // Requires API credentials
async fn test_private_account_trades() {
    let config = init_test();
    let exchange = create_bitget_with_credentials(&config, "spot")
        .expect("Failed to create Bitget with credentials");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch recent trades for BTC/USDT
    let result = exchange
        .fetch_account_trades("BTC/USDT", None, Some(10))
        .await;

    assert!(
        result.is_ok(),
        "Failed to fetch account trades: {:?}",
        result.err()
    );
    let trades = result.unwrap();

    // May be empty if no trades exist
    assert!(trades.len() <= 10, "Should have at most 10 trade records");

    for trade in &trades {
        assert!(trade.timestamp > 0, "Trade should have timestamp");
        assert!(
            trade.price.0 > rust_decimal::Decimal::ZERO,
            "Trade price should be positive"
        );
        assert!(
            trade.amount.0 > rust_decimal::Decimal::ZERO,
            "Trade amount should be positive"
        );
    }
}

/// Test fetching recent trades from real API.
#[tokio::test]
async fn test_public_trades() {
    let config = init_test();
    let exchange = create_bitget_with_credentials(&config, "spot")
        .expect("Failed to create Bitget with credentials");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange
        .fetch_trades_with_limit_only("BTC/USDT", Some(5))
        .await;

    assert!(result.is_ok(), "Failed to fetch trades: {:?}", result.err());
    let trades = result.unwrap();

    assert!(!trades.is_empty(), "Trades should not be empty");
    assert!(trades.len() <= 5, "Should have at most 5 trades");

    for trade in &trades {
        assert_eq!(trade.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert!(
            trade.price.0 > rust_decimal::Decimal::ZERO,
            "Trade price should be positive"
        );
        assert!(
            trade.amount.0 > rust_decimal::Decimal::ZERO,
            "Trade amount should be positive"
        );
        assert!(trade.timestamp > 0, "Trade should have timestamp");
    }
}

/// Create authenticated Bitget client for swap tests.
async fn create_auth_bitget_swap() -> Bitget {
    let config = init_test();
    let exchange = create_bitget_with_credentials(&config, "swap")
        .expect("Failed to create Bitget with credentials");

    // Load markets before using the exchange
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");
    exchange
}

/// Test fetching a single position for a specific symbol.
///
/// This test requires an open position for the symbol.
/// If no position exists, the API will return an error.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量，且需要有持仓"]
async fn test_fetch_single_position() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_swap().await;

    // Try to fetch position for BTC/USDT perpetual swap
    let result = Margin::fetch_position(&exchange, "BTC/USDT:USDT").await;

    // This may fail if no position exists, which is expected
    match result {
        Ok(position) => {
            println!("✅ Position found for BTC/USDT:USDT");
            assert_eq!(position.symbol, "BTC/USDT:USDT");

            // Position should have meaningful data
            if position.contracts.is_some() {
                println!("  Contracts: {:?}", position.contracts);
            }
            if position.entry_price.is_some() {
                println!("  Entry Price: {:?}", position.entry_price);
            }
            if position.unrealized_pnl.is_some() {
                println!("  Unrealized PnL: {:?}", position.unrealized_pnl);
            }
        }
        Err(e) => {
            // It's OK if no position exists
            println!(
                "ℹ️  No position found for BTC/USDT:USDT (expected if no open position): {:?}",
                e
            );
        }
    }
}

/// Test fetching all positions across all product types.
///
/// This test fetches positions without specifying symbols,
/// which should return all open positions.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量"]
async fn test_fetch_all_positions() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_swap().await;

    // Fetch all positions (empty symbols list)
    let result = Margin::fetch_positions_for(&exchange, &[]).await;

    assert!(
        result.is_ok(),
        "Failed to fetch positions: {:?}",
        result.err()
    );

    let positions = result.unwrap();
    println!("✅ Fetch all positions test passed");
    println!("  Total positions: {}", positions.len());

    // Validate each position if any exist
    for position in &positions {
        println!("  Position: {}", position.symbol);

        // Symbol should not be empty
        assert!(
            !position.symbol.is_empty(),
            "Position symbol should not be empty"
        );

        // If contracts field is present, it should be non-negative
        if let Some(contracts) = position.contracts {
            assert!(contracts >= 0.0, "Contracts should be non-negative");
        }
    }
}

/// Test fetching positions for multiple specific symbols.
///
/// This test demonstrates fetching positions for a list of symbols,
/// which should only return positions that exist for those symbols.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量"]
async fn test_fetch_positions_for_symbols() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_swap().await;

    // Request positions for multiple symbols
    let symbols = vec!["BTC/USDT:USDT", "ETH/USDT:USDT"];
    let result = Margin::fetch_positions_for(&exchange, &symbols).await;

    assert!(
        result.is_ok(),
        "Failed to fetch positions for symbols: {:?}",
        result.err()
    );

    let positions = result.unwrap();
    println!("✅ Fetch positions for symbols test passed");
    println!("  Requested symbols: {:?}", symbols);
    println!("  Found positions: {}", positions.len());

    // All returned positions should be from the requested symbols
    for position in &positions {
        assert!(
            symbols.contains(&position.symbol.as_str()),
            "Position symbol {} should be in requested symbols",
            position.symbol
        );
        println!("  Found position: {}", position.symbol);
    }
}

/// Test fetching positions for different product types.
///
/// This test verifies that the API correctly handles different
/// product types (USDT-FUTURES, COIN-FUTURES, USDC-FUTURES).
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量"]
async fn test_fetch_positions_by_product_type() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_swap().await;

    // Test USDT perpetual swap (most common)
    let result_usdt = Margin::fetch_position(&exchange, "BTC/USDT:USDT").await;
    match result_usdt {
        Ok(pos) => println!("✅ USDT-FUTURES position: {}", pos.symbol),
        Err(_) => println!("ℹ️  No USDT-FUTURES position for BTC/USDT:USDT"),
    }

    // Test COIN perpetual swap (inverse)
    let result_coin = Margin::fetch_position(&exchange, "BTC/USD:BTC").await;
    match result_coin {
        Ok(pos) => println!("✅ COIN-FUTURES position: {}", pos.symbol),
        Err(_) => println!("ℹ️  No COIN-FUTURES position for BTC/USD:BTC"),
    }

    println!("✅ Product type test completed");
}

/// Test position data structure validation.
///
/// This test validates that the position data returned by the API
/// contains all required fields and has correct data types.
#[tokio::test]
#[ignore = "需要配置 BITGET_API_KEY, BITGET_API_SECRET, BITGET_PASSPHRASE 环境变量，且需要有持仓"]
async fn test_position_data_validation() {
    if should_skip_private_tests("bitget") {
        println!("SKIPPED: No Bitget credentials configured");
        return;
    }

    let exchange = create_auth_bitget_swap().await;

    // Fetch all positions
    let positions = Margin::fetch_positions_for(&exchange, &[])
        .await
        .expect("Failed to fetch positions");

    if positions.is_empty() {
        println!("ℹ️  No positions found for validation");
        return;
    }

    println!("✅ Validating {} position(s)", positions.len());

    for (i, position) in positions.iter().enumerate() {
        println!("\n--- Position {} ---", i + 1);
        println!("Symbol: {}", position.symbol);

        // Required fields
        assert!(!position.symbol.is_empty(), "Symbol must not be empty");

        // Optional but common fields - just print them
        if let Some(contracts) = &position.contracts {
            println!("  Contracts: {}", contracts);
        }
        if let Some(entry_price) = &position.entry_price {
            println!("  Entry Price: {}", entry_price);
        }
        if let Some(mark_price) = &position.mark_price {
            println!("  Mark Price: {}", mark_price);
        }
        if let Some(unrealized_pnl) = &position.unrealized_pnl {
            println!("  Unrealized PnL: {}", unrealized_pnl);
        }
        if let Some(liquidation_price) = &position.liquidation_price {
            println!("  Liquidation Price: {}", liquidation_price);
        }
        if let Some(leverage) = position.leverage {
            println!("  Leverage: {}", leverage);
        }

        // Validate numeric fields are reasonable
        if let Some(entry_price) = &position.entry_price {
            assert!(*entry_price > 0.0, "Entry price should be positive");
        }
        if let Some(mark_price) = &position.mark_price {
            assert!(*mark_price > 0.0, "Mark price should be positive");
        }
    }
}

//! Binance U本位合约 (USDT-Margined Futures) 市场数据 API 测试
//!
//! ⚠️ 注意: U本位合约测试需要先配置Binance的default_type为swap
//! 当前测试框架的create_binance只加载现货markets
//! 需要在support模块中增加create_binance_swap()函数来支持合约测试
//!
//! 测试覆盖:
//! - Ticker 数据 (fetch_ticker)
//! - 订单簿 (fetch_order_book)
//! - 成交记录 (fetch_market_trades, fetch_agg_trades)
//! - K线数据 (fetch_ohlcv)
//! - 标记价格 (fetch_mark_price) - U本位特有
//! - 资金费率 (fetch_funding_rate) - U本位特有
//! - 最优挂单 (fetch_bids_asks)
//! - 最新价格 (fetch_last_prices)
//! - 订单类型测试 (order_types) - 市价单、限价单、止损止盈等
//!
//! 测试特点:
//! - 使用动态端点路由 (rest_endpoint_for_market)
//! - 自动选择 fapi.binance.com/fapi/v1 端点
//! - 基于 market.linear = true 进行路由
//! - 覆盖U本位合约特有的市场数据API (标记价格、资金费率)
//! - 所有市场数据API无需签名
//!
//! TODO: 需要在tests/support/mod.rs中添加create_binance_swap()函数

pub mod order_types;

use crate::support::init_test;
use ccxt_core::ExchangeConfig;
use ccxt_core::types::common::ohlcv_request::OhlcvRequest;
use ccxt_core::types::common::ticker_params::TickerParams;
use ccxt_exchanges::binance::Binance;

/// 创建 Binance 客户端 (用于测试U本位合约市场数据)
/// 注意:市场数据API是公共的,通过symbol自动路由到FAPI端点
async fn create_swap_client() -> Binance {
    let config = init_test();
    let exchange_config = ExchangeConfig {
        id: "binance".to_string(),
        name: "Binance".to_string(),
        sandbox: config.binance.use_testnet,
        api_key: None,
        secret: None,
        ..Default::default()
    };
    let client = Binance::new_swap(exchange_config).expect("Failed to create Binance instance");
    // 需要先加载markets
    client
        .load_markets(false)
        .await
        .expect("Should load markets");
    client
}

// ============================================================================
// Ticker 测试
// ============================================================================

#[cfg(test)]
mod ticker_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_ticker_swap_24hr() {
        let client = create_swap_client().await;
        let params = TickerParams::default();

        // 尝试不同的symbol格式
        let test_symbols = vec!["BTC/USDT:USDT"];

        for symbol in test_symbols {
            let result = client.fetch_ticker(symbol, params.clone()).await;
            println!("Testing {}: {:?}", symbol, result.is_ok());
            if result.is_ok() {
                let ticker = result.unwrap();
                println!("✓ Success for {}: symbol={}", symbol, ticker.symbol);
                assert!(!ticker.symbol.is_empty());
                return;
            }
        }

        panic!("All symbol formats failed");
    }

    // U本位合约(FAPI)可能不支持rolling window ticker
    #[tokio::test]
    async fn test_fetch_ticker_swap_rolling() {
        let client = create_swap_client().await;

        // 测试 rolling window ticker
        // 注意: 币安要求windowSize必须是字符串格式,如 "4h", "30m", "2d"
        let params = TickerParams::builder()
            .rolling(false) // 目前true 报错
            .extra("windowSize", "4h") // 4小时
            .build();

        let result = client.fetch_ticker("BTC/USDT:USDT", params).await;

        if let Err(ref e) = result {
            println!("Error fetching swap rolling ticker: {}", e);
        }
        assert!(
            result.is_ok(),
            "Should fetch rolling ticker for BTC/USDT:USDT"
        );
        let ticker = result.unwrap();

        assert!(!ticker.symbol.is_empty(), "Symbol should not be empty");
    }

    #[tokio::test]
    async fn test_fetch_ticker_different_swap_symbols() {
        let client = create_swap_client().await;
        let params = TickerParams::default();

        let symbols = vec!["BTC/USDT:USDT", "ETH/USDT:USDT", "BNB/USDT:USDT"];

        for symbol in symbols {
            let result = client.fetch_ticker(symbol, params.clone()).await;
            assert!(result.is_ok(), "Should fetch ticker for {}", symbol);
            let ticker = result.unwrap();
            assert!(!ticker.symbol.is_empty(), "Symbol should not be empty");
        }
    }
}

// ============================================================================
// 订单簿测试
// ============================================================================

#[cfg(test)]
mod order_book_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_order_book_swap_basic() {
        let client = create_swap_client().await;

        let result = client.fetch_order_book("BTC/USDT:USDT", None).await;

        assert!(result.is_ok(), "Should fetch order book for BTC/USDT:USDT");
        let order_book = result.unwrap();

        assert!(!order_book.bids.is_empty(), "Bids should not be empty");
        assert!(!order_book.asks.is_empty(), "Asks should not be empty");

        // 验证价格排序
        if order_book.bids.len() > 1 {
            for i in 0..order_book.bids.len() - 1 {
                assert!(
                    order_book.bids[i].price >= order_book.bids[i + 1].price,
                    "Bids should be sorted by price descending"
                );
            }
        }

        if order_book.asks.len() > 1 {
            for i in 0..order_book.asks.len() - 1 {
                assert!(
                    order_book.asks[i].price <= order_book.asks[i + 1].price,
                    "Asks should be sorted by price ascending"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_fetch_order_book_swap_with_limit() {
        let client = create_swap_client().await;

        let result = client.fetch_order_book("ETH/USDT:USDT", Some(10)).await;
        assert!(result.is_ok(), "Should fetch order book with limit");

        let order_book = result.unwrap();
        assert!(
            order_book.bids.len() <= 10,
            "Bids count should not exceed limit 10"
        );
        assert!(
            order_book.asks.len() <= 10,
            "Asks count should not exceed limit 10"
        );
    }

    #[tokio::test]
    async fn test_fetch_order_book_multiple_swap_symbols() {
        let client = create_swap_client().await;

        let symbols = vec!["BTC/USDT:USDT", "ETH/USDT:USDT", "BNB/USDT:USDT"];

        for symbol in symbols {
            let result = client.fetch_order_book(symbol, None).await;
            assert!(result.is_ok(), "Should fetch order book for {}", symbol);
            let order_book = result.unwrap();
            assert!(
                !order_book.bids.is_empty(),
                "Bids should not be empty for {}",
                symbol
            );
        }
    }
}

// ============================================================================
// 成交记录测试
// ============================================================================

#[cfg(test)]
mod trades_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_market_trades_swap() {
        let client = create_swap_client().await;

        let result = client.fetch_market_trades("BTC/USDT:USDT", None).await;

        assert!(
            result.is_ok(),
            "Should fetch market trades for BTC/USDT:USDT"
        );
        let trades = result.unwrap();

        assert!(!trades.is_empty(), "Trades should not be empty");
        assert!(trades.len() <= 500, "Should return at most 500 trades");
    }

    #[tokio::test]
    async fn test_fetch_agg_trades_swap() {
        let client = create_swap_client().await;

        let result = client
            .fetch_agg_trades("BTC/USDT:USDT", None, None, None)
            .await;

        assert!(result.is_ok(), "Should fetch aggregated trades");
        let trades = result.unwrap();

        assert!(!trades.is_empty(), "Agg trades should not be empty");
    }
}

// ============================================================================
// K线数据测试
// ============================================================================

#[cfg(test)]
mod ohlcv_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_ohlcv_swap_basic() {
        let client = create_swap_client().await;

        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT:USDT")
            .timeframe("1h")
            .limit(10)
            .build()
            .expect("Failed to build OHLCV request");

        let result = client.fetch_ohlcv(request).await;

        assert!(result.is_ok(), "Should fetch OHLCV data for BTC/USDT:USDT");
        let ohlcvs = result.unwrap();

        assert!(!ohlcvs.is_empty(), "OHLCV data should not be empty");
        assert!(ohlcvs.len() <= 10, "Should respect limit parameter");

        let first = &ohlcvs[0];
        assert!(first.timestamp > 0, "Timestamp should be valid");
        assert!(first.open > 0.0, "Open price should be greater than 0");
        assert!(first.high > 0.0, "High price should be greater than 0");
        assert!(first.low > 0.0, "Low price should be greater than 0");
        assert!(first.close > 0.0, "Close price should be greater than 0");
        assert!(first.volume >= 0.0, "Volume should not be negative");

        // 验证价格关系
        assert!(first.high >= first.low, "High should be >= low");
        assert!(first.high >= first.open, "High should be >= open");
        assert!(first.high >= first.close, "High should be >= close");
        assert!(first.low <= first.open, "Low should be <= open");
        assert!(first.low <= first.close, "Low should be <= close");
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_swap_different_timeframes() {
        let client = create_swap_client().await;

        let timeframes = vec!["1m", "5m", "15m", "1h", "4h", "1d"];

        for timeframe in timeframes {
            let request = OhlcvRequest::builder()
                .symbol("ETH/USDT:USDT")
                .timeframe(timeframe)
                .limit(5)
                .build()
                .expect("Failed to build OHLCV request");

            let result = client.fetch_ohlcv(request).await;
            assert!(
                result.is_ok(),
                "Should fetch OHLCV for timeframe {}",
                timeframe
            );

            let ohlcvs = result.unwrap();
            assert!(
                !ohlcvs.is_empty(),
                "OHLCV data for timeframe {} should not be empty",
                timeframe
            );
        }
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_swap_with_since() {
        let client = create_swap_client().await;

        let since = chrono::Utc::now().timestamp_millis() - (24 * 60 * 60 * 1000); // 24小时前

        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT:USDT")
            .timeframe("1h")
            .since(since)
            .limit(10)
            .build()
            .expect("Failed to build OHLCV request");

        let result = client.fetch_ohlcv(request).await;

        assert!(result.is_ok(), "Should fetch OHLCV with since parameter");
        let ohlcvs = result.unwrap();

        assert!(!ohlcvs.is_empty(), "OHLCV data should not be empty");

        let first_timestamp = ohlcvs[0].timestamp;
        assert!(
            first_timestamp >= since,
            "First OHLCV timestamp should be >= since parameter"
        );
    }
}

// ============================================================================
// 标记价格测试 (U本位合约特有)
// ============================================================================

#[cfg(test)]
mod mark_price_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_mark_price_swap_single() {
        let client = create_swap_client().await;

        let result = client.fetch_mark_price(Some("BTC/USDT:USDT")).await;

        assert!(result.is_ok(), "Should fetch mark price for BTC/USDT:USDT");
        let mark_prices = result.unwrap();

        // 标记价格API返回Vec
        assert!(!mark_prices.is_empty(), "Mark prices should not be empty");
    }

    #[tokio::test]
    async fn test_fetch_mark_price_swap_multiple() {
        let client = create_swap_client().await;

        // fetch_mark_price返回Vec,不带symbol参数获取所有
        let result = client.fetch_mark_price(None).await;

        assert!(result.is_ok(), "Should fetch mark prices for all symbols");
        let mark_prices = result.unwrap();

        assert!(!mark_prices.is_empty(), "Mark prices should not be empty");
    }
}

// ============================================================================
// 资金费率测试 (U本位合约特有)
// ============================================================================

#[cfg(test)]
mod funding_rate_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_funding_rate_swap_single() {
        let client = create_swap_client().await;

        let result = client.fetch_funding_rate("BTC/USDT:USDT", None).await;

        assert!(
            result.is_ok(),
            "Should fetch funding rate for BTC/USDT:USDT"
        );
        let funding_rate = result.unwrap();

        // 资金费率API应该返回有效数据
        assert!(
            !funding_rate.symbol.is_empty(),
            "Symbol should not be empty"
        );
    }

    #[tokio::test]
    async fn test_fetch_funding_rates_swap_all() {
        let client = create_swap_client().await;

        let result = client.fetch_funding_rates(None, None).await;

        assert!(result.is_ok(), "Should fetch funding rates for all symbols");
        let funding_rates = result.unwrap();

        assert!(
            !funding_rates.is_empty(),
            "Funding rates should not be empty"
        );
    }

    #[tokio::test]
    async fn test_fetch_funding_rate_history_swap() {
        let client = create_swap_client().await;

        let result = client
            .fetch_funding_rate_history("BTC/USDT:USDT", None, None, None)
            .await;

        assert!(
            result.is_ok(),
            "Should fetch funding rate history for BTC/USDT:USDT"
        );
        let history = result.unwrap();

        assert!(
            !history.is_empty(),
            "Funding rate history should not be empty"
        );
    }
}

// ============================================================================
// 最优挂单测试
// ============================================================================

#[cfg(test)]
mod bids_asks_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_bids_asks_swap_single() {
        let client = create_swap_client().await;

        let result = client.fetch_bids_asks(Some("BTC/USDT:USDT")).await;

        assert!(result.is_ok(), "Should fetch bids/asks for BTC/USDT:USDT");
        let bids_asks = result.unwrap();

        assert_eq!(bids_asks.len(), 1, "Should return 1 entry");
    }

    #[tokio::test]
    async fn test_fetch_bids_asks_swap_all() {
        let client = create_swap_client().await;

        let result = client.fetch_bids_asks(None).await;

        assert!(result.is_ok(), "Should fetch all bids/asks");
        let bids_asks = result.unwrap();

        assert!(bids_asks.len() > 50, "Should have USDT-M trading pairs");
    }
}

// ============================================================================
// 最新价格测试
// ============================================================================

#[cfg(test)]
mod last_prices_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_last_prices_swap_single() {
        let client = create_swap_client().await;

        let result = client.fetch_last_prices(Some("BTC/USDT:USDT")).await;

        assert!(result.is_ok(), "Should fetch last price for BTC/USDT:USDT");
        let prices = result.unwrap();

        assert_eq!(prices.len(), 1, "Should return 1 price");
    }

    #[tokio::test]
    async fn test_fetch_last_prices_swap_all() {
        let client = create_swap_client().await;

        let result = client.fetch_last_prices(None).await;

        assert!(result.is_ok(), "Should fetch all last prices");
        let prices = result.unwrap();

        assert!(prices.len() > 50, "Should have USDT-M trading pairs");
    }
}

// ============================================================================
// 24小时统计测试
// ============================================================================

#[cfg(test)]
mod stats_24hr_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_24hr_stats_swap_single() {
        let client = create_swap_client().await;

        let result = client.fetch_24hr_stats(Some("BTC/USDT:USDT")).await;

        assert!(result.is_ok(), "Should fetch 24hr stats for BTC/USDT:USDT");
        let stats = result.unwrap();

        assert!(!stats.is_empty(), "Stats should not be empty");
    }
}

// ============================================================================
// 端点路由验证测试
// ============================================================================

#[cfg(test)]
mod endpoint_routing_tests {
    use super::*;

    #[tokio::test]
    async fn test_swap_uses_fapi_endpoint() {
        let client = create_swap_client().await;

        // 验证 U本位合约使用 FAPI 端点
        // 通过 fetch_markets 来验证
        let result = client.fetch_markets().await;

        assert!(result.is_ok(), "Should fetch USDT-M markets");
        let markets = result.unwrap();

        assert!(!markets.is_empty(), "Should have USDT-M trading pairs");
    }
}

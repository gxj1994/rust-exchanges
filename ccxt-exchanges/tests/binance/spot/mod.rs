//! Binance 现货测试
//!
//! 测试覆盖:
//! - 市场数据 API (ticker, order_book, trades, ohlcv等)
//! - 订单创建 API (market, limit, post-only, timeInForce)
//! - 订单查询 API (fetch_order, fetch_open_orders, cancel等)

use crate::support::{create_binance, init_test};
use ccxt_core::types::common::ohlcv_request::OhlcvRequest;
use ccxt_core::types::common::ticker_params::TickerParams;
use ccxt_exchanges::binance::Binance;

/// 创建现货 Binance 客户端
async fn create_spot_client() -> Binance {
    let config = init_test();
    let client = create_binance(&config)
        .unwrap_or_else(|e| panic!("Failed to create Binance spot client: {}", e));
    client
        .load_markets(false)
        .await
        .expect("Should load markets");
    client
}

// 订单类型测试
pub mod order_types;

// 订单查询测试
pub mod order_query;

// ============================================================================
// Ticker 测试
// ============================================================================

#[cfg(test)]
mod ticker_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_ticker_spot_24hr() {
        let client = create_spot_client().await;
        let params = TickerParams::default();

        // 测试 24 小时 ticker
        let result = client.fetch_ticker("BTC/USDT", params).await;

        assert!(result.is_ok(), "Should fetch 24hr ticker for BTC/USDT");
        let ticker = result.unwrap();

        // Ticker的字段是Option类型，测试API能否成功调用
        assert!(!ticker.symbol.is_empty(), "Symbol should not be empty");
    }

    #[tokio::test]
    async fn test_fetch_ticker_spot_rolling() {
        let client = create_spot_client().await;

        // 测试 rolling window ticker
        // 注意: 币安要求windowSize必须是字符串格式,如 "4h", "30m", "2d"
        // 使用extra方法直接传入字符串格式
        let params = TickerParams::builder()
            .rolling(true)
            .extra("windowSize", "4h") // 4小时
            .build();

        let result = client.fetch_ticker("ETH/USDT", params).await;

        if let Err(ref e) = result {
            println!("Error fetching rolling ticker: {}", e);
        }
        assert!(result.is_ok(), "Should fetch rolling ticker for ETH/USDT");
        let ticker = result.unwrap();

        assert!(ticker.symbol.contains("ETH"), "Symbol should contain ETH");
    }

    #[tokio::test]
    async fn test_fetch_ticker_different_symbols() {
        let client = create_spot_client().await;
        let params = TickerParams::default();

        let symbols = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"];

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
    async fn test_fetch_order_book_spot_basic() {
        let client = create_spot_client().await;

        let result = client.fetch_order_book("BTC/USDT", None).await;

        assert!(result.is_ok(), "Should fetch order book for BTC/USDT");
        let order_book = result.unwrap();

        assert!(
            order_book.symbol.contains("BTC"),
            "Symbol should contain BTC"
        );
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
    async fn test_fetch_order_book_spot_with_limit() {
        let client = create_spot_client().await;

        let result = client.fetch_order_book("ETH/USDT", Some(10)).await;
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
    async fn test_fetch_order_book_multiple_symbols() {
        let client = create_spot_client().await;

        let symbols = vec!["BTC/USDT", "ETH/USDT", "BNB/USDT"];

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
    async fn test_fetch_market_trades_spot() {
        let client = create_spot_client().await;

        let result = client.fetch_market_trades("BTC/USDT", None).await;

        assert!(result.is_ok(), "Should fetch market trades for BTC/USDT");
        let trades = result.unwrap();

        assert!(!trades.is_empty(), "Trades should not be empty");
        assert!(trades.len() <= 500, "Should return at most 500 trades");

        let first_trade = &trades[0];
        // Trade的price和amount字段类型需要验证,先只检查API能调用成功
        assert!(first_trade.timestamp > 0, "Trade timestamp should be valid");
    }

    #[tokio::test]
    async fn test_fetch_agg_trades_spot() {
        let client = create_spot_client().await;

        let result = client.fetch_agg_trades("BTC/USDT", None, None, None).await;

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
    async fn test_fetch_ohlcv_spot_basic() {
        let client = create_spot_client().await;

        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
            .timeframe("1h")
            .limit(10)
            .build()
            .expect("Failed to build OHLCV request");

        let result = client.fetch_ohlcv(request).await;

        assert!(result.is_ok(), "Should fetch OHLCV data for BTC/USDT");
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
    async fn test_fetch_ohlcv_spot_different_timeframes() {
        let client = create_spot_client().await;

        let timeframes = vec!["1m", "5m", "15m", "1h", "4h", "1d"];

        for timeframe in timeframes {
            let request = OhlcvRequest::builder()
                .symbol("ETH/USDT")
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
    async fn test_fetch_ohlcv_spot_with_since() {
        let client = create_spot_client().await;

        let since = chrono::Utc::now().timestamp_millis() - (24 * 60 * 60 * 1000); // 24小时前

        let request = OhlcvRequest::builder()
            .symbol("BTC/USDT")
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
// 最优挂单测试
// ============================================================================

#[cfg(test)]
mod bids_asks_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_bids_asks_spot_single() {
        let client = create_spot_client().await;

        let result = client.fetch_bids_asks(Some("BTC/USDT")).await;

        assert!(result.is_ok(), "Should fetch bids/asks for BTC/USDT");
        let bids_asks = result.unwrap();

        assert_eq!(bids_asks.len(), 1, "Should return 1 entry");
        // BidAsk的bid_price和ask_price是Decimal类型,测试API能成功调用即可
    }

    #[tokio::test]
    async fn test_fetch_bids_asks_spot_all() {
        let client = create_spot_client().await;

        let result = client.fetch_bids_asks(None).await;

        assert!(result.is_ok(), "Should fetch all bids/asks");
        let bids_asks = result.unwrap();

        assert!(
            bids_asks.len() > 100,
            "Should have more than 100 trading pairs"
        );
    }
}

// ============================================================================
// 最新价格测试
// ============================================================================

#[cfg(test)]
mod last_prices_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_last_prices_spot_single() {
        let client = create_spot_client().await;

        let result = client.fetch_last_prices(Some("BTC/USDT")).await;

        assert!(result.is_ok(), "Should fetch last price for BTC/USDT");
        let prices = result.unwrap();

        assert_eq!(prices.len(), 1, "Should return 1 price");
        // LastPrice的price字段类型验证,测试API能成功调用即可
    }

    #[tokio::test]
    async fn test_fetch_last_prices_spot_all() {
        let client = create_spot_client().await;

        let result = client.fetch_last_prices(None).await;

        assert!(result.is_ok(), "Should fetch all last prices");
        let prices = result.unwrap();

        assert!(
            prices.len() > 100,
            "Should have more than 100 trading pairs"
        );
    }
}

// ============================================================================
// 24小时统计测试
// ============================================================================

#[cfg(test)]
mod stats_24hr_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_24hr_stats_spot_single() {
        let client = create_spot_client().await;

        let result = client.fetch_24hr_stats(Some("BTC/USDT")).await;

        assert!(result.is_ok(), "Should fetch 24hr stats for BTC/USDT");
        let stats = result.unwrap();

        assert!(!stats.is_empty(), "Stats should not be empty");
    }
}

// ============================================================================
// 交易限制测试
// ============================================================================

#[cfg(test)]
mod trading_limits_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_trading_limits_spot() {
        let client = create_spot_client().await;

        let result = client.fetch_trading_limits("BTC/USDT").await;

        assert!(result.is_ok(), "Should fetch trading limits for BTC/USDT");
        let limits = result.unwrap();

        // TradingLimits的amount字段是Option<MinMax>
        assert!(limits.amount.is_some(), "Should have amount limits");
    }
}

//! WebSocket helper functions for exchange implementations.
//!
//! This module provides reusable helpers to reduce duplication across
//! WebSocket implementations:
//! - Timeframe conversion utilities
//! - Channel name parsing
//! - Stream merging utilities

use ccxt_core::{
    error::{Error, Result},
    types::Timeframe,
    ws::SubscriptionChannel,
};

/// Convert unified Timeframe to uppercase interval format.
///
/// Used by exchanges that use uppercase for hour/day/week/month:
/// - OKX: "1H", "4H", "1D", "1W", "1M"
/// - Bitget: "1H", "4H", "1D", "1W", "1M"
/// - Minutes stay lowercase: "1m", "5m", "15m", "30m"
///
/// # Example
/// ```
/// use ccxt_core::types::Timeframe;
/// use ccxt_exchanges::common::timeframe_to_uppercase_interval;
/// let interval = timeframe_to_uppercase_interval(Timeframe::H1);
/// assert_eq!(interval, "1H");
/// ```
pub fn timeframe_to_uppercase_interval(timeframe: Timeframe) -> String {
    let s = timeframe.to_string();
    let mut chars = s.chars().collect::<Vec<_>>();
    if chars.len() >= 2 {
        let last = chars.last_mut().unwrap();
        match *last {
            'h' => *last = 'H',
            'd' => *last = 'D',
            'w' => *last = 'W',
            _ => {}
        }
    }
    chars.into_iter().collect()
}

/// Convert unified Timeframe to Bybit interval format.
///
/// Bybit uses:
/// - "1", "3", "5", "15", "30" for minutes
/// - "60", "120", "240", "360", "720" for hours (in minutes)
/// - "D", "W", "M" for day/week/month
pub fn timeframe_to_bybit_interval(timeframe: Timeframe) -> String {
    match timeframe {
        Timeframe::S1 => "1s".to_string(),
        Timeframe::M1 => "1".to_string(),
        Timeframe::M3 => "3".to_string(),
        Timeframe::M5 => "5".to_string(),
        Timeframe::M15 => "15".to_string(),
        Timeframe::M30 => "30".to_string(),
        Timeframe::H1 => "60".to_string(),
        Timeframe::H2 => "120".to_string(),
        Timeframe::H4 => "240".to_string(),
        Timeframe::H6 => "360".to_string(),
        Timeframe::H12 => "720".to_string(),
        Timeframe::D1 => "D".to_string(),
        Timeframe::W1 => "W".to_string(),
        Timeframe::Mon1 => "M".to_string(),
        // Fallback for unsupported timeframes
        Timeframe::H8 => "480".to_string(), // 8 hours = 480 minutes
        Timeframe::D3 => "D".to_string(),   // No 3-day support, use 1-day
    }
}

/// Convert channel name string to ChannelType.
///
/// Supports common channel name variations:
/// - "ticker" / "tickers" -> Ticker
/// - "orderbook" / "book" -> OrderBook
/// - "trades" / "trade" -> Trades
/// - "kline" / "ohlcv" / "candle" -> Kline
/// - "balance" / "account" -> Balance
/// - "orders" / "order" -> Orders
/// - "account_trades" / "mytrades" -> MyTrades
/// - Others -> Custom
pub fn channel_name_to_type(channel: &str) -> ccxt_core::ws::ChannelType {
    match channel.to_lowercase().as_str() {
        "ticker" | "tickers" => ccxt_core::ws::ChannelType::Ticker,
        "orderbook" | "book" => ccxt_core::ws::ChannelType::OrderBook,
        "trades" | "trade" => ccxt_core::ws::ChannelType::Trades,
        "kline" | "ohlcv" | "candle" => ccxt_core::ws::ChannelType::Kline,
        "balance" | "account" => ccxt_core::ws::ChannelType::Balance,
        "orders" | "order" => ccxt_core::ws::ChannelType::Orders,
        "account_trades" | "mytrades" => ccxt_core::ws::ChannelType::MyTrades,
        _ => ccxt_core::ws::ChannelType::Custom,
    }
}

/// Build a SubscriptionChannel from channel name and optional symbol.
///
/// This helper function is used by `subscribe` and `unsubscribe` methods
/// to convert string channel names to typed SubscriptionChannel.
///
/// # Errors
///
/// Returns an error if:
/// - A symbol is required but not provided (for ticker, orderbook, trades)
/// - An unsupported channel type is requested
/// - Kline channel is requested (use watch_ohlcv instead)
pub fn build_subscription_channel(
    channel: &str,
    symbol: Option<&str>,
) -> Result<SubscriptionChannel> {
    let channel_type = channel_name_to_type(channel);
    match channel_type {
        ccxt_core::ws::ChannelType::Ticker => {
            Ok(SubscriptionChannel::ticker(symbol.ok_or_else(|| {
                Error::invalid_request("symbol is required for ticker channel")
            })?))
        }
        ccxt_core::ws::ChannelType::OrderBook => {
            Ok(SubscriptionChannel::orderbook(symbol.ok_or_else(|| {
                Error::invalid_request("symbol is required for orderbook channel")
            })?))
        }
        ccxt_core::ws::ChannelType::Trades => {
            Ok(SubscriptionChannel::trades(symbol.ok_or_else(|| {
                Error::invalid_request("symbol is required for trades channel")
            })?))
        }
        ccxt_core::ws::ChannelType::Kline => Err(Error::invalid_request(
            "use watch_ohlcv for kline subscription with timeframe parameter",
        )),
        ccxt_core::ws::ChannelType::Balance => Ok(SubscriptionChannel::balance()),
        ccxt_core::ws::ChannelType::Orders => {
            Ok(SubscriptionChannel::orders(symbol.map(ToString::to_string)))
        }
        ccxt_core::ws::ChannelType::MyTrades => {
            Ok(SubscriptionChannel::orders(symbol.map(ToString::to_string)))
        }
        _ => Err(Error::invalid_request(format!(
            "Unknown or unsupported channel: {}",
            channel
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeframe_to_uppercase_interval() {
        assert_eq!(timeframe_to_uppercase_interval(Timeframe::M1), "1m");
        assert_eq!(timeframe_to_uppercase_interval(Timeframe::H1), "1H");
        assert_eq!(timeframe_to_uppercase_interval(Timeframe::D1), "1D");
        assert_eq!(timeframe_to_uppercase_interval(Timeframe::W1), "1W");
        assert_eq!(timeframe_to_uppercase_interval(Timeframe::Mon1), "1M");
    }

    #[test]
    fn test_timeframe_to_bybit_interval() {
        assert_eq!(timeframe_to_bybit_interval(Timeframe::M1), "1");
        assert_eq!(timeframe_to_bybit_interval(Timeframe::M5), "5");
        assert_eq!(timeframe_to_bybit_interval(Timeframe::H1), "60");
        assert_eq!(timeframe_to_bybit_interval(Timeframe::D1), "D");
        assert_eq!(timeframe_to_bybit_interval(Timeframe::W1), "W");
    }

    #[test]
    fn test_channel_name_to_type() {
        assert_eq!(
            channel_name_to_type("ticker"),
            ccxt_core::ws::ChannelType::Ticker
        );
        assert_eq!(
            channel_name_to_type("TICKERS"),
            ccxt_core::ws::ChannelType::Ticker
        );
        assert_eq!(
            channel_name_to_type("orderbook"),
            ccxt_core::ws::ChannelType::OrderBook
        );
        assert_eq!(
            channel_name_to_type("book"),
            ccxt_core::ws::ChannelType::OrderBook
        );
        assert_eq!(
            channel_name_to_type("trades"),
            ccxt_core::ws::ChannelType::Trades
        );
        assert_eq!(
            channel_name_to_type("balance"),
            ccxt_core::ws::ChannelType::Balance
        );
        assert_eq!(
            channel_name_to_type("orders"),
            ccxt_core::ws::ChannelType::Orders
        );
        assert_eq!(
            channel_name_to_type("account_trades"),
            ccxt_core::ws::ChannelType::MyTrades
        );
        assert_eq!(
            channel_name_to_type("custom_channel"),
            ccxt_core::ws::ChannelType::Custom
        );
    }

    #[test]
    fn test_build_subscription_channel_ticker() {
        let channel = build_subscription_channel("ticker", Some("BTC/USDT")).unwrap();
        assert_eq!(channel.channel_type, ccxt_core::ws::ChannelType::Ticker);
        assert_eq!(channel.symbol, "BTC/USDT");
    }

    #[test]
    fn test_build_subscription_channel_balance() {
        let channel = build_subscription_channel("balance", None).unwrap();
        assert_eq!(channel.channel_type, ccxt_core::ws::ChannelType::Balance);
        assert!(channel.is_private());
    }

    #[test]
    fn test_build_subscription_channel_missing_symbol() {
        let result = build_subscription_channel("ticker", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_subscription_channel_kline() {
        let result = build_subscription_channel("kline", Some("BTC/USDT"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("watch_ohlcv"));
    }
}

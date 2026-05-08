//! Core type definitions for CCXT
//!
//! This module contains all the fundamental data structures used throughout the library,
//! including markets, orders, trades, balances, tickers, and order books.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

// Re-export submodules by category
pub mod market;
pub use market::funding_rate;
pub mod account;
pub mod common;
pub mod financial;
pub mod order;
pub mod trading;

// Re-export commonly used types (following mod_old.rs exact order and aliases)
pub use account::balance::{Balance, BalanceEntry, MaxBorrowable, MaxTransferable};
pub use account_config::{AccountConfig, CommissionRate};
pub use common::EndpointType;
pub use common::{Currency, CurrencyNetwork, MinMax, PrecisionMode};
pub use common::{DefaultSubType, DefaultType, DefaultTypeError, resolve_market_type};
pub use order::bid_ask::BidAsk;
// Handle naming conflicts with aliases (from trading/fee.rs)
pub use trading::fee::{
    FundingRate as FeeFundingRate, FundingRateHistory as FeeFundingRateHistory, LeverageTier,
    TradingFee as FeeTradingFee,
};
// Funding rate types (from market/funding_rate.rs)
pub use account::ledger::{LedgerDirection, LedgerEntry, LedgerEntryType};
pub use funding_rate::{
    FundingFee, FundingHistory, FundingRate, FundingRateHistory, NextFundingRate,
};
pub use market::mark_price::MarkPrice;
pub use trading::margin::{
    BorrowInterest, BorrowRate, BorrowRateHistory, IsolatedBorrowRate, MarginAdjustment,
    MarginLoan, MarginRepay,
};
// Market types (note: BidAsk has alias to avoid conflict with order::bid_ask::BidAsk)
pub use account::{
    DepositAddress, Transaction, TransactionFee, TransactionStatus, TransactionType,
};
pub use account::{DepositWithdrawFee, NetworkInfo, Transfer, TransferType};
pub use account::{Leverage, MarginType, Position, PositionSide};
pub use common::{ContractType, ExpiryDate, ParsedSymbol, SymbolMarketType};
pub use common::{IntoTickerParams, TickerParams, TickerParamsBuilder};
pub use common::{OhlcvRequest, OhlcvRequestBuilder};
pub use market::market::{
    BidAsk as MarketBidAsk, LastPrice, Market, MarketLimits, MarketPrecision, MarketType,
    ServerTime, Stats24hr, TradingFee,
};
pub use market::market_data::{IndexPrice, Liquidation, PremiumIndex};
pub use market::ohlcv::OHLCV;
pub use market::orderbook::{OrderBook, OrderBookDelta, OrderBookEntry, OrderBookSide};
pub use market::ticker::Ticker;
pub use market::trade::{AggTrade, TakerOrMaker, Trade};
pub use order::{AmountSpec, OrderRequest, OrderRequestBuilder};
pub use order::{
    BatchCancelResult, BatchOrderRequest, BatchOrderResult, BatchOrderUpdate,
    CancelAllOrdersResult, CancelReplaceResponse, OcoOrder, OcoOrderInfo, Order, OrderReport,
    OrderSide, OrderStatus, OrderType, TimeInForce,
};
pub use trading::{
    AccountType, BalanceParams, LeverageParams, MarginMode, OhlcvParams, OrderBookParams,
    PriceType, TransferParams, WithdrawParams,
};
pub use trading::{MaxLeverage, OpenInterest, OpenInterestHistory};

// Types that remain in root for backward compatibility
pub mod account_config;

/// Type alias for timestamps (milliseconds since Unix epoch)
pub type Timestamp = i64;

/// Trading symbol with type safety.
///
/// Uses the newtype pattern to provide compile-time distinction from regular strings.
/// Represents trading pair symbols in the format "BASE/QUOTE" (e.g., "BTC/USDT").
///
/// # Backward Compatibility
///
/// This type is designed for gradual migration. It implements `Deref<Target=str>`
/// and `AsRef<str>` to maintain compatibility with existing code that expects `&str`.
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::Symbol;
///
/// let symbol = Symbol::new("BTC/USDT").unwrap();
/// assert_eq!(symbol.as_str(), "BTC/USDT");
///
/// // Backward compatible: can use as &str
/// fn takes_str(s: &str) -> bool { s.contains("BTC") }
/// assert!(takes_str(&symbol));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Symbol(String);

impl Symbol {
    /// Creates an empty Symbol (for Default implementation)
    #[allow(dead_code)]
    fn empty() -> Self {
        Self(String::new())
    }
}

impl Symbol {
    /// Creates a new Symbol after validating the format.
    ///
    /// # Validation Rules
    ///
    /// - Must contain exactly one '/' separator
    /// - Base and quote parts must not be empty
    ///
    /// # Errors
    ///
    /// Returns `None` if the format is invalid.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::Symbol;
    ///
    /// assert!(Symbol::new("BTC/USDT").is_some());
    /// assert!(Symbol::new("INVALID").is_none());  // Missing separator
    /// assert!(Symbol::new("/USDT").is_none());    // Empty base
    /// assert!(Symbol::new("BTC/").is_none());     // Empty quote
    /// ```
    #[must_use]
    pub fn new(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        if parts[0].is_empty() || parts[1].is_empty() {
            return None;
        }
        Some(Self(s.to_string()))
    }

    /// Creates a new Symbol without validation.
    ///
    /// # Safety
    ///
    /// Caller must ensure the symbol is in valid format. Use [`Symbol::new`] for
    /// validated construction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::Symbol;
    ///
    /// let symbol = Symbol::new_unchecked("BTC/USDT");
    /// assert_eq!(symbol.as_str(), "BTC/USDT");
    /// ```
    pub fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the symbol as a string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the base currency (left side of '/').
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::Symbol;
    ///
    /// let symbol = Symbol::new("BTC/USDT").unwrap();
    /// assert_eq!(symbol.base(), "BTC");
    /// ```
    #[must_use]
    pub fn base(&self) -> &str {
        self.0.split('/').next().unwrap_or(&self.0)
    }

    /// Returns the quote currency (right side of '/').
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::Symbol;
    ///
    /// let symbol = Symbol::new("BTC/USDT").unwrap();
    /// assert_eq!(symbol.quote(), "USDT");
    /// ```
    #[must_use]
    pub fn quote(&self) -> &str {
        self.0.split('/').nth(1).unwrap_or("")
    }
}

impl std::ops::Deref for Symbol {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Symbol> for String {
    fn from(symbol: Symbol) -> Self {
        symbol.0
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Re-export financial types with enhanced type safety
pub use financial::{Amount, Cost, Price};

/// Fee information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fee {
    /// Fee currency code
    pub currency: String,

    /// Fee cost
    pub cost: Decimal,

    /// Fee rate (if available)
    pub rate: Option<Decimal>,
}

impl Fee {
    /// Create a new fee
    #[must_use]
    pub fn new(currency: String, cost: Decimal) -> Self {
        Self {
            currency,
            cost,
            rate: None,
        }
    }

    /// Create a new fee with rate
    #[must_use]
    pub fn with_rate(currency: String, cost: Decimal, rate: Decimal) -> Self {
        Self {
            currency,
            cost,
            rate: Some(rate),
        }
    }
}

/// OHLCV (Open, High, Low, Close, Volume) candlestick data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ohlcv {
    /// Timestamp in milliseconds
    pub timestamp: Timestamp,

    /// Opening price
    pub open: Price,

    /// Highest price
    pub high: Price,

    /// Lowest price
    pub low: Price,

    /// Closing price
    pub close: Price,

    /// Volume traded
    pub volume: Amount,
}

impl Ohlcv {
    /// Create a new OHLCV entry
    #[must_use]
    pub fn new(
        timestamp: Timestamp,
        open: Price,
        high: Price,
        low: Price,
        close: Price,
        volume: Amount,
    ) -> Self {
        Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    /// Convert to array format [timestamp, open, high, low, close, volume]
    #[must_use]
    pub fn to_array(&self) -> [Decimal; 6] {
        [
            Decimal::from(self.timestamp),
            self.open.as_decimal(),
            self.high.as_decimal(),
            self.low.as_decimal(),
            self.close.as_decimal(),
            self.volume.as_decimal(),
        ]
    }

    /// Create from array format [timestamp, open, high, low, close, volume]
    #[must_use]
    pub fn from_array(arr: [Decimal; 6]) -> Self {
        Self {
            timestamp: arr[0].to_i64().unwrap_or(0),
            open: Price::new(arr[1]),
            high: Price::new(arr[2]),
            low: Price::new(arr[3]),
            close: Price::new(arr[4]),
            volume: Amount::new(arr[5]),
        }
    }
}

/// Timeframe enum for OHLCV data
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum Timeframe {
    /// 1 second
    #[serde(rename = "1s")]
    S1,
    /// 1 minute
    #[serde(rename = "1m")]
    M1,
    /// 3 minutes
    #[serde(rename = "3m")]
    M3,
    /// 5 minutes
    #[serde(rename = "5m")]
    M5,
    /// 15 minutes
    #[serde(rename = "15m")]
    M15,
    /// 30 minutes
    #[serde(rename = "30m")]
    M30,
    /// 1 hour
    #[serde(rename = "1h")]
    #[default]
    H1,
    /// 2 hours
    #[serde(rename = "2h")]
    H2,
    /// 4 hours
    #[serde(rename = "4h")]
    H4,
    /// 6 hours
    #[serde(rename = "6h")]
    H6,
    /// 8 hours
    #[serde(rename = "8h")]
    H8,
    /// 12 hours
    #[serde(rename = "12h")]
    H12,
    /// 1 day
    #[serde(rename = "1d")]
    D1,
    /// 3 days
    #[serde(rename = "3d")]
    D3,
    /// 1 week
    #[serde(rename = "1w")]
    W1,
    /// 1 month
    // REVIEW NOTE: Mon1命名不够直观，容易与Monday混淆
    // 建议改为 Month1 或 M1Month
    #[serde(rename = "1M")]
    Mon1,
}

impl Timeframe {
    /// Convert timeframe to milliseconds
    #[must_use]
    pub fn as_millis(&self) -> i64 {
        match self {
            Self::S1 => 1_000,
            Self::M1 => 60_000,
            Self::M3 => 180_000,
            Self::M5 => 300_000,
            Self::M15 => 900_000,
            Self::M30 => 1_800_000,
            Self::H1 => 3_600_000,
            Self::H2 => 7_200_000,
            Self::H4 => 14_400_000,
            Self::H6 => 21_600_000,
            Self::H8 => 28_800_000,
            Self::H12 => 43_200_000,
            Self::D1 => 86_400_000,
            Self::D3 => 259_200_000,
            Self::W1 => 604_800_000,
            Self::Mon1 => 2_592_000_000, // 30 days
        }
    }

    /// Convert timeframe to seconds
    #[must_use]
    pub fn as_seconds(&self) -> i64 {
        self.as_millis() / 1000
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::S1 => "1s",
            Self::M1 => "1m",
            Self::M3 => "3m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::M30 => "30m",
            Self::H1 => "1h",
            Self::H2 => "2h",
            Self::H4 => "4h",
            Self::H6 => "6h",
            Self::H8 => "8h",
            Self::H12 => "12h",
            Self::D1 => "1d",
            Self::D3 => "3d",
            Self::W1 => "1w",
            Self::Mon1 => "1M",
        };
        write!(f, "{s}")
    }
}

/// Trading limits for a market
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradingLimits {
    /// Minimum amount (deprecated, use amount.min)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<Amount>,

    /// Maximum amount (deprecated, use amount.max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<Amount>,

    /// Amount/quantity limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<MinMax>,

    /// Price limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<MinMax>,

    /// Cost (amount * price) limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<MinMax>,
}

impl TradingLimits {
    /// Create new trading limits (legacy)
    #[must_use]
    pub fn new(min: Option<Amount>, max: Option<Amount>) -> Self {
        Self {
            min,
            max,
            amount: None,
            price: None,
            cost: None,
        }
    }

    /// Create new trading limits with detailed constraints
    #[must_use]
    pub fn new_detailed(
        amount: Option<MinMax>,
        price: Option<MinMax>,
        cost: Option<MinMax>,
    ) -> Self {
        Self {
            min: None,
            max: None,
            amount,
            price,
            cost,
        }
    }

    /// Check if amount is within limits
    #[must_use]
    pub fn is_valid(&self, amount: Amount) -> bool {
        let min_ok = self.min.is_none_or(|min| amount >= min);
        let max_ok = self.max.is_none_or(|max| amount <= max);
        min_ok && max_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_fee_creation() {
        let fee = Fee::new("USDT".to_string(), dec!(10.5));
        assert_eq!(fee.currency, "USDT");
        assert_eq!(fee.cost, dec!(10.5));
        assert_eq!(fee.rate, None);

        let fee_with_rate = Fee::with_rate("USDT".to_string(), dec!(10.5), dec!(0.001));
        assert_eq!(fee_with_rate.rate, Some(dec!(0.001)));
    }

    #[test]
    fn test_ohlcv_conversion() {
        let ohlcv = Ohlcv::new(
            1234567890,
            Price::from(dec!(100)),
            Price::from(dec!(110)),
            Price::from(dec!(95)),
            Price::from(dec!(105)),
            Amount::from(dec!(1000)),
        );

        let arr = ohlcv.to_array();
        assert_eq!(arr[0], Decimal::from(1234567890));
        assert_eq!(arr[1], dec!(100));
        assert_eq!(arr[5], dec!(1000));
    }

    #[test]
    fn test_timeframe_conversion() {
        assert_eq!(Timeframe::M1.as_millis(), 60_000);
        assert_eq!(Timeframe::H1.as_millis(), 3_600_000);
        assert_eq!(Timeframe::D1.as_millis(), 86_400_000);

        assert_eq!(Timeframe::M1.as_seconds(), 60);
        assert_eq!(Timeframe::H1.as_seconds(), 3_600);
    }

    #[test]
    fn test_trading_limits() {
        let limits = TradingLimits::new(
            Some(Amount::from(dec!(0.01))),
            Some(Amount::from(dec!(100))),
        );

        assert!(limits.is_valid(Amount::from(dec!(1.0))));
        assert!(limits.is_valid(Amount::from(dec!(0.01))));
        assert!(limits.is_valid(Amount::from(dec!(100))));
        assert!(!limits.is_valid(Amount::from(dec!(0.001))));
        assert!(!limits.is_valid(Amount::from(dec!(101))));
    }
}

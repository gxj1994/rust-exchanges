//! Parameter types for trait methods with builder pattern support.
//!
//! This module provides ergonomic parameter structs for the decomposed Exchange traits.
//! Each parameter type uses the builder pattern for flexible and readable API calls.
//!
//! # Example
//!
//! ```rust
//! use ccxt_core::types::trading::params::{OhlcvParams, AccountType};
//! use ccxt_core::types::{Timeframe, OrderSide};
//! use rust_decimal_macros::dec;
//!
//! // OHLCV parameters with builder pattern
//! let params = OhlcvParams::new(Timeframe::H1)
//!     .since(1609459200000)
//!     .limit(100);
//!
//! ```

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::types::account::position::MarginType;

/// Account type for balance queries and transfers.
///
/// Represents different account types available on exchanges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    /// Spot trading account.
    #[default]
    Spot,
    /// Cross margin account.
    Margin,
    /// Isolated margin account.
    IsolatedMargin,
    /// USDT-margined futures account.
    Futures,
    /// Coin-margined futures account.
    Delivery,
    /// Funding/wallet account.
    Funding,
    /// Options trading account.
    Option,
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AccountType::Spot => "spot",
            AccountType::Margin => "margin",
            AccountType::IsolatedMargin => "isolated_margin",
            AccountType::Futures => "futures",
            AccountType::Delivery => "delivery",
            AccountType::Funding => "funding",
            AccountType::Option => "option",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for AccountType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "spot" | "main" => Ok(AccountType::Spot),
            "margin" | "cross" => Ok(AccountType::Margin),
            "isolated_margin" | "isolated" => Ok(AccountType::IsolatedMargin),
            "futures" | "future" | "swap" | "linear" | "umfuture" => Ok(AccountType::Futures),
            "delivery" | "inverse" | "cmfuture" => Ok(AccountType::Delivery),
            "funding" | "wallet" => Ok(AccountType::Funding),
            "option" | "options" => Ok(AccountType::Option),
            _ => Err(format!("Invalid account type: {s}")),
        }
    }
}

/// Margin mode for futures trading.
///
/// Type alias for `MarginType` to provide a more intuitive name in the context
/// of the new trait hierarchy.
pub type MarginMode = MarginType;

/// Price type for OHLCV data (futures-specific).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PriceType {
    /// Contract/last price (default).
    #[default]
    Contract,
    /// Mark price.
    Mark,
    /// Index price.
    Index,
    /// Premium index.
    PremiumIndex,
}

impl std::fmt::Display for PriceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PriceType::Contract => "contract",
            PriceType::Mark => "mark",
            PriceType::Index => "index",
            PriceType::PremiumIndex => "premiumIndex",
        };
        write!(f, "{s}")
    }
}

// ============================================================================
// OHLCV Parameters
// ============================================================================

/// Parameters for fetching OHLCV (candlestick) data.
///
/// Uses the builder pattern for ergonomic parameter construction.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::trading::params::{OhlcvParams, PriceType};
/// use ccxt_core::types::Timeframe;
///
/// let params = OhlcvParams::new(Timeframe::H1)
///     .since(1609459200000)
///     .limit(100)
///     .until(1609545600000)
///     .price(PriceType::Mark);
/// ```
#[derive(Debug, Clone, Default)]
pub struct OhlcvParams {
    /// Timeframe for the candlesticks.
    pub timeframe: crate::types::Timeframe,
    /// Start timestamp in milliseconds.
    pub since: Option<i64>,
    /// Maximum number of candles to return.
    pub limit: Option<u32>,
    /// End timestamp in milliseconds.
    pub until: Option<i64>,
    /// Price type (mark, index, premiumIndex for futures).
    pub price: Option<PriceType>,
}

impl OhlcvParams {
    /// Create new OHLCV parameters with the specified timeframe.
    #[must_use]
    pub fn new(timeframe: crate::types::Timeframe) -> Self {
        Self {
            timeframe,
            ..Default::default()
        }
    }

    /// Set the start timestamp.
    #[must_use]
    pub fn since(mut self, ts: i64) -> Self {
        self.since = Some(ts);
        self
    }

    /// Set the maximum number of candles.
    #[must_use]
    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }

    /// Set the end timestamp.
    #[must_use]
    pub fn until(mut self, ts: i64) -> Self {
        self.until = Some(ts);
        self
    }

    /// Set the price type (for futures).
    #[must_use]
    pub fn price(mut self, p: PriceType) -> Self {
        self.price = Some(p);
        self
    }
}

// ============================================================================
// OrderBook Parameters
// ============================================================================

/// Parameters for fetching order book data.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::trading::params::OrderBookParams;
///
/// let params = OrderBookParams::default().limit(100);
/// ```
#[derive(Debug, Clone, Default)]
pub struct OrderBookParams {
    /// Maximum depth (number of price levels) to return.
    pub limit: Option<u32>,
}

impl OrderBookParams {
    /// Set the maximum depth.
    #[must_use]
    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }
}

// ============================================================================
// Balance Parameters
// ============================================================================

/// Parameters for fetching account balance.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::trading::params::{BalanceParams, AccountType};
///
/// // Spot balance
/// let params = BalanceParams::spot();
///
/// // Futures balance for specific currencies
/// let params = BalanceParams::futures()
///     .currencies(&["BTC", "USDT"]);
/// ```
#[derive(Debug, Clone, Default)]
pub struct BalanceParams {
    /// Account type to query.
    pub account_type: Option<AccountType>,
    /// Filter by specific currencies.
    pub currencies: Option<Vec<String>>,
}

impl BalanceParams {
    /// Create parameters for spot account balance.
    #[must_use]
    pub fn spot() -> Self {
        Self {
            account_type: Some(AccountType::Spot),
            currencies: None,
        }
    }

    /// Create parameters for margin account balance.
    #[must_use]
    pub fn margin() -> Self {
        Self {
            account_type: Some(AccountType::Margin),
            currencies: None,
        }
    }

    /// Create parameters for futures account balance.
    #[must_use]
    pub fn futures() -> Self {
        Self {
            account_type: Some(AccountType::Futures),
            currencies: None,
        }
    }

    /// Filter by specific currencies.
    pub fn currencies(mut self, codes: &[&str]) -> Self {
        self.currencies = Some(codes.iter().map(std::string::ToString::to_string).collect());
        self
    }
}

// ============================================================================
// Leverage Parameters
// ============================================================================

/// Parameters for setting leverage.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::trading::params::LeverageParams;
/// use ccxt_core::types::account::position::MarginType;
///
/// let params = LeverageParams::new("BTC/USDT:USDT", 10)
///     .margin_mode(MarginType::Isolated);
/// ```
#[derive(Debug, Clone)]
pub struct LeverageParams {
    /// Trading symbol.
    pub symbol: String,
    /// Leverage multiplier.
    pub leverage: u32,
    /// Margin mode (cross or isolated).
    pub margin_mode: Option<MarginMode>,
}

impl LeverageParams {
    /// Create new leverage parameters.
    #[must_use]
    pub fn new(symbol: &str, leverage: u32) -> Self {
        Self {
            symbol: symbol.to_string(),
            leverage,
            margin_mode: None,
        }
    }

    /// Set margin mode to cross.
    #[must_use]
    pub fn cross(mut self) -> Self {
        self.margin_mode = Some(MarginMode::Cross);
        self
    }

    /// Set margin mode to isolated.
    #[must_use]
    pub fn isolated(mut self) -> Self {
        self.margin_mode = Some(MarginMode::Isolated);
        self
    }

    /// Set the margin mode explicitly.
    #[must_use]
    pub fn margin_mode(mut self, mode: MarginMode) -> Self {
        self.margin_mode = Some(mode);
        self
    }
}

// ============================================================================
// Withdraw Parameters
// ============================================================================

/// Parameters for withdrawing funds.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::trading::params::WithdrawParams;
/// use rust_decimal_macros::dec;
///
/// let params = WithdrawParams::new("USDT", dec!(100), "TAddress...")
///     .network("TRC20")
///     .tag("memo123");
/// ```
#[derive(Debug, Clone)]
pub struct WithdrawParams {
    /// Currency code to withdraw.
    pub currency: String,
    /// Amount to withdraw.
    pub amount: Decimal,
    /// Destination address.
    pub address: String,
    /// Address tag/memo (for certain chains).
    pub tag: Option<String>,
    /// Specific network (e.g., "ERC20", "TRC20").
    pub network: Option<String>,
    /// Custom withdrawal fee.
    pub fee: Option<Decimal>,
}

impl WithdrawParams {
    /// Create new withdrawal parameters.
    #[must_use]
    pub fn new(currency: &str, amount: Decimal, address: &str) -> Self {
        Self {
            currency: currency.to_string(),
            amount,
            address: address.to_string(),
            tag: None,
            network: None,
            fee: None,
        }
    }

    /// Set the address tag/memo.
    #[must_use]
    pub fn tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    /// Set the network.
    #[must_use]
    pub fn network(mut self, network: &str) -> Self {
        self.network = Some(network.to_string());
        self
    }

    /// Set a custom fee.
    #[must_use]
    pub fn fee(mut self, fee: Decimal) -> Self {
        self.fee = Some(fee);
        self
    }
}

// ============================================================================
// Transfer Parameters
// ============================================================================

/// Parameters for transferring funds between accounts.
///
/// # Example
///
/// ```rust
/// use ccxt_core::types::trading::params::{TransferParams, AccountType};
/// use rust_decimal_macros::dec;
///
/// // Transfer from spot to futures
/// let params = TransferParams::spot_to_futures("USDT", dec!(1000));
///
/// // Custom transfer
/// let params = TransferParams::new("BTC", dec!(0.1), AccountType::Spot, AccountType::Margin);
/// ```
#[derive(Debug, Clone)]
pub struct TransferParams {
    /// Currency code to transfer.
    pub currency: String,
    /// Amount to transfer.
    pub amount: Decimal,
    /// Source account type.
    pub from_account: AccountType,
    /// Destination account type.
    pub to_account: AccountType,
}

impl TransferParams {
    /// Create new transfer parameters.
    #[must_use]
    pub fn new(currency: &str, amount: Decimal, from: AccountType, to: AccountType) -> Self {
        Self {
            currency: currency.to_string(),
            amount,
            from_account: from,
            to_account: to,
        }
    }

    /// Transfer from spot to futures account.
    #[must_use]
    pub fn spot_to_futures(currency: &str, amount: Decimal) -> Self {
        Self::new(currency, amount, AccountType::Spot, AccountType::Futures)
    }

    /// Transfer from futures to spot account.
    #[must_use]
    pub fn futures_to_spot(currency: &str, amount: Decimal) -> Self {
        Self::new(currency, amount, AccountType::Futures, AccountType::Spot)
    }

    /// Transfer from spot to margin account.
    #[must_use]
    pub fn spot_to_margin(currency: &str, amount: Decimal) -> Self {
        Self::new(currency, amount, AccountType::Spot, AccountType::Margin)
    }

    /// Transfer from margin to spot account.
    #[must_use]
    pub fn margin_to_spot(currency: &str, amount: Decimal) -> Self {
        Self::new(currency, amount, AccountType::Margin, AccountType::Spot)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Timeframe;
    use rust_decimal_macros::dec;

    #[test]
    fn test_account_type_display() {
        assert_eq!(AccountType::Spot.to_string(), "spot");
        assert_eq!(AccountType::Futures.to_string(), "futures");
        assert_eq!(AccountType::Margin.to_string(), "margin");
    }

    #[test]
    fn test_account_type_from_str() {
        assert_eq!("spot".parse::<AccountType>().unwrap(), AccountType::Spot);
        assert_eq!(
            "futures".parse::<AccountType>().unwrap(),
            AccountType::Futures
        );
        assert_eq!(
            "margin".parse::<AccountType>().unwrap(),
            AccountType::Margin
        );
        assert_eq!("main".parse::<AccountType>().unwrap(), AccountType::Spot);
    }

    #[test]
    fn test_ohlcv_params_builder() {
        let params = OhlcvParams::new(Timeframe::H1)
            .since(1609459200000)
            .limit(100)
            .until(1609545600000)
            .price(PriceType::Mark);

        assert_eq!(params.timeframe, Timeframe::H1);
        assert_eq!(params.since, Some(1609459200000));
        assert_eq!(params.limit, Some(100));
        assert_eq!(params.until, Some(1609545600000));
        assert_eq!(params.price, Some(PriceType::Mark));
    }

    #[test]
    fn test_order_book_params() {
        let params = OrderBookParams::default().limit(50);
        assert_eq!(params.limit, Some(50));
    }

    #[test]
    fn test_balance_params() {
        let params = BalanceParams::futures().currencies(&["BTC", "USDT"]);
        assert_eq!(params.account_type, Some(AccountType::Futures));
        assert_eq!(
            params.currencies,
            Some(vec!["BTC".to_string(), "USDT".to_string()])
        );
    }

    #[test]
    fn test_leverage_params() {
        let params = LeverageParams::new("BTC/USDT:USDT", 10).isolated();
        assert_eq!(params.symbol, "BTC/USDT:USDT");
        assert_eq!(params.leverage, 10);
        assert_eq!(params.margin_mode, Some(MarginMode::Isolated));
    }

    #[test]
    fn test_withdraw_params() {
        let params = WithdrawParams::new("USDT", dec!(100), "TAddress123")
            .network("TRC20")
            .tag("memo");

        assert_eq!(params.currency, "USDT");
        assert_eq!(params.amount, dec!(100));
        assert_eq!(params.address, "TAddress123");
        assert_eq!(params.network, Some("TRC20".to_string()));
        assert_eq!(params.tag, Some("memo".to_string()));
    }

    #[test]
    fn test_transfer_params() {
        let params = TransferParams::spot_to_futures("USDT", dec!(1000));
        assert_eq!(params.currency, "USDT");
        assert_eq!(params.amount, dec!(1000));
        assert_eq!(params.from_account, AccountType::Spot);
        assert_eq!(params.to_account, AccountType::Futures);
    }
}

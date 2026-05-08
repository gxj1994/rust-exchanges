//! Order request types with Builder pattern using typestate.
//!
//! This module provides a type-safe way to construct order requests using the
//! typestate pattern, which ensures at compile-time that all required fields
//! are set before building an order request.
//!
//! # Design Philosophy
//!
//! Fields are organized into logical groups and merged to eliminate redundancy
//! across different exchange APIs:
//!
//! 1. **Core Required** (4): symbol, side, order_type, amount
//! 2. **Common Optional** (5): price, time_in_force, client_order_id, reduce_only, post_only
//! 3. **Conditional Orders** (2): stop_price, trigger_price_type (merged from 6 legacy fields)
//! 4. **Trailing Stop** (3): trailing_callback_rate, trailing_amount_type, activation_price
//! 5. **Futures** (1): position_side
//! 6. **TP/SL Limit** (2): tp_limit_price, sl_limit_price
//!
//! # Examples
//!
//! ```rust
//! use ccxt_core::types::order::order_request::{OrderRequest, TriggerPriceType, TrailingAmountType};
//! use ccxt_core::types::{OrderSide, OrderType, TimeInForce};
//! use ccxt_core::types::financial::{Amount, Price};
//! use rust_decimal_macros::dec;
//!
//! // Simple market order
//! let request = OrderRequest::builder()
//!     .symbol("BTC/USDT")
//!     .side(OrderSide::Buy)
//!     .order_type(OrderType::Market)
//!     .amount(Amount::new(dec!(0.1)))
//!     .build().expect("OrderRequest should build successfully");
//!
//! // Limit order with post-only
//! let request = OrderRequest::builder()
//!     .symbol("ETH/USDT")
//!     .side(OrderSide::Sell)
//!     .order_type(OrderType::Limit)
//!     .amount(Amount::new(dec!(1.0)))
//!     .price(Price::new(dec!(3000)))
//!     .post_only(true)
//!     .build().expect("OrderRequest should build successfully");
//!
//! // Stop loss with mark price trigger
//! let request = OrderRequest::builder()
//!     .symbol("BTC/USDT")
//!     .side(OrderSide::Sell)
//!     .order_type(OrderType::StopLoss)
//!     .amount(Amount::new(dec!(0.1)))
//!     .stop_price(Price::new(dec!(45000)))
//!     .trigger_price_type(TriggerPriceType::MarkPrice)
//!     .build().expect("OrderRequest should build successfully");
//! ```

use crate::types::financial::{Amount, Price};
use crate::types::order::order::{OrderSide, OrderType, TimeInForce};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::marker::PhantomData;

// ============================================================================
// Error Types
// ============================================================================

/// Validation errors for order request building.
#[derive(Debug, Clone, thiserror::Error)]
pub enum OrderValidationError {
    /// Limit order requires price
    #[error("Limit order requires price")]
    MissingPriceForLimitOrder,

    /// Stop order requires stop_price
    #[error("Stop order requires stop_price")]
    MissingStopPrice,

    /// TrailingStop requires trailing_callback_rate
    #[error("TrailingStop order requires trailing_callback_rate")]
    MissingTrailingCallback,

    /// TrailingStop requires both trailing_callback_rate and trailing_amount_type
    #[error(
        "TrailingStop order requires both trailing_callback_rate and trailing_amount_type to be set together"
    )]
    IncompleteTrailingConfig,

    /// Quote amount is only supported for market orders
    #[error("Quote amount (e.g., spend USDT) is only supported for market orders")]
    QuoteAmountOnlyForMarketOrder,

    /// Configuration conflict between fields
    #[error("Configuration conflict between '{field1}' and '{field2}': {message}")]
    ConfigurationConflict {
        /// First conflicting field
        field1: String,
        /// Second conflicting field
        field2: String,
        /// Conflict description
        message: String,
    },
}

// ============================================================================
// Enum Types for Enhanced Type Safety
// ============================================================================

/// Trigger price type for conditional orders.
///
/// Specifies which price source to use for triggering conditional orders
/// (stop loss, take profit, trailing stop, etc.).
///
/// # Exchange Mapping
///
/// - **Binance**: `workingType` field - `CONTRACT_PRICE`, `MARK_PRICE`
/// - **Bybit**: `triggerPriceType` - `LastPrice`, `IndexPrice`, `MarkPrice`
/// - **Bitget**: `tpTriggerBy`/`slTriggerBy` - `last_price`, `mark_price`, `index_price`
/// - **OKX**: `slTriggerPxType`/`tpTriggerPxType` - `last`, `index`, `mark`
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::order::order_request::TriggerPriceType;
///
/// let trigger_type = TriggerPriceType::MarkPrice;
/// assert_eq!(trigger_type.as_str(), "mark_price");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerPriceType {
    /// Latest trade price (default for most exchanges)
    /// - Binance: CONTRACT_PRICE
    /// - Bybit/Bitget/OKX: LastPrice
    LastPrice,

    /// Mark price (used for futures to prevent manipulation)
    /// - Binance: MARK_PRICE
    /// - Bybit/Bitget/OKX: MarkPrice
    MarkPrice,

    /// Index price (global average across exchanges)
    /// - Bybit/Bitget/OKX: IndexPrice
    IndexPrice,
}

impl TriggerPriceType {
    /// Returns the string representation for API requests.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LastPrice => "last_price",
            Self::MarkPrice => "mark_price",
            Self::IndexPrice => "index_price",
        }
    }
}

impl std::fmt::Display for TriggerPriceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Trailing callback amount type for trailing stop orders.
///
/// Specifies whether the trailing callback value is in basis points or percentage.
///
/// # Exchange Mapping
///
/// - **Binance**:
///   - `trailingDelta` - basis points (e.g., 100 = 1%)
///   - No direct percentage field
/// - **Bybit**: `trailingPercent` - percentage (e.g., 1.5 = 1.5%)
/// - **Bitget**: `callbackRate` - percentage
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::order::order_request::TrailingAmountType;
///
/// let trailing_type = TrailingAmountType::Percent;
/// assert_eq!(trailing_type.as_str(), "percent");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrailingAmountType {
    /// Basis points (1 basis point = 0.01%)
    /// Example: 100 basis points = 1%
    /// Used by: Binance trailingDelta
    BasisPoints,

    /// Percentage value
    /// Example: 1.5 means 1.5%
    /// Used by: Bybit trailingPercent, Bitget callbackRate
    Percent,
}

impl TrailingAmountType {
    /// Returns the string representation for API requests.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BasisPoints => "basis_points",
            Self::Percent => "percent",
        }
    }
}

impl std::fmt::Display for TrailingAmountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Position side for hedge mode futures trading.
///
/// In hedge mode, you can hold both long and short positions simultaneously.
/// This enum specifies which position the order applies to.
///
/// # Exchange Mapping
///
/// - **Binance**: `positionSide` - `LONG`, `SHORT`, `BOTH`
/// - **Bybit**: `positionIdx` - 0 (one-way), 1 (long), 2 (short)
/// - **OKX**: `posSide` - `long`, `short`, `net`
/// - **Bitget**: `posSide` - `long`, `short` (hedge mode required)
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::order::order_request::PositionSide;
///
/// let position = PositionSide::Long;
/// assert_eq!(position.as_str(), "long");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionSide {
    /// Long position
    Long,
    /// Short position
    Short,
    /// Both position (one-way mode, not hedge mode)
    Both,
}

impl PositionSide {
    /// Returns the string representation for API requests.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
            Self::Both => "both",
        }
    }
}

// ============================================================================
// Amount Specification for Base/Quote Amount Support
// ============================================================================

/// Amount specification for order quantity.
///
/// Supports both base currency amount and quote currency amount.
/// This is particularly useful for market buy orders where you want to specify
/// how much quote currency (e.g., USDT) to spend rather than how much base
/// currency (e.g., BTC) to buy.
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::order::order_request::AmountSpec;
/// use ccxt_core::types::financial::Amount;
/// use rust_decimal_macros::dec;
///
/// // Buy 0.1 BTC (base amount)
/// let base = AmountSpec::Base(Amount::new(dec!(0.1)));
///
/// // Spend 1000 USDT (quote amount)
/// let quote = AmountSpec::Quote(Amount::new(dec!(1000)));
/// ```
///
/// # Exchange Support
/// - **Binance**: `quoteOrderQty` for market buys
/// - **KuCoin**: `funds` for market buys
/// - **Bybit/OKX/Bitget**: Only base amount supported
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum AmountSpec {
    /// Base currency amount (e.g., BTC quantity)
    Base(Amount),
    /// Quote currency amount (e.g., USDT value)
    Quote(Amount),
}

impl AmountSpec {
    /// Creates a base currency amount specification.
    #[must_use]
    pub fn base(amount: impl Into<Amount>) -> Self {
        Self::Base(amount.into())
    }

    /// Creates a quote currency amount specification.
    #[must_use]
    pub fn quote(amount: impl Into<Amount>) -> Self {
        Self::Quote(amount.into())
    }

    /// Returns `true` if this is a base currency amount.
    #[must_use]
    pub fn is_base(&self) -> bool {
        matches!(self, Self::Base(_))
    }

    /// Returns `true` if this is a quote currency amount.
    #[must_use]
    pub fn is_quote(&self) -> bool {
        matches!(self, Self::Quote(_))
    }

    /// Returns the underlying amount value.
    #[must_use]
    pub fn as_decimal(&self) -> rust_decimal::Decimal {
        match self {
            Self::Base(amount) | Self::Quote(amount) => amount.as_decimal(),
        }
    }
}

impl From<Amount> for AmountSpec {
    fn from(amount: Amount) -> Self {
        Self::Base(amount)
    }
}

impl From<rust_decimal::Decimal> for AmountSpec {
    fn from(amount: rust_decimal::Decimal) -> Self {
        Self::Base(Amount::new(amount))
    }
}

impl std::fmt::Display for AmountSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(amount) | Self::Quote(amount) => write!(f, "{}", amount),
        }
    }
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// OrderRequest struct
// ============================================================================

/// Order request configuration built via builder pattern.
///
/// This struct contains all the fields needed to create an order on an exchange.
/// Use [`OrderRequest::builder()`] to construct instances with compile-time
/// validation of required fields.
///
/// # Field Design Philosophy
///
/// Fields are organized into logical groups:
/// 1. **Core Required** (4): symbol, side, order_type, amount
/// 2. **Common Optional** (5): price, time_in_force, client_order_id, reduce_only, post_only
/// 3. **Conditional Orders** (2): stop_price, trigger_price_type (merged from 6 legacy fields)
/// 4. **Trailing Stop** (3): trailing_callback_rate, trailing_amount_type, activation_price
/// 5. **Futures** (1): position_side
/// 6. **TP/SL Limit** (2): tp_limit_price, sl_limit_price
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::order::order_request::{OrderRequest, TriggerPriceType};
/// use ccxt_core::types::{OrderSide, OrderType};
/// use ccxt_core::types::financial::{Amount, Price};
/// use rust_decimal_macros::dec;
///
/// // Simple market order
/// let request = OrderRequest::builder()
///     .symbol("BTC/USDT")
///     .side(OrderSide::Buy)
///     .order_type(OrderType::Market)
///     .amount(Amount::new(dec!(0.1)))
///     .build().expect("OrderRequest should build successfully");
///
/// // Stop loss with mark price trigger
/// let request = OrderRequest::builder()
///     .symbol("BTC/USDT")
///     .side(OrderSide::Sell)
///     .order_type(OrderType::StopLoss)
///     .amount(Amount::new(dec!(0.1)))
///     .stop_price(Price::new(dec!(45000)))
///     .trigger_price_type(TriggerPriceType::MarkPrice)
///     .build().expect("OrderRequest should build successfully");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    // =========================================================================
    // Core Required Fields (4)
    // =========================================================================
    /// Trading pair symbol (e.g., "BTC/USDT").
    ///
    /// # Exchange Format Mapping
    /// - **Binance/Bybit/Bitget/MEXC**: `BTCUSDT` (no separator)
    /// - **OKX/KuCoin**: `BTC-USDT` (dash separator)
    /// - **Kraken**: `XBT/USD` (slash separator, BTC→XBT)
    /// - **Gate.io**: `BTC_USDT` (underscore separator)
    ///
    /// The exchange implementation will convert the unified format to its specific format.
    pub symbol: String,

    /// Order side (buy or sell).
    ///
    /// # Exchange Mapping
    /// - **Binance/Bybit/OKX/Bitget**: `Buy`/`Sell` or `buy`/`sell`
    /// - **Kraken**: `buy`/`sell` (lowercase)
    /// - **KuCoin**: `buy`/`sell`
    pub side: OrderSide,

    /// Order type (market, limit, stop loss, take profit, etc.).
    ///
    /// # Supported Types
    /// - `Market`: Execute immediately at best available price
    /// - `Limit`: Execute at specified price or better
    /// - `StopLoss`: Trigger market order when stop price is reached
    /// - `StopLossLimit`: Trigger limit order when stop price is reached
    /// - `TakeProfit`: Trigger market order when target price is reached
    /// - `TakeProfitLimit`: Trigger limit order when target price is reached
    /// - `TrailingStop`: Dynamic stop loss that follows price movement
    ///
    /// # Exchange Mapping
    /// - **Binance**: `MARKET`, `LIMIT`, `STOP_LOSS`, `TAKE_PROFIT`, etc.
    /// - **Bybit**: `Market`, `Limit`
    /// - **OKX**: `market`, `limit`, with algo orders for stop/tp
    /// - **Bitget**: `market`, `limit`, with trigger orders
    pub order_type: OrderType,

    /// Order amount/quantity.
    ///
    /// # Amount Specification
    /// Supports both base and quote currency amounts:
    /// - **Base currency**: BTC quantity (most common)
    /// - **Quote currency**: USDT value (for market buys on Binance/KuCoin)
    ///
    /// # Exchange Field Names
    /// - **Binance**: `quantity` (base) or `quoteOrderQty` (quote)
    /// - **Bybit**: `qty`
    /// - **OKX**: `sz` (size)
    /// - **KuCoin**: `size` (base) or `funds` (quote)
    /// - **Bitget**: `sz`
    pub amount: AmountSpec,

    // =========================================================================
    // Common Optional Fields (5)
    // =========================================================================
    /// Order price (required for limit orders, optional for market orders).
    ///
    /// # When Required
    /// - **Limit orders**: Mandatory
    /// - **Stop loss limit / Take profit limit**: Mandatory (execution price)
    /// - **Market orders**: Not used (ignored if provided)
    ///
    /// # Exchange Mapping
    /// - **Binance/Bybit/KuCoin**: `price`
    /// - **OKX**: `px`
    /// - **Bitget**: `price`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Price>,

    /// Time in force (how long the order remains active).
    ///
    /// # Supported Values
    /// - `GTC` (Good Till Cancelled): Order stays active until filled or cancelled
    /// - `IOC` (Immediate Or Cancel): Fill immediately, cancel remaining
    /// - `FOK` (Fill Or Kill): Fill completely or cancel entirely
    /// - `PO` (Post Only): Only place as maker order (rebalance fees)
    ///
    /// # Exchange Mapping
    /// - **Binance**: `timeInForce` - `GTC`, `IOC`, `FOK`
    /// - **Bybit**: `timeInForce` - `GTC`, `IOC`, `FOK`, `PostOnly`
    /// - **OKX**: No direct field, use order type `post_only`
    /// - **Bitget**: `force` - `normal` (GTC), `fok`, `ioc`, `post_only`
    /// - **Kraken**: `oflags` - composite flags like `post`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,

    /// Client-specified order ID for idempotency and tracking.
    ///
    /// # Use Cases
    /// - Prevent duplicate order submission (idempotency)
    /// - Track orders across system restarts
    /// - Correlate internal order IDs with exchange orders
    ///
    /// # Exchange Mapping
    /// - **Binance**: `newClientOrderId`
    /// - **Bybit**: `orderLinkId`
    /// - **OKX**: `clOrdId`
    /// - **KuCoin**: `clientOid` (required!)
    /// - **Bitget**: `clientOid`
    /// - **Kraken**: `userref` (integer only)
    ///
    /// # Constraints
    /// - Maximum length varies by exchange (typically 32-64 chars)
    /// - Must be unique per exchange account
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,

    /// Reduce-only flag (for futures/perpetual contracts).
    ///
    /// When `true`, the order will only reduce an existing position.
    /// If the order would increase the position, it will be rejected.
    ///
    /// # Use Cases
    /// - Closing positions without accidentally opening new ones
    /// - Risk management in hedging strategies
    ///
    /// # Exchange Support
    /// - **Binance**: `reduceOnly` (futures only)
    /// - **Bybit**: `reduceOnly` (linear/inverse contracts)
    /// - **OKX**: `reduceOnly` (swap/futures)
    /// - **Bitget**: `reduceOnly` (USDT/USDC futures)
    /// - **Spot markets**: Not applicable (ignored)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,

    /// Post-only flag (maker-only order).
    ///
    /// When `true`, the order will only be placed if it would not immediately
    /// match with an existing order (i.e., it will be a maker order).
    /// If it would match immediately, the order is cancelled.
    ///
    /// # Benefits
    /// - Lower trading fees (maker fees < taker fees)
    /// - Guaranteed liquidity provision
    ///
    /// # Exchange Implementation
    /// - **Binance**: Use `order_type = LimitMaker` or `timeInForce = GTE_GTC`
    /// - **Bybit**: `timeInForce = PostOnly`
    /// - **OKX**: `order_type = post_only`
    /// - **Bitget**: `force = post_only`
    /// - **Kraken**: `oflags = post`
    ///
    /// # Note
    /// Some exchanges use order type or time_in_force instead of a separate flag.
    /// The exchange implementation will handle the mapping appropriately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,

    // =========================================================================
    // Conditional Orders (2 fields, merged from 6 legacy fields)
    // =========================================================================
    /// Stop/trigger price for conditional orders.
    ///
    /// # Unified Field Design
    /// This single field replaces the legacy redundant fields:
    /// - `stop_price` (Binance style)
    /// - `trigger_price` (OKX style)
    /// - `take_profit_price` (preset TP)
    /// - `stop_loss_price` (preset SL)
    ///
    /// The order semantics are determined by `order_type`:
    /// - `StopLoss`/`StopLossLimit`: This is the stop loss trigger price
    /// - `TakeProfit`/`TakeProfitLimit`: This is the take profit trigger price
    /// - `TrailingStop`: This is the activation price (use `activation_price` instead)
    /// - `Limit`/`Market` with preset TP/SL: Use `tp_limit_price`/`sl_limit_price`
    ///
    /// # Exchange Mapping
    /// - **Binance**: `stopPrice`
    /// - **Bybit**: `triggerPrice`
    /// - **OKX**: `slTriggerPx` or `tpTriggerPx` (in algo orders)
    /// - **Bitget**: `triggerPrice` or `presetStopLossPrice`/`presetTakeProfitPrice`
    /// - **Kraken**: `price` (for stop-loss/take-profit order types)
    ///
    /// # When Required
    /// - **StopLoss/StopLossLimit**: Mandatory
    /// - **TakeProfit/TakeProfitLimit**: Mandatory
    /// - **Regular Limit/Market**: Optional (for preset TP/SL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<Price>,

    /// Trigger price type (which price source triggers the order).
    ///
    /// # Unified Field Design
    /// This field replaces the legacy redundant fields:
    /// - `working_type` (Binance)
    /// - `tp_trigger_by` (Binance/Bitget)
    /// - `sl_trigger_by` (Binance/Bitget)
    ///
    /// # Supported Types
    /// See [`TriggerPriceType`] for available options.
    ///
    /// # Exchange Mapping
    /// - **Binance**: `workingType` - `CONTRACT_PRICE`, `MARK_PRICE`
    /// - **Bybit**: `triggerPriceType` - `LastPrice`, `IndexPrice`, `MarkPrice`
    /// - **OKX**: `slTriggerPxType`/`tpTriggerPxType` - `last`, `index`, `mark`
    /// - **Bitget**: `tpTriggerBy`/`slTriggerBy` - `last_price`, `mark_price`, `index_price`
    ///
    /// # When to Use
    /// - **Futures contracts**: Use `MarkPrice` to prevent liquidation wicks
    /// - **Spot markets**: Use `LastPrice` (default)
    /// - **High leverage**: Prefer `MarkPrice` for stability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price_type: Option<TriggerPriceType>,

    // =========================================================================
    // Trailing Stop Orders (3 fields, merged from 4 legacy fields)
    // =========================================================================
    /// Trailing callback rate for trailing stop orders.
    ///
    /// # Unified Field Design
    /// This field replaces the legacy redundant fields:
    /// - `trailing_delta` (Binance basis points)
    /// - `trailing_percent` (Bybit percentage)
    /// - `callback_rate` (Bitget percentage)
    ///
    /// The unit is determined by `trailing_amount_type`.
    ///
    /// # How Trailing Stops Work
    /// 1. Order activates when price reaches `activation_price`
    /// 2. Stop price follows the favorable price movement by the callback rate
    /// 3. When price reverses by the callback rate, order triggers
    ///
    /// # Examples
    /// - Long position: Stop follows price UP, triggers when price drops by callback%
    /// - Short position: Stop follows price DOWN, triggers when price rises by callback%
    ///
    /// # Exchange Mapping
    /// - **Binance**: `trailingDelta` (basis points, e.g., 100 = 1%)
    /// - **Bybit**: `trailingPercent` (percentage, e.g., 1.5 = 1.5%)
    /// - **Bitget**: `callbackRate` (percentage)
    ///
    /// # When Required
    /// - **TrailingStop order type**: Mandatory
    /// - **Other order types**: Ignored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_callback_rate: Option<rust_decimal::Decimal>,

    /// Type of trailing callback amount (basis points vs percentage).
    ///
    /// # Unified Field Design
    /// This field clarifies the unit of `trailing_callback_rate`.
    ///
    /// # Examples
    /// - `BasisPoints` + rate 100 = 1.00%
    /// - `Percent` + rate 1.5 = 1.5%
    ///
    /// # Exchange Defaults
    /// - **Binance**: Always basis points (`trailingDelta`)
    /// - **Bybit/Bitget**: Always percentage
    ///
    /// When not specified, the exchange implementation will use its default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_amount_type: Option<TrailingAmountType>,

    /// Activation price for trailing stop orders.
    ///
    /// The price at which the trailing stop mechanism activates.
    /// If not specified, trailing starts immediately when order is placed.
    ///
    /// # Use Cases
    /// - Wait for price to reach a certain level before trailing begins
    /// - Lock in profits after a target price is hit
    ///
    /// # Exchange Mapping
    /// - **Binance**: `activationPrice` (futures only)
    /// - **Bybit**: Not directly supported (use conditional orders)
    /// - **Bitget**: Not directly supported
    ///
    /// # When Used
    /// - **TrailingStop order type**: Optional (immediate activation if omitted)
    /// - **Other order types**: Ignored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_price: Option<Price>,

    // =========================================================================
    // Futures-Specific Fields (1)
    // =========================================================================
    /// Position side (for hedge mode futures trading).
    ///
    /// In hedge mode, you can hold both long and short positions simultaneously.
    /// This field specifies which position the order applies to.
    ///
    /// # Supported Values
    /// - `Long`: Open or close long position
    /// - `Short`: Open or close short position
    /// - `Both`: One-way mode (default, not hedge mode)
    ///
    /// # Exchange Mapping
    /// - **Binance**: `positionSide` - `LONG`, `SHORT`, `BOTH`
    /// - **Bybit**: `positionIdx` - 0 (one-way), 1 (long), 2 (short)
    /// - **OKX**: `posSide` - `long`, `short`, `net`
    /// - **Bitget**: `posSide` - `long`, `short` (hedge mode required)
    ///
    /// # When Required
    /// - **Hedge mode futures**: Mandatory
    /// - **One-way mode futures**: Optional (defaults to `Both`)
    /// - **Spot markets**: Not applicable (ignored)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<PositionSide>,

    // =========================================================================
    // TP/SL Limit Execution Prices (2)
    // =========================================================================
    /// Take profit limit order execution price.
    ///
    /// When a take profit order is triggered, if this field is set,
    /// it places a **limit order** at this price instead of a market order.
    ///
    /// # Use Case
    /// - Control slippage when taking profit
    /// - Ensure minimum profit execution price
    ///
    /// # Exchange Support
    /// - **Bitget**: `tpLimitPrice` (with `tpOrderType = limit`)
    /// - **Binance**: Not directly supported (use STOP_LIMIT order type)
    /// - **Bybit**: Not directly supported
    ///
    /// # When Used
    /// - **TakeProfitLimit order type**: Mandatory (this is the limit price)
    /// - **TakeProfit order type**: Optional (converts to limit instead of market)
    /// - **Other order types**: Ignored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tp_limit_price: Option<Price>,

    /// Stop loss limit order execution price.
    ///
    /// When a stop loss order is triggered, if this field is set,
    /// it places a **limit order** at this price instead of a market order.
    ///
    /// # Use Case
    /// - Control slippage when stopping loss
    /// - Prevent worst-case execution in volatile markets
    ///
    /// # Exchange Support
    /// - **Bitget**: `slLimitPrice` (with `slOrderType = limit`)
    /// - **Binance**: Not directly supported (use STOP_LOSS_LIMIT order type)
    /// - **Bybit**: Not directly supported
    ///
    /// # When Used
    /// - **StopLossLimit order type**: Mandatory (this is the limit price)
    /// - **StopLoss order type**: Optional (converts to limit instead of market)
    /// - **Other order types**: Ignored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sl_limit_price: Option<Price>,

    // =========================================================================
    // Exchange-Specific Parameters
    // =========================================================================
    /// Exchange-specific parameters not covered by unified model.
    ///
    /// # Examples
    /// - Binance: `priceMatch`, `selfTradePreventionMode`, `goodTillDate`
    /// - KuCoin: `hidden`, `iceberg`, `visibleSize`
    /// - OKX: `tag`, `attachAlgoOrds`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<HashMap<String, Value>>,
}

impl OrderRequest {
    /// Creates a new builder for constructing an `OrderRequest`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ccxt_core::types::order::order_request::OrderRequest;
    /// use ccxt_core::types::{OrderSide, OrderType};
    /// use ccxt_core::types::financial::Amount;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequest::builder()
    ///     .symbol("BTC/USDT")
    ///     .side(OrderSide::Buy)
    ///     .order_type(OrderType::Market)
    ///     .amount(Amount::new(dec!(0.1)))
    ///     .build().expect("OrderRequest should build successfully");
    /// ```
    #[must_use]
    pub fn builder() -> OrderRequestBuilder<Missing, Missing, Missing, Missing> {
        OrderRequestBuilder::new()
    }

    /// Returns true if this is a market order.
    #[must_use]
    pub fn is_market_order(&self) -> bool {
        matches!(self.order_type, OrderType::Market)
    }

    /// Returns true if this is a limit order.
    #[must_use]
    pub fn is_limit_order(&self) -> bool {
        matches!(self.order_type, OrderType::Limit | OrderType::LimitMaker)
    }

    /// Returns true if this is a stop order.
    #[must_use]
    pub fn is_stop_order(&self) -> bool {
        matches!(
            self.order_type,
            OrderType::StopLoss
                | OrderType::StopLossLimit
                | OrderType::StopMarket
                | OrderType::StopLimit
        )
    }

    /// Returns true if this is a take profit order.
    #[must_use]
    pub fn is_take_profit_order(&self) -> bool {
        matches!(
            self.order_type,
            OrderType::TakeProfit | OrderType::TakeProfitLimit
        )
    }

    /// Returns true if this is a trailing stop order.
    #[must_use]
    pub fn is_trailing_stop_order(&self) -> bool {
        matches!(self.order_type, OrderType::TrailingStop)
    }
}

// ============================================================================
// Typestate markers
// ============================================================================

/// Marker type indicating a required field has not been set.
#[derive(Debug, Clone, Copy, Default)]
pub struct Missing;

/// Marker type indicating a required field has been set with value T.
#[derive(Debug, Clone)]
pub struct Set<T>(pub T);

// ============================================================================
// OrderRequestBuilder with typestate pattern
// ============================================================================

/// Builder for `OrderRequest` with typestate pattern for required fields.
///
/// The typestate pattern ensures at compile-time that all required fields
/// (symbol, side, `order_type`, amount) are set before the `build()` method
/// becomes available.
///
/// # Type Parameters
///
/// * `Symbol` - Either `Missing` or `Set<String>` for the symbol field
/// * `Side` - Either `Missing` or `Set<OrderSide>` for the side field
/// * `Type` - Either `Missing` or `Set<OrderType>` for the `order_type` field
/// * `Amt` - Either `Missing` or `Set<Amount>` for the amount field
///
/// # Examples
///
/// ```rust
/// use ccxt_core::types::order::order_request::{OrderRequestBuilder, TriggerPriceType};
/// use ccxt_core::types::{OrderSide, OrderType, TimeInForce};
/// use ccxt_core::types::financial::{Amount, Price};
/// use rust_decimal_macros::dec;
///
/// // This compiles - all required fields are set
/// let request = OrderRequestBuilder::new()
///     .symbol("BTC/USDT")
///     .side(OrderSide::Buy)
///     .order_type(OrderType::Limit)
///     .amount(Amount::new(dec!(0.1)))
///     .price(Price::new(dec!(50000)))
///     .time_in_force(TimeInForce::GTC)
///     .build().expect("OrderRequest should build successfully");
/// ```
#[derive(Debug, Clone)]
pub struct OrderRequestBuilder<Symbol, Side, Type, Amt> {
    symbol: Symbol,
    side: Side,
    order_type: Type,
    amount: Amt,
    price: Option<Price>,
    stop_price: Option<Price>,
    time_in_force: Option<TimeInForce>,
    client_order_id: Option<String>,
    reduce_only: Option<bool>,
    post_only: Option<bool>,
    trigger_price_type: Option<TriggerPriceType>,
    trailing_callback_rate: Option<rust_decimal::Decimal>,
    trailing_amount_type: Option<TrailingAmountType>,
    activation_price: Option<Price>,
    position_side: Option<PositionSide>,
    tp_limit_price: Option<Price>,
    sl_limit_price: Option<Price>,
    extra: Option<HashMap<String, Value>>,
    _marker: PhantomData<(Symbol, Side, Type, Amt)>,
}

impl OrderRequestBuilder<Missing, Missing, Missing, Missing> {
    /// Creates a new `OrderRequestBuilder` with all required fields unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            symbol: Missing,
            side: Missing,
            order_type: Missing,
            amount: Missing,
            price: None,
            stop_price: None,
            time_in_force: None,
            client_order_id: None,
            reduce_only: None,
            post_only: None,
            trigger_price_type: None,
            trailing_callback_rate: None,
            trailing_amount_type: None,
            activation_price: None,
            position_side: None,
            tp_limit_price: None,
            sl_limit_price: None,
            _marker: PhantomData,
            extra: None,
        }
    }

    /// Creates a market buy order builder with base currency amount.
    ///
    /// # Example
    /// ```rust
    /// use ccxt_core::types::order::order_request::OrderRequestBuilder;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequestBuilder::market_buy("BTC/USDT", dec!(0.1))
    ///     .build()
    ///     .expect("OrderRequest should build successfully");
    /// ```
    #[must_use]
    pub fn market_buy(
        symbol: &str,
        amount: impl Into<Amount>,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.to_string()),
            side: Set(OrderSide::Buy),
            order_type: Set(OrderType::Market),
            amount: Set(AmountSpec::base(amount)),
            price: None,
            stop_price: None,
            time_in_force: None,
            client_order_id: None,
            reduce_only: None,
            post_only: None,
            trigger_price_type: None,
            trailing_callback_rate: None,
            trailing_amount_type: None,
            activation_price: None,
            position_side: None,
            tp_limit_price: None,
            sl_limit_price: None,
            _marker: PhantomData,
            extra: None,
        }
    }

    /// Creates a market buy order builder with quote currency amount.
    ///
    /// # Example
    /// ```rust
    /// use ccxt_core::types::order::order_request::OrderRequestBuilder;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequestBuilder::market_buy_quote("BTC/USDT", dec!(1000))
    ///     .build()
    ///     .expect("OrderRequest should build successfully");
    /// ```
    #[must_use]
    pub fn market_buy_quote(
        symbol: &str,
        amount: impl Into<Amount>,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.to_string()),
            side: Set(OrderSide::Buy),
            order_type: Set(OrderType::Market),
            amount: Set(AmountSpec::quote(amount)),
            price: None,
            stop_price: None,
            time_in_force: None,
            client_order_id: None,
            reduce_only: None,
            post_only: None,
            trigger_price_type: None,
            trailing_callback_rate: None,
            trailing_amount_type: None,
            activation_price: None,
            position_side: None,
            tp_limit_price: None,
            sl_limit_price: None,
            _marker: PhantomData,
            extra: None,
        }
    }

    /// Creates a market sell order builder with base currency amount.
    ///
    /// # Example
    /// ```rust
    /// use ccxt_core::types::order::order_request::OrderRequestBuilder;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequestBuilder::market_sell("BTC/USDT", dec!(0.1))
    ///     .build()
    ///     .expect("OrderRequest should build successfully");
    /// ```
    #[must_use]
    pub fn market_sell(
        symbol: &str,
        amount: impl Into<Amount>,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.to_string()),
            side: Set(OrderSide::Sell),
            order_type: Set(OrderType::Market),
            amount: Set(AmountSpec::base(amount)),
            price: None,
            stop_price: None,
            time_in_force: None,
            client_order_id: None,
            reduce_only: None,
            post_only: None,
            trigger_price_type: None,
            trailing_callback_rate: None,
            trailing_amount_type: None,
            activation_price: None,
            position_side: None,
            tp_limit_price: None,
            sl_limit_price: None,
            _marker: PhantomData,
            extra: None,
        }
    }

    /// Creates a limit buy order builder with base currency amount.
    ///
    /// # Example
    /// ```rust
    /// use ccxt_core::types::order::order_request::OrderRequestBuilder;
    /// use ccxt_core::types::financial::Price;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequestBuilder::limit_buy("BTC/USDT", dec!(0.1), dec!(50000))
    ///     .build()
    ///     .expect("OrderRequest should build successfully");
    /// ```
    #[must_use]
    pub fn limit_buy(
        symbol: &str,
        amount: impl Into<Amount>,
        price: impl Into<Price>,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.to_string()),
            side: Set(OrderSide::Buy),
            order_type: Set(OrderType::Limit),
            amount: Set(AmountSpec::base(amount)),
            price: Some(price.into()),
            stop_price: None,
            time_in_force: Some(TimeInForce::GTC),
            client_order_id: None,
            reduce_only: None,
            post_only: None,
            trigger_price_type: None,
            trailing_callback_rate: None,
            trailing_amount_type: None,
            activation_price: None,
            position_side: None,
            tp_limit_price: None,
            sl_limit_price: None,
            _marker: PhantomData,
            extra: None,
        }
    }

    /// Creates a limit Buy order builder with quote currency amount.
    ///
    /// Note: This method creates a limit order with quote amount, which may not be
    /// supported by all exchanges. Check exchange-specific documentation.
    #[must_use]
    pub fn limit_buy_quote(
        symbol: &str,
        amount: impl Into<Amount>,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.to_string()),
            side: Set(OrderSide::Buy),
            order_type: Set(OrderType::Limit),
            amount: Set(AmountSpec::quote(amount)),
            price: None,
            stop_price: None,
            time_in_force: None,
            client_order_id: None,
            reduce_only: None,
            post_only: None,
            trigger_price_type: None,
            trailing_callback_rate: None,
            trailing_amount_type: None,
            activation_price: None,
            position_side: None,
            tp_limit_price: None,
            sl_limit_price: None,
            _marker: PhantomData,
            extra: None,
        }
    }

    /// Creates a limit sell order builder with base currency amount.
    ///
    /// # Example
    /// ```rust
    /// use ccxt_core::types::order::order_request::OrderRequestBuilder;
    /// use ccxt_core::types::financial::Price;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequestBuilder::limit_sell("BTC/USDT", dec!(0.1), dec!(50000))
    ///     .build()
    ///     .expect("OrderRequest should build successfully");
    /// ```
    #[must_use]
    pub fn limit_sell(
        symbol: &str,
        amount: impl Into<Amount>,
        price: impl Into<Price>,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.to_string()),
            side: Set(OrderSide::Sell),
            order_type: Set(OrderType::Limit),
            amount: Set(AmountSpec::base(amount)),
            price: Some(price.into()),
            stop_price: None,
            time_in_force: Some(TimeInForce::GTC),
            client_order_id: None,
            reduce_only: None,
            post_only: None,
            trigger_price_type: None,
            trailing_callback_rate: None,
            trailing_amount_type: None,
            activation_price: None,
            position_side: None,
            tp_limit_price: None,
            sl_limit_price: None,
            _marker: PhantomData,
            extra: None,
        }
    }
}

impl Default for OrderRequestBuilder<Missing, Missing, Missing, Missing> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Required field setters (transition from Missing to Set)
// ============================================================================

impl<Side, Type, Amt> OrderRequestBuilder<Missing, Side, Type, Amt> {
    /// Sets the trading symbol (required).
    pub fn symbol(
        self,
        symbol: impl Into<String>,
    ) -> OrderRequestBuilder<Set<String>, Side, Type, Amt> {
        OrderRequestBuilder {
            symbol: Set(symbol.into()),
            side: self.side,
            order_type: self.order_type,
            amount: self.amount,
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }
}

impl<Symbol, Type, Amt> OrderRequestBuilder<Symbol, Missing, Type, Amt> {
    /// Sets the order side (required).
    pub fn side(self, side: OrderSide) -> OrderRequestBuilder<Symbol, Set<OrderSide>, Type, Amt> {
        OrderRequestBuilder {
            symbol: self.symbol,
            side: Set(side),
            order_type: self.order_type,
            amount: self.amount,
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }
}

impl<Symbol, Side, Amt> OrderRequestBuilder<Symbol, Side, Missing, Amt> {
    /// Sets the order type (required).
    pub fn order_type(
        self,
        order_type: OrderType,
    ) -> OrderRequestBuilder<Symbol, Side, Set<OrderType>, Amt> {
        OrderRequestBuilder {
            symbol: self.symbol,
            side: self.side,
            order_type: Set(order_type),
            amount: self.amount,
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }
}

impl<Symbol, Side, Type> OrderRequestBuilder<Symbol, Side, Type, Missing> {
    /// Sets the order amount (required).
    ///
    /// Accepts both base currency amount and quote currency amount.
    /// For market buy orders on Binance/KuCoin, you can use `quote_amount()`
    /// to specify how much quote currency to spend.
    pub fn amount(
        self,
        amount: impl Into<AmountSpec>,
    ) -> OrderRequestBuilder<Symbol, Side, Type, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: self.symbol,
            side: self.side,
            order_type: self.order_type,
            amount: Set(amount.into()),
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }

    /// Sets the order amount as base currency quantity (convenience method).
    ///
    /// # Example
    /// ```rust,ignore
    /// // Buy 0.1 BTC
    /// .base_amount(dec!(0.1))
    /// ```
    pub fn base_amount(
        self,
        amount: impl Into<Amount>,
    ) -> OrderRequestBuilder<Symbol, Side, Type, Set<AmountSpec>> {
        self.amount(AmountSpec::Base(amount.into()))
    }

    /// Sets the order amount as quote currency value (convenience method).
    ///
    /// This is useful for market buy orders where you want to specify
    /// how much quote currency (e.g., USDT) to spend.
    ///
    /// # Exchange Support
    /// - **Binance**: `quoteOrderQty`
    /// - **KuCoin**: `funds`
    /// - **Bybit/OKX/Bitget**: Not supported (will be ignored)
    ///
    /// # Example
    /// ```rust,ignore
    /// // Spend 1000 USDT
    /// .quote_amount(dec!(1000))
    /// ```
    pub fn quote_amount(
        self,
        amount: impl Into<Amount>,
    ) -> OrderRequestBuilder<Symbol, Side, Type, Set<AmountSpec>> {
        self.amount(AmountSpec::Quote(amount.into()))
    }
}

// ============================================================================
// Optional field setters (available at any state)
// ============================================================================

impl<Symbol, Side, Type, Amt> OrderRequestBuilder<Symbol, Side, Type, Amt> {
    /// Sets the order price (optional, required for limit orders).
    pub fn price(mut self, price: Price) -> Self {
        self.price = Some(price);
        self
    }

    /// Sets the stop/trigger price for conditional orders.
    ///
    /// # Usage
    /// - For `StopLoss`/`StopLossLimit`: Sets the stop loss trigger price
    /// - For `TakeProfit`/`TakeProfitLimit`: Sets the take profit trigger price
    pub fn stop_price(mut self, stop_price: Price) -> Self {
        self.stop_price = Some(stop_price);
        self
    }

    /// Sets the time in force (optional).
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = Some(tif);
        self
    }

    /// Sets the client order ID (optional).
    pub fn client_order_id(mut self, id: impl Into<String>) -> Self {
        self.client_order_id = Some(id.into());
        self
    }

    /// Sets the reduce-only flag (optional, for futures).
    pub fn reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = Some(reduce_only);
        self
    }

    /// Sets the post-only flag (optional).
    pub fn post_only(mut self, post_only: bool) -> Self {
        self.post_only = Some(post_only);
        self
    }

    /// Sets the trigger price type (optional).
    ///
    /// Specifies which price source to use for triggering conditional orders.
    pub fn trigger_price_type(mut self, trigger_type: TriggerPriceType) -> Self {
        self.trigger_price_type = Some(trigger_type);
        self
    }

    /// Sets the trailing callback rate using basis points (Binance style).
    ///
    /// # Example
    /// ```rust,ignore
    /// // Binance: 1% = 100 basis points
    /// .trailing_callback_rate_bps(dec!(100))  // 1%
    /// .trailing_callback_rate_bps(dec!(50))   // 0.5%
    /// ```
    pub fn trailing_callback_rate_bps(self, rate: rust_decimal::Decimal) -> Self {
        self.trailing_stop_config(TrailingAmountType::BasisPoints, rate)
    }

    /// Sets the trailing callback rate using percentage (Bybit/Bitget style).
    ///
    /// # Example
    /// ```rust,ignore
    /// // Bybit/Bitget: direct percentage
    /// .trailing_callback_rate_percent(dec!(1.5))  // 1.5%
    /// .trailing_callback_rate_percent(dec!(0.5))  // 0.5%
    /// ```
    pub fn trailing_callback_rate_percent(self, rate: rust_decimal::Decimal) -> Self {
        self.trailing_stop_config(TrailingAmountType::Percent, rate)
    }

    /// Sets trailing stop configuration (recommended for TrailingStop orders).
    ///
    /// This method sets both `trailing_callback_rate` and `trailing_amount_type`
    /// together, ensuring type safety and preventing incomplete configuration.
    ///
    /// # Examples
    /// ```rust,ignore
    /// // Binance style: 1% = 100 basis points
    /// .trailing_stop_config(TrailingAmountType::BasisPoints, dec!(100))
    ///
    /// // Bybit/Bitget style: 1.5%
    /// .trailing_stop_config(TrailingAmountType::Percent, dec!(1.5))
    /// ```
    pub fn trailing_stop_config(
        mut self,
        amount_type: TrailingAmountType,
        callback_rate: rust_decimal::Decimal,
    ) -> Self {
        self.trailing_callback_rate = Some(callback_rate);
        self.trailing_amount_type = Some(amount_type);
        self
    }

    /// Sets trailing stop configuration with activation price (complete setup).
    ///
    /// This is the most convenient method for setting up trailing stops.
    ///
    /// # Examples
    /// ```rust,ignore
    /// // Trailing stop activates at 50000, with 1% callback
    /// .trailing_stop_with_activation(
    ///     dec!(1.5),
    ///     TrailingAmountType::Percent,
    ///     Price::new(dec!(50000))
    /// )
    /// ```
    pub fn trailing_stop_with_activation(
        self,
        callback_rate: rust_decimal::Decimal,
        amount_type: TrailingAmountType,
        activation_price: Price,
    ) -> Self {
        self.trailing_stop_config(amount_type, callback_rate)
            .activation_price(activation_price)
    }

    /// Sets the activation price (optional, for trailing stop orders).
    pub fn activation_price(mut self, activation_price: Price) -> Self {
        self.activation_price = Some(activation_price);
        self
    }

    /// Sets the position side (optional, for hedge mode futures).
    pub fn position_side(mut self, position_side: PositionSide) -> Self {
        self.position_side = Some(position_side);
        self
    }

    /// Set take profit limit price (for limit TP orders).
    #[must_use]
    pub fn tp_limit_price(mut self, price: Price) -> Self {
        self.tp_limit_price = Some(price);
        self
    }

    /// Set stop loss limit price (for limit SL orders).
    #[must_use]
    pub fn sl_limit_price(mut self, price: Price) -> Self {
        self.sl_limit_price = Some(price);
        self
    }

    // =========================================================================
    // Exchange-Specific Parameters
    // =========================================================================

    /// Add exchange-specific parameter.
    ///
    /// # Examples
    /// ```rust,ignore
    /// // Binance: self-trade prevention
    /// .extra("selfTradePreventionMode", json!("EXPIRE_MAKER"))
    ///
    /// // KuCoin: hidden order
    /// .extra("hidden", json!(true))
    ///
    /// // OKX: attach algo orders
    /// .extra("attachAlgoOrds", json!([...]))
    /// ```
    pub fn extra(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value);
        self
    }

    /// Add multiple exchange-specific parameters.
    ///
    /// # Example
    /// ```rust,ignore
    /// .extras(HashMap::from([
    ///     ("selfTradePreventionMode", json!("EXPIRE_MAKER")),
    ///     ("priceMatch", json!("none")),
    /// ]))
    /// ```
    pub fn extras(mut self, params: HashMap<String, Value>) -> Self {
        let extra = self.extra.get_or_insert_with(HashMap::new);
        extra.extend(params);
        self
    }

    // =========================================================================
    // Convenience Methods for Common Order Types
    // =========================================================================

    /// Creates a market order with minimal required fields.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_core::types::order::order_request::OrderRequest;
    /// use ccxt_core::types::{OrderSide, OrderType};
    /// use ccxt_core::types::financial::Amount;
    /// use rust_decimal_macros::dec;
    ///
    /// let request = OrderRequest::builder()
    ///     .market_order("BTC/USDT", OrderSide::Buy, Amount::new(dec!(0.1)));
    /// ```
    pub fn market_order(
        self,
        symbol: impl Into<String>,
        side: OrderSide,
        amount: Amount,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.into()),
            side: Set(side),
            order_type: Set(OrderType::Market),
            amount: Set(AmountSpec::Base(amount)),
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }

    /// Creates a limit order with price.
    pub fn limit_order(
        self,
        symbol: impl Into<String>,
        side: OrderSide,
        amount: Amount,
        price: Price,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.into()),
            side: Set(side),
            order_type: Set(OrderType::Limit),
            amount: Set(AmountSpec::Base(amount)),
            price: Some(price),
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }

    /// Creates a stop loss order.
    pub fn stop_loss(
        self,
        symbol: impl Into<String>,
        side: OrderSide,
        amount: Amount,
        stop_price: Price,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.into()),
            side: Set(side),
            order_type: Set(OrderType::StopLoss),
            amount: Set(AmountSpec::Base(amount)),
            price: self.price,
            stop_price: Some(stop_price),
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }

    /// Creates a take profit order.
    pub fn take_profit(
        self,
        symbol: impl Into<String>,
        side: OrderSide,
        amount: Amount,
        take_profit_price: Price,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.into()),
            side: Set(side),
            order_type: Set(OrderType::TakeProfit),
            amount: Set(AmountSpec::Base(amount)),
            price: self.price,
            stop_price: Some(take_profit_price),
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }

    /// Creates a post-only limit order.
    pub fn post_only_limit(
        self,
        symbol: impl Into<String>,
        side: OrderSide,
        amount: Amount,
        price: Price,
    ) -> OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
        OrderRequestBuilder {
            symbol: Set(symbol.into()),
            side: Set(side),
            order_type: Set(OrderType::Limit),
            amount: Set(AmountSpec::Base(amount)),
            price: Some(price),
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: Some(true),
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            _marker: PhantomData,
            extra: self.extra,
        }
    }
}

// ============================================================================
// Build method (only available when all required fields are set)
// ============================================================================

impl OrderRequestBuilder<Set<String>, Set<OrderSide>, Set<OrderType>, Set<AmountSpec>> {
    /// Builds the final `OrderRequest`.
    ///
    /// This method is only available when all required fields (symbol, side,
    /// `order_type`, amount) have been set.
    ///
    /// # Validation
    /// Returns an error if:
    /// - Limit order without price
    /// - Stop order without stop_price
    /// - TrailingStop without both trailing_callback_rate and trailing_amount_type
    pub fn build(self) -> Result<OrderRequest, OrderValidationError> {
        // Validate limit orders require price
        match self.order_type.0 {
            OrderType::Limit
            | OrderType::LimitMaker
            | OrderType::StopLossLimit
            | OrderType::TakeProfitLimit
            | OrderType::StopLimit => {
                if self.price.is_none() {
                    return Err(OrderValidationError::MissingPriceForLimitOrder);
                }
            }
            _ => {}
        }

        // Validate stop orders require stop_price
        match self.order_type.0 {
            OrderType::StopLoss
            | OrderType::StopLossLimit
            | OrderType::TakeProfit
            | OrderType::TakeProfitLimit
            | OrderType::StopMarket
            | OrderType::StopLimit => {
                if self.stop_price.is_none() {
                    return Err(OrderValidationError::MissingStopPrice);
                }
            }
            _ => {}
        }

        // Validate trailing stop requires both trailing_callback_rate and trailing_amount_type
        if self.order_type.0 == OrderType::TrailingStop {
            match (self.trailing_callback_rate, self.trailing_amount_type) {
                (Some(_), Some(_)) => {} // Both set, OK
                (None, None) => {
                    return Err(OrderValidationError::MissingTrailingCallback);
                }
                _ => {
                    return Err(OrderValidationError::IncompleteTrailingConfig);
                }
            }
        }

        // Validate quote amount is only used for market orders
        if self.amount.0.is_quote() && self.order_type.0 != OrderType::Market {
            return Err(OrderValidationError::QuoteAmountOnlyForMarketOrder);
        }

        // ✅ NEW: Validate configuration conflicts
        // Post-only orders cannot use FOK time in force
        if self.post_only == Some(true) && self.time_in_force == Some(TimeInForce::FOK) {
            return Err(OrderValidationError::ConfigurationConflict {
                field1: "post_only".to_string(),
                field2: "time_in_force".to_string(),
                message: "Post-only orders cannot use FOK (Fill or Kill)".to_string(),
            });
        }

        Ok(OrderRequest {
            symbol: self.symbol.0,
            side: self.side.0,
            order_type: self.order_type.0,
            amount: self.amount.0,
            price: self.price,
            stop_price: self.stop_price,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id,
            reduce_only: self.reduce_only,
            post_only: self.post_only,
            trigger_price_type: self.trigger_price_type,
            trailing_callback_rate: self.trailing_callback_rate,
            trailing_amount_type: self.trailing_amount_type,
            activation_price: self.activation_price,
            position_side: self.position_side,
            tp_limit_price: self.tp_limit_price,
            sl_limit_price: self.sl_limit_price,
            extra: self.extra,
        })
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_order_request_builder_market_order() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .build()
            .expect("Market order should build successfully");

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.side, OrderSide::Buy);
        assert_eq!(request.order_type, OrderType::Market);
        assert_eq!(request.amount.as_decimal(), dec!(0.1));
        assert!(request.price.is_none());
        assert!(request.is_market_order());
    }

    #[test]
    fn test_order_request_builder_limit_order() {
        let request = OrderRequest::builder()
            .symbol("ETH/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(1.5)))
            .price(Price::new(dec!(3000)))
            .build()
            .expect("OrderRequest should build successfully");

        assert_eq!(request.symbol, "ETH/USDT");
        assert_eq!(request.side, OrderSide::Sell);
        assert_eq!(request.order_type, OrderType::Limit);
        assert_eq!(request.amount.as_decimal(), dec!(1.5));
        assert_eq!(request.price.unwrap().as_decimal(), dec!(3000));
        assert!(request.is_limit_order());
    }

    #[test]
    fn test_order_request_builder_with_trigger_price_type() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLoss)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(45000)))
            .trigger_price_type(TriggerPriceType::MarkPrice)
            .build()
            .expect("OrderRequest should build successfully");

        assert_eq!(request.stop_price.unwrap().as_decimal(), dec!(45000));
        assert_eq!(
            request.trigger_price_type,
            Some(TriggerPriceType::MarkPrice)
        );
        assert!(request.is_stop_order());
    }

    #[test]
    fn test_order_request_builder_with_trailing_stop() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.1)))
            .trailing_callback_rate_percent(dec!(1.5))
            .activation_price(Price::new(dec!(52000)))
            .build()
            .expect("OrderRequest should build successfully");

        assert_eq!(request.order_type, OrderType::TrailingStop);
        assert_eq!(request.trailing_callback_rate, Some(dec!(1.5)));
        assert_eq!(
            request.trailing_amount_type,
            Some(TrailingAmountType::Percent)
        );
        assert_eq!(request.activation_price.unwrap().as_decimal(), dec!(52000));
        assert!(request.is_trailing_stop_order());
    }

    #[test]
    fn test_order_request_builder_convenience_methods() {
        // Test market_order convenience method
        let market = OrderRequest::builder().market_order(
            "BTC/USDT",
            OrderSide::Buy,
            Amount::new(dec!(0.1)),
        );
        let request = market
            .build()
            .expect("OrderRequest should build successfully");
        assert_eq!(request.order_type, OrderType::Market);

        // Test limit_order convenience method
        let limit = OrderRequest::builder().limit_order(
            "ETH/USDT",
            OrderSide::Sell,
            Amount::new(dec!(1.0)),
            Price::new(dec!(3000)),
        );
        let request = limit
            .build()
            .expect("OrderRequest should build successfully");
        assert_eq!(request.order_type, OrderType::Limit);
        assert_eq!(request.price.unwrap().as_decimal(), dec!(3000));

        // Test stop_loss convenience method
        let stop = OrderRequest::builder().stop_loss(
            "BTC/USDT",
            OrderSide::Sell,
            Amount::new(dec!(0.1)),
            Price::new(dec!(45000)),
        );
        let request = stop
            .build()
            .expect("OrderRequest should build successfully");
        assert_eq!(request.order_type, OrderType::StopLoss);
        assert_eq!(request.stop_price.unwrap().as_decimal(), dec!(45000));
    }

    #[test]
    fn test_order_request_builder_post_only_limit() {
        let request = OrderRequest::builder()
            .post_only_limit(
                "BTC/USDT",
                OrderSide::Buy,
                Amount::new(dec!(0.1)),
                Price::new(dec!(50000)),
            )
            .build()
            .expect("OrderRequest should build successfully");

        assert_eq!(request.order_type, OrderType::Limit);
        assert_eq!(request.post_only, Some(true));
        assert_eq!(request.price.unwrap().as_decimal(), dec!(50000));
    }

    #[test]
    fn test_order_request_builder_with_all_fields() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .stop_price(Price::new(dec!(49000)))
            .time_in_force(TimeInForce::GTC)
            .client_order_id("my-order-123")
            .reduce_only(false)
            .post_only(true)
            .trigger_price_type(TriggerPriceType::MarkPrice)
            .trailing_callback_rate_bps(dec!(100))
            .activation_price(Price::new(dec!(52000)))
            .position_side(PositionSide::Long)
            .tp_limit_price(Price::new(dec!(55000)))
            .sl_limit_price(Price::new(dec!(48000)))
            .build()
            .expect("OrderRequest should build successfully");

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.price.unwrap().as_decimal(), dec!(50000));
        assert_eq!(request.stop_price.unwrap().as_decimal(), dec!(49000));
        assert_eq!(request.time_in_force, Some(TimeInForce::GTC));
        assert_eq!(request.client_order_id, Some("my-order-123".to_string()));
        assert_eq!(request.reduce_only, Some(false));
        assert_eq!(request.post_only, Some(true));
        assert_eq!(
            request.trigger_price_type,
            Some(TriggerPriceType::MarkPrice)
        );
        assert_eq!(request.trailing_callback_rate, Some(dec!(100)));
        assert_eq!(
            request.trailing_amount_type,
            Some(TrailingAmountType::BasisPoints)
        );
        assert_eq!(request.activation_price.unwrap().as_decimal(), dec!(52000));
        assert_eq!(request.position_side, Some(PositionSide::Long));
        assert_eq!(request.tp_limit_price.unwrap().as_decimal(), dec!(55000));
        assert_eq!(request.sl_limit_price.unwrap().as_decimal(), dec!(48000));
    }

    #[test]
    fn test_order_request_default_optional_fields() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .build()
            .expect("OrderRequest should build successfully");

        // All optional fields should be None by default
        assert!(request.price.is_none());
        assert!(request.stop_price.is_none());
        assert!(request.time_in_force.is_none());
        assert!(request.client_order_id.is_none());
        assert!(request.reduce_only.is_none());
        assert!(request.post_only.is_none());
        assert!(request.trigger_price_type.is_none());
        assert!(request.trailing_callback_rate.is_none());
        assert!(request.trailing_amount_type.is_none());
        assert!(request.activation_price.is_none());
        assert!(request.position_side.is_none());
        assert!(request.tp_limit_price.is_none());
        assert!(request.sl_limit_price.is_none());
    }

    #[test]
    fn test_trigger_price_type_enum() {
        assert_eq!(TriggerPriceType::LastPrice.as_str(), "last_price");
        assert_eq!(TriggerPriceType::MarkPrice.as_str(), "mark_price");
        assert_eq!(TriggerPriceType::IndexPrice.as_str(), "index_price");

        assert_eq!(TriggerPriceType::LastPrice.to_string(), "last_price");
    }

    #[test]
    fn test_trailing_amount_type_enum() {
        assert_eq!(TrailingAmountType::BasisPoints.as_str(), "basis_points");
        assert_eq!(TrailingAmountType::Percent.as_str(), "percent");

        assert_eq!(TrailingAmountType::Percent.to_string(), "percent");
    }

    #[test]
    fn test_order_request_is_methods() {
        let market = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .build()
            .expect("OrderRequest should build successfully");
        assert!(market.is_market_order());
        assert!(!market.is_limit_order());

        let limit = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .build()
            .expect("OrderRequest should build successfully");
        assert!(limit.is_limit_order());
        assert!(!limit.is_market_order());

        let limit_maker = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::LimitMaker)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .build()
            .expect("OrderRequest should build successfully");
        assert!(limit_maker.is_limit_order());

        let stop_loss = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLoss)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(45000)))
            .build()
            .expect("OrderRequest should build successfully");
        assert!(stop_loss.is_stop_order());

        let take_profit = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TakeProfit)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(55000)))
            .build()
            .expect("OrderRequest should build successfully");
        assert!(take_profit.is_take_profit_order());

        let trailing = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.1)))
            .trailing_stop_config(TrailingAmountType::Percent, dec!(1.5))
            .build()
            .expect("OrderRequest should build successfully");
        assert!(trailing.is_trailing_stop_order());
    }

    #[test]
    fn test_order_request_serialization() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .trigger_price_type(TriggerPriceType::MarkPrice)
            .build()
            .expect("OrderRequest should build successfully");

        // Test serialization
        let json = serde_json::to_string(&request).expect("Serialization should succeed");
        assert!(json.contains("BTC/USDT"));
        assert!(json.contains("buy"));
        assert!(json.contains("limit"));
        assert!(json.contains("mark_price"));

        // Test deserialization
        let deserialized: OrderRequest =
            serde_json::from_str(&json).expect("Deserialization should succeed");
        assert_eq!(deserialized.symbol, request.symbol);
        assert_eq!(deserialized.side, request.side);
        assert_eq!(deserialized.order_type, request.order_type);
        assert_eq!(deserialized.trigger_price_type, request.trigger_price_type);
    }

    #[test]
    fn test_build_validation_limit_order_requires_price() {
        // Limit order without price should fail
        let result = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderValidationError::MissingPriceForLimitOrder
        ));
    }

    #[test]
    fn test_build_validation_stop_order_requires_stop_price() {
        // Stop order without stop_price should fail
        let result = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLoss)
            .amount(Amount::new(dec!(0.1)))
            .build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderValidationError::MissingStopPrice
        ));
    }

    #[test]
    fn test_build_validation_trailing_stop_requires_both_fields() {
        // TrailingStop without trailing config should fail
        let result = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.1)))
            .build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderValidationError::MissingTrailingCallback
        ));

        // Note: trailing_callback_rate_bps/percent now set both fields together,
        // so we can't test incomplete config with the new API.
        // The trailing_stop_config() method ensures both fields are always set.
    }

    #[test]
    fn test_build_validation_stop_limit_requires_price() {
        // StopLimit order without price should fail
        let result = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLimit)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(45000)))
            .build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderValidationError::MissingPriceForLimitOrder
        ));

        // StopLimit with both stop_price and price should succeed
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLimit)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(45000)))
            .price(Price::new(dec!(44500)))
            .build()
            .expect("StopLimit with both prices should build successfully");

        assert_eq!(request.order_type, OrderType::StopLimit);
        assert_eq!(request.stop_price, Some(Price::new(dec!(45000))));
        assert_eq!(request.price, Some(Price::new(dec!(44500))));
    }

    #[test]
    fn test_build_validation_quote_amount_only_for_market() {
        // Quote amount with market order should succeed
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .quote_amount(dec!(1000))
            .build()
            .expect("Quote amount with market order should build successfully");

        assert!(request.amount.is_quote());

        // Quote amount with limit order should fail
        let result = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .quote_amount(dec!(1000))
            .price(Price::new(dec!(50000)))
            .build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderValidationError::QuoteAmountOnlyForMarketOrder
        ));

        // Quote amount with stop loss should fail
        let result = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLoss)
            .quote_amount(dec!(1000))
            .stop_price(Price::new(dec!(45000)))
            .build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OrderValidationError::QuoteAmountOnlyForMarketOrder
        ));
    }

    #[test]
    fn test_amount_spec_base_and_quote() {
        // Test base amount
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .base_amount(dec!(0.1))
            .build()
            .expect("Should build successfully");

        assert!(request.amount.is_base());
        assert_eq!(request.amount.as_decimal(), dec!(0.1));

        // Test quote amount
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .quote_amount(dec!(1000))
            .build()
            .expect("Should build successfully");

        assert!(request.amount.is_quote());
        assert_eq!(request.amount.as_decimal(), dec!(1000));
    }

    #[test]
    fn test_trailing_stop_config_convenience_method() {
        // Test trailing_stop_config
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.1)))
            .trailing_stop_config(TrailingAmountType::BasisPoints, dec!(100))
            .build()
            .expect("Should build successfully");

        assert_eq!(request.trailing_callback_rate, Some(dec!(100)));
        assert_eq!(
            request.trailing_amount_type,
            Some(TrailingAmountType::BasisPoints)
        );

        // Test trailing_stop_with_activation
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.1)))
            .trailing_stop_with_activation(
                dec!(1.5),
                TrailingAmountType::Percent,
                Price::new(dec!(50000)),
            )
            .build()
            .expect("Should build successfully");

        assert_eq!(request.trailing_callback_rate, Some(dec!(1.5)));
        assert_eq!(
            request.trailing_amount_type,
            Some(TrailingAmountType::Percent)
        );
        assert_eq!(request.activation_price, Some(Price::new(dec!(50000))));
    }

    #[test]
    fn test_extra_field() {
        use serde_json::json;

        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .extra("selfTradePreventionMode", json!("EXPIRE_MAKER"))
            .extra("priceMatch", json!("none"))
            .build()
            .expect("Should build successfully");

        assert!(request.extra.is_some());
        let extra = request.extra.unwrap();
        assert_eq!(
            extra.get("selfTradePreventionMode").unwrap(),
            &json!("EXPIRE_MAKER")
        );
        assert_eq!(extra.get("priceMatch").unwrap(), &json!("none"));
    }

    #[test]
    fn test_convenience_methods_market_buy() {
        let request = OrderRequestBuilder::market_buy("BTC/USDT", dec!(0.1))
            .build()
            .expect("Should build successfully");

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.side, OrderSide::Buy);
        assert_eq!(request.order_type, OrderType::Market);
        assert!(request.amount.is_base());
        assert_eq!(request.amount.as_decimal(), dec!(0.1));
        assert!(request.price.is_none());
    }

    #[test]
    fn test_convenience_methods_market_buy_quote() {
        let request = OrderRequestBuilder::market_buy_quote("BTC/USDT", dec!(1000))
            .build()
            .expect("Should build successfully");

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.side, OrderSide::Buy);
        assert_eq!(request.order_type, OrderType::Market);
        assert!(request.amount.is_quote());
        assert_eq!(request.amount.as_decimal(), dec!(1000));
        assert!(request.price.is_none());
    }

    #[test]
    fn test_convenience_methods_market_sell() {
        let request = OrderRequestBuilder::market_sell("BTC/USDT", dec!(0.5))
            .build()
            .expect("Should build successfully");

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.side, OrderSide::Sell);
        assert_eq!(request.order_type, OrderType::Market);
        assert!(request.amount.is_base());
        assert_eq!(request.amount.as_decimal(), dec!(0.5));
        assert!(request.price.is_none());
    }

    #[test]
    fn test_convenience_methods_limit_buy() {
        let request = OrderRequestBuilder::limit_buy("BTC/USDT", dec!(0.1), dec!(50000))
            .build()
            .expect("Should build successfully");

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.side, OrderSide::Buy);
        assert_eq!(request.order_type, OrderType::Limit);
        assert!(request.amount.is_base());
        assert_eq!(request.amount.as_decimal(), dec!(0.1));
        assert_eq!(request.price, Some(Price::new(dec!(50000))));
        assert_eq!(request.time_in_force, Some(TimeInForce::GTC));
    }

    #[test]
    fn test_convenience_methods_limit_sell() {
        let request = OrderRequestBuilder::limit_sell("BTC/USDT", dec!(0.2), dec!(55000))
            .build()
            .expect("Should build successfully");

        assert_eq!(request.symbol, "BTC/USDT");
        assert_eq!(request.side, OrderSide::Sell);
        assert_eq!(request.order_type, OrderType::Limit);
        assert!(request.amount.is_base());
        assert_eq!(request.amount.as_decimal(), dec!(0.2));
        assert_eq!(request.price, Some(Price::new(dec!(55000))));
        assert_eq!(request.time_in_force, Some(TimeInForce::GTC));
    }

    #[test]
    fn test_convenience_methods_can_add_optional_fields() {
        let request = OrderRequestBuilder::market_buy("BTC/USDT", dec!(0.1))
            .client_order_id("my-order-123".to_string())
            .reduce_only(true)
            .build()
            .expect("Should build successfully");

        assert_eq!(request.client_order_id, Some("my-order-123".to_string()));
        assert_eq!(request.reduce_only, Some(true));
    }
}

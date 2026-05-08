//! CCXT Exchange Implementations
//!
//! This library contains concrete implementations of cryptocurrency exchanges
//! built on top of ccxt-core.
//!
//! # Supported Exchanges
//!
//! - Binance ✅
//! - OKX ✅
//! - Bybit ✅
//! - Bitget ✅
//! - HyperLiquid ✅
//! - Gate.io ✅
//!
//! # Features
//!
//! This crate uses feature flags to control which exchanges are compiled:
//!
//! - `binance` - Enable Binance exchange (enabled by default)
//! - `okx` - Enable OKX exchange (enabled by default)
//! - `bybit` - Enable Bybit exchange (enabled by default)
//! - `bitget` - Enable Bitget exchange (enabled by default)
//! - `hyperliquid` - Enable HyperLiquid exchange (enabled by default)
//! - `gate` - Enable Gate.io exchange (enabled by default)
//! - `websocket` - Enable WebSocket support (enabled by default)
//! - `full` - Enable all exchanges and WebSocket
//! - `market-data-only` - Enable all exchanges without WebSocket
//!
//! # Example
//!
//! ```rust,no_run
//! use ccxt_exchanges::binance::Binance;
//!
//! # async fn example() -> Result<(), ccxt_core::Error> {
//! let exchange = Binance::builder()
//!     .api_key("your_api_key")
//!     .secret("your_secret")
//!     .build()?;
//!
//! let markets = exchange.fetch_markets().await?;
//! println!("Found {} markets", markets.len());
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
// =============================================================================
// Global Clippy Lint Suppressions
// =============================================================================
// REVIEW NOTE: 全局抑制规则过多（共18个），建议逐步移除并修复相关问题
// 优先级: 高
// 与ccxt-core相比，此处的抑制规则更多，应该优先清理
//
// 特别关注的问题：
// - cast_* 系列抑制了5个规则，建议统一使用try_into()
// - needless_pass_by_value: 建议考虑使用引用传递
// - collapsible_if: 建议合并可折叠的if语句
// =============================================================================
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)] // REVIEW: 建议移除
#![allow(clippy::missing_panics_doc)] // REVIEW: 建议移除
#![allow(clippy::must_use_candidate)] // REVIEW: 建议移除
#![allow(clippy::doc_markdown)] // REVIEW: 建议移除
#![allow(clippy::similar_names)]
#![allow(clippy::uninlined_format_args)] // REVIEW: 建议移除，使用内联格式参数
#![allow(clippy::cast_possible_truncation)] // REVIEW: 建议使用try_into()
#![allow(clippy::cast_sign_loss)] // REVIEW: 建议使用try_into()
#![allow(clippy::cast_possible_wrap)] // REVIEW: 建议使用try_into()
#![allow(clippy::cast_lossless)] // REVIEW: 建议显式转换
#![allow(clippy::cast_precision_loss)] // REVIEW: 建议使用Decimal避免精度损失
#![allow(clippy::struct_excessive_bools)] // REVIEW: 考虑重构为枚举
#![allow(clippy::too_many_lines)] // REVIEW: 建议拆分函数
#![allow(clippy::return_self_not_must_use)] // REVIEW: 建议移除
#![allow(clippy::unreadable_literal)] // REVIEW: 建议使用1_000_000格式
#![allow(clippy::needless_pass_by_value)] // REVIEW: 建议使用&str替代String
#![allow(clippy::redundant_closure)] // REVIEW: 建议使用函数指针
#![allow(clippy::collapsible_if)] // REVIEW: 建议合并if语句

// Re-export ccxt-core
pub use ccxt_core;

/// Common shared building blocks for exchange implementations
pub mod common;

/// Binance exchange implementation
#[cfg(feature = "binance")]
pub mod binance;

/// Bitget exchange implementation
#[cfg(feature = "bitget")]
pub mod bitget;

/// Bybit exchange implementation
#[cfg(feature = "bybit")]
pub mod bybit;

/// HyperLiquid exchange implementation
#[cfg(feature = "hyperliquid")]
pub mod hyperliquid;

/// Gate.io exchange implementation
#[cfg(feature = "gate")]
pub mod gate;

/// OKX exchange implementation
#[cfg(feature = "okx")]
pub mod okx;

/// Prelude module for convenient imports
pub mod prelude {
    pub use ccxt_core::prelude::*;
    pub use ccxt_core::{ArcExchange, BoxedExchange, Exchange, ExchangeCapabilities};
    pub use ccxt_core::{FullExchange, MessageStream, WsExchange};

    #[cfg(feature = "binance")]
    pub use crate::binance::{Binance, BinanceBuilder};

    #[cfg(feature = "bitget")]
    pub use crate::bitget::{Bitget, BitgetBuilder};

    #[cfg(feature = "bybit")]
    pub use crate::bybit::{Bybit, BybitBuilder};

    #[cfg(feature = "hyperliquid")]
    pub use crate::hyperliquid::{HyperLiquid, HyperLiquidBuilder};

    #[cfg(feature = "okx")]
    pub use crate::okx::{Okx, OkxBuilder};
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

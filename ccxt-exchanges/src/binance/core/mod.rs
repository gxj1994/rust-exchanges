//! Binance core modules.
//!
//! This module contains core definitions and configuration:
//! - Exchange options and settings
//! - Unified endpoint management
//! - Constants and timeframes
//! - Symbol conversion
//! - Error handling

pub mod builder;
pub mod constants;
pub mod endpoints;
pub mod error;
pub mod options;
pub mod symbol;

// Re-export main types for convenience
pub use builder::BinanceBuilder;
pub use constants::timeframes;
pub use options::BinanceOptions;
pub use symbol::BinanceSymbolConverter;

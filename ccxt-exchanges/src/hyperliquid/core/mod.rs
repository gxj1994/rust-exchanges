//! HyperLiquid core module.
//!
//! This module contains core HyperLiquid components including
//! the builder pattern for constructing exchange instances, unified endpoint management, and error handling.

pub mod builder;
pub mod endpoints;
pub mod error;
pub mod symbol;

pub use builder::{HyperLiquidBuilder, validate_default_type};
pub use error::{HyperLiquidErrorCode, is_error_response, parse_error};
pub use symbol::HyperliquidSymbolConverter;

//! Bybit core module.
//!
//! This module contains core Bybit components including
//! builder pattern, symbol conversion utilities, unified endpoint management, and error handling.

pub mod builder;
pub mod endpoints;
pub mod error;
pub mod symbol;

pub use builder::BybitBuilder;
pub use error::{BybitErrorCode, is_error_response, parse_error};
pub use symbol::BybitSymbolConverter;

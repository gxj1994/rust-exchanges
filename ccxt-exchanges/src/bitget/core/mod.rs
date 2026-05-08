//! Bitget core module.
//!
//! This module contains core Bitget components including
//! builder pattern, symbol conversion utilities, unified endpoint management, and error handling.

pub mod builder;
pub mod endpoints;
pub mod error;
pub mod symbol;

pub use builder::BitgetBuilder;
pub use error::{BitgetErrorCode, is_error_response, parse_error};
pub use symbol::BitgetSymbolConverter;

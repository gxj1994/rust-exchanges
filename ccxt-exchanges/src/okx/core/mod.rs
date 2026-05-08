//! OKX core module.
//!
//! This module contains core OKX components including
//! builder pattern, symbol conversion utilities, unified endpoint management, and error handling.

pub mod builder;
pub mod endpoints;
pub mod error;
pub mod symbol;

pub use builder::OkxBuilder;
pub use error::{OkxErrorCode, is_error_response, parse_error};
pub use symbol::OkxSymbolConverter;

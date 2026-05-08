//! Bybit authentication and signing modules.
//!
//! This module contains all authentication-related functionality:
//! - HMAC-SHA256 signature generation
//! - Signed request building
//! - WebSocket authentication strategy

mod core;
pub mod signed_request;
pub mod ws_auth;

pub use core::BybitAuth;
pub use signed_request::BybitSignedRequestBuilder;
pub use signed_request::HttpMethod;
pub use ws_auth::BybitWsAuth;

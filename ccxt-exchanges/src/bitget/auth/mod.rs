//! Bitget authentication and signing modules.
//!
//! This module contains all authentication-related functionality:
//! - HMAC-SHA256 signature generation (Base64 encoded)
//! - Signed request building
//! - WebSocket authentication strategy

mod core;
pub mod signed_request;
pub mod ws_auth;

pub use core::BitgetAuth;
pub use signed_request::BitgetSignedRequestBuilder;
pub use signed_request::HttpMethod;
pub use ws_auth::BitgetWsAuth;

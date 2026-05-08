//! OKX authentication and signing modules.
//!
//! This module contains all authentication-related functionality:
//! - HMAC-SHA256 signature generation for OKX API
//! - Signed request building
//! - WebSocket authentication strategy

mod core;
pub mod signed_request;
pub mod ws_auth;

// Re-export main types for convenience
pub use core::OkxAuth;
pub use signed_request::{HttpMethod, OkxSignedRequestBuilder};
pub use ws_auth::OkxWsAuth;

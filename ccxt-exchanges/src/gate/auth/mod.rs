//! Gate.io authentication module
//!
//! Contains HMAC-SHA512 signature implementation for Gate.io API
//! and signed request builder for authenticated API calls.

pub mod core;
pub mod signed_request;
pub mod ws_auth;

pub use ws_auth::GateWsAuth;

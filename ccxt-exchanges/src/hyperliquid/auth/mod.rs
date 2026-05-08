//! HyperLiquid authentication and signing modules.
//!
//! This module contains all authentication-related functionality:
//! - EIP-712 signature generation for Ethereum wallet authentication
//! - Signed request building
//! - WebSocket authentication strategy

mod core;
pub mod signed_request;
pub mod ws_auth;

// Re-export main types for convenience
pub use core::HyperLiquidAuth;
pub use signed_request::HyperliquidSignedRequestBuilder;
pub use ws_auth::HyperliquidWsAuth;

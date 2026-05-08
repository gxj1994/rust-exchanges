//! Binance authentication and signing modules.
//!
//! This module contains all authentication-related functionality:
//! - HMAC-SHA256 signature generation
//! - Signed request building
//! - Signing strategy implementation
//! - WebSocket authentication strategy (listenKey management)

mod core;
pub mod signed_request;
pub mod signing_strategy;
pub mod ws_auth;

// Re-export main types for convenience
pub use core::BinanceAuth;
pub use signed_request::{HttpMethod, SignedRequestBuilder};
pub use signing_strategy::BinanceSigningStrategy;
pub use ws_auth::{BinanceWsAuth, BinanceWsMarket};

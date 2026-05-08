//! Binance network-related modules.
//!
//! This module contains all network and communication related functionality:
//! - Rate limiting for API requests
//! - Time synchronization with Binance servers  
//! - Endpoint routing for different market types

pub mod endpoint_router;
pub mod rate_limiter;
pub mod time_sync;

// Re-export main types for convenience
pub use rate_limiter::WeightRateLimiter;
pub use time_sync::{TimeSyncConfig, TimeSyncManager};

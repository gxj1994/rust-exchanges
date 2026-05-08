//! Gate.io contract (swap/futures) REST API implementation.
//!
//! This module provides contract trading functionality including:
//! - Contract market data
//! - Contract order management
//! - Contract account operations

pub mod account;
pub mod market_data;
pub mod trading;
// TODO: Add trading and account modules when implemented

use super::Gate;

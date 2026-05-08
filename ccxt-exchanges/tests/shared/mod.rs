//! Shared tests module for cross-exchange tests
//!
//! This module contains tests that apply to multiple exchanges:
//! - property: Property-based tests (symbol converter, time sync, sandbox/testnet)
//! - integration: Integration tests (logging, orderbook resync)
//! - stress: Stress tests

pub mod integration;
pub mod property;
pub mod stress;

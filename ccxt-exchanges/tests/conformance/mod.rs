//! Shared conformance tests for exchange implementations.
//!
//! This module provides a lightweight registration-based harness so exchanges
//! can be validated against the same contract as they migrate toward the
//! capability-based architecture.

pub mod binance;
pub mod bitget;
pub mod bybit;
pub mod capability_conformance;
pub mod gate;
pub mod okx;

// 重新导出主要类型和函数
pub use capability_conformance::*;

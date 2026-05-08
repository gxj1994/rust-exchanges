//! Gate.io Account trait implementation.
//!
//! This module implements the `Account` trait from `ccxt-core` for Gate.io,
//! providing account balance and asset management capabilities.
//!
//! # Supported Operations
//!
//! - Fetch account balance (spot)
//! - Fetch account balance (futures/swap)
//! - Query account information

use async_trait::async_trait;
use ccxt_core::{Result, traits::Account, types::Balance};

use crate::gate::Gate;

#[async_trait]
impl Account for Gate {
    async fn fetch_balance(&self) -> Result<Balance> {
        Gate::fetch_balance(self).await
    }

    async fn fetch_balance_with_params(
        &self,
        _params: ccxt_core::types::trading::params::BalanceParams,
    ) -> Result<Balance> {
        Gate::fetch_balance(self).await
    }

    async fn fetch_account_trades_since(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<ccxt_core::types::Trade>> {
        // Call internal method to avoid trait method name collision
        self.fetch_account_trades_internal(symbol, since, limit)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::GateBuilder;

    #[test]
    fn test_gate_account_trait_object_safety() {
        let gate = GateBuilder::default().build().unwrap();
        let account: Box<dyn Account> = Box::new(gate);

        assert_eq!(account.id(), "gate");
        // Account capabilities will be enabled when methods are implemented
    }
}

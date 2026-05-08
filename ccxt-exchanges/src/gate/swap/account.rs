//! Gate.io contract (swap/futures) account operations.
//!
//! This module contains contract account management methods.

use crate::gate::parser;
use ccxt_core::{Error, ParseError, Result, types::Balance};

use super::Gate;

impl Gate {
    /// Fetch contract account balance.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/accounts`
    pub async fn fetch_contract_balance(&self) -> Result<Balance> {
        let settle = self.get_contract_settle();

        let url = format!("/api/v4/futures/{}/accounts", settle);
        let response = self.signed_request(&url).execute().await?;

        parser::parse_futures_balance(&response, &settle)
    }

    /// Fetch contract positions.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/positions`
    pub async fn fetch_contract_positions(&self) -> Result<Vec<ccxt_core::types::Position>> {
        let settle = self.get_contract_settle();

        let url = format!("/api/v4/futures/{}/positions", settle);
        let response = self.signed_request(&url).execute().await?;

        let positions_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        positions_array
            .iter()
            .map(|pos_data| parser::parse_contract_position(pos_data, settle))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::gate::GateBuilder;

    #[test]
    fn test_contract_account_methods_exist() {
        let gate = GateBuilder::default().build().unwrap();

        // Verify methods exist (compile-time check)
        let _ = async {
            let _ = gate.fetch_contract_balance().await;
            let _ = gate.fetch_contract_positions().await;
        };
    }
}

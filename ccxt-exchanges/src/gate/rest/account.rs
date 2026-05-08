//! Gate.io account operations.
//!
//! This module contains account-related methods including balance queries.

use crate::gate::parser;
use ccxt_core::{Error, ParseError, Result, traits::MarketData};

use super::Gate;

impl Gate {
    /// Fetch account balance.
    ///
    /// # Returns
    ///
    /// Returns the account balance with all currency balances.
    pub async fn fetch_balance(&self) -> Result<ccxt_core::types::Balance> {
        let response = self
            .signed_request("/api/v4/spot/accounts")
            .execute()
            .await?;

        parser::parse_balance(&response)
    }

    /// Fetch personal trade history.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/spot/my_trades?currency_pair=BTC_USDT`
    pub async fn fetch_account_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<ccxt_core::types::Trade>> {
        // Internal implementation to avoid trait method name collision
        self.fetch_account_trades_internal(symbol, since, limit)
            .await
    }

    /// Internal implementation of fetch_account_trades to avoid name collision with Account trait.
    pub(crate) async fn fetch_account_trades_internal(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<ccxt_core::types::Trade>> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))?;
        let currency_pair = Gate::to_exchange_symbol(&market.id);

        let mut builder = self
            .signed_request("/api/v4/spot/my_trades")
            .param("currency_pair", currency_pair);

        if let Some(s) = since {
            builder = builder.param("from", (s / 1000).to_string()); // Convert ms to seconds
        }

        if let Some(l) = limit {
            builder = builder.param("limit", l.to_string());
        }

        let response = builder.execute().await?;

        let trades_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        trades_array
            .iter()
            .map(|trade_data| parser::parse_trade(trade_data, Some(market)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::gate::GateBuilder;

    #[test]
    fn test_account_trait_object_safety() {
        let gate = GateBuilder::default().build().unwrap();
        let _: &dyn ccxt_core::traits::Account = &gate;
    }
}

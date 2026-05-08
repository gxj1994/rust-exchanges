use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Account,
    types::{Balance, Trade, trading::params::BalanceParams},
};

use crate::bybit::Bybit;

#[async_trait]
impl Account for Bybit {
    async fn fetch_balance_with_params(&self, params: BalanceParams) -> Result<Balance> {
        let mut balance = Bybit::fetch_balance(self).await?;

        if let Some(currencies) = params.currencies {
            balance
                .balances
                .retain(|code, _| currencies.iter().any(|currency| currency == code));
        }

        Ok(balance)
    }

    async fn fetch_account_trades_since(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        Bybit::fetch_account_trades(self, symbol, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_bybit_account_trait_object_safety() {
        let bybit = Bybit::new(ExchangeConfig::default()).unwrap();
        let account: Box<dyn Account> = Box::new(bybit);

        assert_eq!(account.id(), "bybit");
        assert!(account.capabilities().fetch_balance());
        assert!(account.capabilities().fetch_account_trades());
    }
}

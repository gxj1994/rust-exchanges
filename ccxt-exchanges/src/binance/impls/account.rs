use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Account,
    types::{Balance, trading::params::BalanceParams},
};

use crate::binance::Binance;

#[async_trait]
impl Account for Binance {
    async fn fetch_balance_with_params(&self, params: BalanceParams) -> Result<Balance> {
        let mut balance = Binance::fetch_balance(self, params.account_type).await?;

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
    ) -> Result<Vec<ccxt_core::Trade>> {
        Binance::fetch_account_trades(self, symbol, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_binance_account_trait_object_safety() {
        let binance = Binance::new(ExchangeConfig::default()).unwrap();
        let account: Box<dyn Account> = Box::new(binance);

        assert_eq!(account.id(), "binance");
        assert!(account.capabilities().fetch_balance());
        assert!(account.capabilities().fetch_account_trades());
    }
}

use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Account,
    types::{Balance, Trade, trading::params::BalanceParams},
};

use crate::okx::Okx;

#[async_trait]
impl Account for Okx {
    async fn fetch_balance_with_params(&self, params: BalanceParams) -> Result<Balance> {
        let mut balance = Okx::fetch_balance(self).await?;

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
        Okx::fetch_account_trades(self, symbol, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_okx_account_trait_object_safety() {
        let okx = Okx::new(ExchangeConfig::default()).unwrap();
        let account: Box<dyn Account> = Box::new(okx);

        assert_eq!(account.id(), "okx");
        assert!(account.capabilities().fetch_balance());
        assert!(account.capabilities().fetch_account_trades());
    }
}

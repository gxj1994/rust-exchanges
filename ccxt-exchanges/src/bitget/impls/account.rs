use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Account,
    types::{Balance, Trade, trading::params::BalanceParams},
};

use crate::bitget::Bitget;

#[async_trait]
impl Account for Bitget {
    async fn fetch_balance_with_params(&self, params: BalanceParams) -> Result<Balance> {
        let mut balance = Bitget::fetch_balance(self).await?;

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
        Bitget::fetch_account_trades(self, symbol, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_bitget_account_trait_object_safety() {
        let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
        let account: Box<dyn Account> = Box::new(bitget);

        assert_eq!(account.id(), "bitget");
        assert!(account.capabilities().fetch_balance());
        assert!(account.capabilities().fetch_account_trades());
    }
}

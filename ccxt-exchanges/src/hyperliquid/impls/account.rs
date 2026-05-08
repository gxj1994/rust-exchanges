use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::Account,
    types::{Balance, Trade, trading::params::BalanceParams},
};

use crate::hyperliquid::HyperLiquid;

#[async_trait]
impl Account for HyperLiquid {
    async fn fetch_balance_with_params(&self, params: BalanceParams) -> Result<Balance> {
        let mut balance = HyperLiquid::fetch_balance(self).await?;

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
        HyperLiquid::fetch_account_trades(self, symbol, since, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperliquid::HyperLiquidOptions;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_hyperliquid_account_trait_object_safety() {
        let options = HyperLiquidOptions::default();
        let hyperliquid =
            HyperLiquid::new_with_options(ExchangeConfig::default(), options, None).unwrap();
        let account: Box<dyn Account> = Box::new(hyperliquid);

        assert_eq!(account.id(), "hyperliquid");
        assert!(account.capabilities().fetch_balance());
    }
}

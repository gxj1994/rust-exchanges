use async_trait::async_trait;
use ccxt_core::{
    Result, Trade,
    traits::Trading,
    types::{Order, OrderRequest},
};

use crate::bitget::Bitget;

#[async_trait]
impl Trading for Bitget {
    async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        Bitget::create_order(self, request).await
    }

    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        Bitget::cancel_order(self, id, symbol).await
    }

    async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        Bitget::cancel_all_orders(self, symbol).await
    }

    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        Bitget::fetch_order(self, id, symbol).await
    }

    async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        Bitget::fetch_open_orders(self, symbol, None, None).await
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        Bitget::fetch_history_orders(self, symbol, since, limit).await
    }

    async fn fetch_trades_with_limit(
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
    fn test_bitget_trading_trait_object_safety() {
        let bitget = Bitget::new(ExchangeConfig::default()).unwrap();
        let trading: Box<dyn Trading> = Box::new(bitget);

        assert_eq!(trading.id(), "bitget");
        assert!(trading.capabilities().create_order());
    }
}

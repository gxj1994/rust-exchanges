use async_trait::async_trait;
use ccxt_core::{
    Result, Trade,
    traits::Trading,
    types::{Order, OrderRequest},
};

use crate::bybit::Bybit;

#[async_trait]
impl Trading for Bybit {
    async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        Bybit::create_order(self, request).await
    }

    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        Bybit::cancel_order(self, id, symbol).await
    }

    async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        Bybit::cancel_all_orders(self, symbol).await
    }

    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        Bybit::fetch_order(self, id, symbol).await
    }

    async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        Bybit::fetch_open_orders(self, symbol, None, None).await
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        Bybit::fetch_history_orders(self, symbol, since, limit).await
    }

    async fn fetch_trades_with_limit(
        &self,
        symbol: &str,
        _since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        Bybit::fetch_market_trades(self, symbol, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_bybit_trading_trait_object_safety() {
        let bybit = Bybit::new(ExchangeConfig::default()).unwrap();
        let trading: Box<dyn Trading> = Box::new(bybit);

        assert_eq!(trading.id(), "bybit");
        assert!(trading.capabilities().create_order());
    }
}

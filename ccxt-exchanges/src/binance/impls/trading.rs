use async_trait::async_trait;
use ccxt_core::{
    Result, Trade,
    traits::Trading,
    types::{Order, OrderRequest},
};

use crate::binance::Binance;

#[async_trait]
impl Trading for Binance {
    async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        Binance::create_order(self, request).await
    }

    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        Binance::cancel_order(self, id, symbol).await
    }

    async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        Binance::cancel_all_orders(self, symbol).await
    }

    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        Binance::fetch_order(self, id, symbol).await
    }

    async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        Binance::fetch_open_orders(self, symbol).await
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        Binance::fetch_history_orders(self, symbol, since, limit).await
    }

    async fn fetch_trades_with_limit(
        &self,
        symbol: &str,
        _since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        Binance::fetch_market_trades(self, symbol, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_binance_trading_trait_object_safety() {
        let binance = Binance::new(ExchangeConfig::default()).unwrap();
        let trading: Box<dyn Trading> = Box::new(binance);

        assert_eq!(trading.id(), "binance");
        assert!(trading.capabilities().create_order());
    }
}

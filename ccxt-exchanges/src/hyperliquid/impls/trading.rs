use async_trait::async_trait;
use ccxt_core::{
    Result, Trade,
    traits::Trading,
    types::{Order, OrderRequest},
};

use crate::hyperliquid::HyperLiquid;

#[async_trait]
impl Trading for HyperLiquid {
    async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        HyperLiquid::create_order(self, request).await
    }

    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        HyperLiquid::cancel_order(self, id, symbol).await
    }

    async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        HyperLiquid::cancel_all_orders(self, Some(symbol)).await
    }

    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        HyperLiquid::fetch_order(self, id, symbol).await
    }

    async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        HyperLiquid::fetch_open_orders(self, symbol, None, None).await
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        HyperLiquid::fetch_history_orders(self, symbol, since, limit).await
    }

    async fn fetch_trades_with_limit(
        &self,
        symbol: &str,
        _since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        HyperLiquid::fetch_market_trades(self, symbol, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperliquid::HyperLiquidOptions;
    use ccxt_core::ExchangeConfig;

    #[test]
    fn test_hyperliquid_trading_trait_object_safety() {
        let options = HyperLiquidOptions::default();
        let hyperliquid =
            HyperLiquid::new_with_options(ExchangeConfig::default(), options, None).unwrap();
        let trading: Box<dyn Trading> = Box::new(hyperliquid);

        assert_eq!(trading.id(), "hyperliquid");
        assert!(trading.capabilities().create_order());
        assert!(trading.capabilities().cancel_order());
    }
}

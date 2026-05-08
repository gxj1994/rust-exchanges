//! Gate.io trading operations.
//!
//! This module contains trading methods including order creation, cancellation, and order management.

use crate::gate::parser;
use crate::gate::rest::builder::order_builder;
use ccxt_core::{
    Error, ParseError, Result,
    traits::MarketData,
    types::{Order, OrderRequest},
};

use super::Gate;
use super::HttpMethod;

impl Gate {
    /// Create a new spot order.
    pub async fn create_spot_order(&self, request: OrderRequest) -> Result<Order> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(&request.symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", request.symbol)))?;

        let body = order_builder::build_create_order_payload(&request, market)?;

        let response = self
            .signed_request("/api/v4/spot/orders")
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        parser::parse_order(&response, Some(market))
    }

    /// Cancel an existing spot order.
    pub async fn cancel_spot_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))?;

        let currency_pair = Gate::to_exchange_symbol(&market.id);

        let response = self
            .signed_request(&format!("/api/v4/spot/orders/{}", id))
            .method(HttpMethod::Delete)
            .param("currency_pair", currency_pair)
            .execute()
            .await?;

        parser::parse_order(&response, Some(market))
    }

    /// Fetch a specific spot order by ID.
    pub async fn fetch_spot_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))?;

        let currency_pair = Gate::to_exchange_symbol(&market.id);

        let response = self
            .signed_request(&format!("/api/v4/spot/orders/{}", id))
            .param("currency_pair", currency_pair)
            .execute()
            .await?;

        parser::parse_order(&response, Some(market))
    }

    /// Fetch all open spot orders.
    ///
    /// # API Endpoint
    ///
    /// Uses `GET /api/v4/spot/open_orders`:
    /// - With `currency_pair` parameter: returns orders for specific symbol
    /// - Without `currency_pair`: returns orders for all markets
    pub async fn fetch_spot_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        let mut builder = self.signed_request("/api/v4/spot/open_orders");

        if let Some(sym) = symbol {
            let markets = self.load_markets().await?;
            let market = markets
                .get(sym)
                .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", sym)))?;
            let currency_pair = Gate::to_exchange_symbol(&market.id);
            builder = builder.param("currency_pair", currency_pair);
        }

        let response = builder.execute().await?;

        let market_groups = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        let mut all_orders = Vec::new();
        let markets = self.load_markets().await?;

        for group in market_groups {
            // Get currency_pair from group to find market
            let cp = group.get("currency_pair").and_then(|v| v.as_str());
            let market = cp.and_then(|c| {
                markets
                    .iter()
                    .find(|(_, m)| m.id == c)
                    .map(|(_, m)| m.as_ref())
            });

            // Parse all orders in this group
            if let Some(orders) = group.get("orders").and_then(|v| v.as_array()) {
                for order_data in orders {
                    if let Ok(order) = parser::parse_order(order_data, market) {
                        all_orders.push(order);
                    }
                }
            }
        }

        Ok(all_orders)
    }

    /// Fetch closed and canceled spot orders.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/spot/orders?status=finished,closed,cancelled`
    pub async fn fetch_spot_history_orders(
        &self,
        symbol: Option<&str>,
        _since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let mut builder = self
            .signed_request("/api/v4/spot/orders")
            .param("status", "finished"); // Gate uses "finished" for closed orders

        if let Some(sym) = symbol {
            let markets = self.load_markets().await?;
            let market = markets
                .get(sym)
                .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", sym)))?;
            let currency_pair = Gate::to_exchange_symbol(&market.id);
            builder = builder.param("currency_pair", currency_pair);
        }

        if let Some(l) = limit {
            builder = builder.param("limit", l.to_string());
        }

        let response = builder.execute().await?;

        let orders_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        orders_array
            .iter()
            .map(|order_data| parser::parse_order(order_data, None))
            .collect()
    }

    /// Cancel all open spot orders for a symbol.
    ///
    /// # API Endpoint
    ///
    /// `DELETE /api/v4/spot/orders?currency_pair=BTC_USDT`
    pub async fn cancel_all_spot_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Market {} not found", symbol)))?;

        let currency_pair = Gate::to_exchange_symbol(&market.id);

        // Gate's cancel all orders returns array of canceled orders
        let response = self
            .signed_request("/api/v4/spot/orders")
            .method(HttpMethod::Delete)
            .param("currency_pair", currency_pair)
            .execute()
            .await?;

        // Response might be an array of canceled orders or a success message
        if let Some(orders_array) = response.as_array() {
            orders_array
                .iter()
                .map(|order_data| parser::parse_order(order_data, Some(market)))
                .collect()
        } else {
            // If it's not an array, return empty vec (Gate sometimes returns {})
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gate::GateBuilder;

    #[test]
    fn test_trading_trait_object_safety() {
        let gate = GateBuilder::default().build().unwrap();
        let _: &dyn ccxt_core::traits::Trading = &gate;
    }
}

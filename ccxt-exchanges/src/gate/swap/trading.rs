//! Gate.io contract (swap/futures) trading operations.
//!
//! This module contains contract order management methods.

use crate::gate::parser;
use crate::gate::rest::HttpMethod;
use crate::gate::rest::builder::order_builder;
use ccxt_core::{
    Error, ParseError, Result,
    traits::MarketData,
    types::{Order, OrderRequest},
};

use super::Gate;
impl Gate {
    /// Create a contract order.
    ///
    /// # API Endpoint
    ///
    /// `POST /api/v4/futures/{settle}/orders`
    pub async fn create_contract_order(&self, request: OrderRequest) -> Result<Order> {
        let markets = self.load_markets().await?;
        let market = markets.get(&request.symbol).ok_or_else(|| {
            Error::bad_symbol(format!("Contract market {} not found", request.symbol))
        })?;

        let settle = market.settle.as_deref().unwrap_or("usdt").to_lowercase();

        // Build contract-specific order payload
        let body = order_builder::build_create_contract_order_payload(&request, market)?;

        println!(
            "  Payload: {}",
            serde_json::to_string_pretty(&body).unwrap()
        );

        let url = format!("/api/v4/futures/{}/orders", settle);
        let response = self
            .signed_request(&url)
            .method(HttpMethod::Post)
            .param("contract", &market.id)
            .body(body)
            .execute()
            .await?;

        parser::parse_contract_order(&response, Some(&market))
    }

    /// Cancel a contract order.
    ///
    /// # API Endpoint
    ///
    /// `DELETE /api/v4/futures/{settle}/orders/{order_id}?contract={contract}`
    pub async fn cancel_contract_order(&self, order_id: &str, symbol: &str) -> Result<Order> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", symbol)))?;

        let settle = market.settle.as_deref().unwrap_or("usdt").to_lowercase();
        let contract = &market.id;

        let url = format!("/api/v4/futures/{}/orders/{}", settle, order_id);
        let response = self
            .signed_request(&url)
            .method(HttpMethod::Delete)
            .param("contract", contract)
            .execute()
            .await?;

        parser::parse_contract_order(&response, Some(&market))
    }

    /// Cancel all open contract orders for a symbol.
    ///
    /// # API Endpoint
    ///
    /// `DELETE /api/v4/futures/{settle}/orders?contract={contract}`
    pub async fn cancel_all_contract_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", symbol)))?;

        let settle = market.settle.as_deref().unwrap_or("usdt").to_lowercase();
        let contract = &market.id;

        let url = format!("/api/v4/futures/{}/orders", settle);
        let response = self
            .signed_request(&url)
            .method(HttpMethod::Delete)
            .param("contract", contract)
            .execute()
            .await?;

        if let Some(orders_array) = response.as_array() {
            orders_array
                .iter()
                .map(|order_data| parser::parse_contract_order(order_data, Some(&market)))
                .collect()
        } else {
            Ok(vec![])
        }
    }

    /// Fetch a contract order by ID.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/orders/{order_id}`
    pub async fn fetch_contract_order(&self, order_id: &str, symbol: &str) -> Result<Order> {
        let markets = self.load_markets().await?;
        let market = markets
            .get(symbol)
            .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", symbol)))?;

        let settle = market.settle.as_deref().unwrap_or("usdt").to_lowercase();

        let url = format!("/api/v4/futures/{}/orders/{}", settle, order_id);
        let response = self.signed_request(&url).execute().await?;

        parser::parse_contract_order(&response, Some(&market))
    }

    /// Fetch open contract orders.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/orders?status=open`
    pub async fn fetch_contract_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        let settle = self.get_contract_settle();

        let mut builder = self
            .signed_request(&format!("/api/v4/futures/{}/orders", settle))
            .param("status", "open");

        if let Some(sym) = symbol {
            let markets = self.load_markets().await?;
            let market = markets
                .get(sym)
                .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", sym)))?;
            builder = builder.param("contract", &market.id);
        }

        let response = builder.execute().await?;

        let orders_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("response", "Expected array")))?;

        orders_array
            .iter()
            .map(|order_data| parser::parse_contract_order(order_data, None))
            .collect()
    }

    /// Fetch closed contract orders.
    ///
    /// # API Endpoint
    ///
    /// `GET /api/v4/futures/{settle}/orders?status=finished`
    pub async fn fetch_contract_history_orders(
        &self,
        symbol: Option<&str>,
        _since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let settle = self.get_contract_settle();

        let mut builder = self
            .signed_request(&format!("/api/v4/futures/{}/orders", settle))
            .param("status", "finished");

        if let Some(sym) = symbol {
            let markets = self.load_markets().await?;
            let market = markets
                .get(sym)
                .ok_or_else(|| Error::bad_symbol(format!("Contract market {} not found", sym)))?;
            builder = builder.param("contract", &market.id);
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
            .map(|order_data| parser::parse_contract_order(order_data, None))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::gate::GateBuilder;

    #[test]
    fn test_contract_trading_methods_exist() {
        let gate = GateBuilder::default().build().unwrap();

        // Verify the struct can be created (compile-time check)
        assert!(gate.get_contract_rest_url().contains("fx-api"));
    }
}

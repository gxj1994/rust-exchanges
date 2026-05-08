//! Binance spot trading operations.
//!
//! This module contains all spot trading methods including order creation,
//! cancellation, and order management.

use super::super::{Binance, core::constants::endpoints, parser, signed_request::HttpMethod};
use super::builder::order_builder;
use ccxt_core::network::endpoint_manager::ExchangeEndpointManager;
use ccxt_core::{
    Error, ParseError, Result,
    types::{MarketType, OcoOrder, Order, OrderRequest, OrderSide, OrderStatus},
};
use std::collections::{BTreeMap, HashMap};
use tracing::warn;

impl Binance {
    /// Create a new order using the builder pattern.
    ///
    /// This is the preferred method for creating orders. It accepts an [`OrderRequest`]
    /// built using the builder pattern, which provides compile-time validation of
    /// required fields and a more ergonomic API.
    ///
    /// # Arguments
    ///
    /// * `request` - Order request built via [`OrderRequest::builder()`]
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ccxt_exchanges::binance::Binance;
    /// use ccxt_core::{ExchangeConfig, types::{OrderRequest, OrderSide, OrderType, Amount, Price}};
    /// use rust_decimal_macros::dec;
    ///
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Create a market order using the builder
    /// let request = OrderRequest::builder()
    ///     .symbol("BTC/USDT")
    ///     .side(OrderSide::Buy)
    ///     .order_type(OrderType::Market)
    ///     .amount(Amount::new(dec!(0.001)))
    ///     .build()?;
    ///
    /// let order = binance.create_order(request).await?;
    /// println!("Order created: {:?}", order);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// _Requirements: 2.2, 2.6_
    pub async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        let market = self.base().market(&request.symbol).await?;

        // 根据 market_type 路由到不同的构建和解析逻辑
        match market.market_type {
            MarketType::Spot => {
                // 现货逻辑
                let payload = order_builder::build_create_order_payload(&request, &market)?;

                let base_url = self.rest_endpoint_for_market(&market);
                let url = format!("{}{}", base_url, endpoints::ORDER);

                let data = self
                    .signed_request(url)
                    .method(HttpMethod::Post)
                    .params(
                        payload
                            .as_object()
                            .unwrap()
                            .iter()
                            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                            .collect::<BTreeMap<String, String>>(),
                    )
                    .execute()
                    .await?;

                // 现货响应解析 (ACK/RESULT 智能路由)
                if data.get("status").is_none() {
                    parser::parse_ack_order(&data, &request, &market)
                } else {
                    parser::parse_order(&data, Some(&market))
                }
            }
            MarketType::Swap | MarketType::Futures => {
                // 普通订单使用 /fapi/v1/order
                let payload = order_builder::build_futures_create_order_payload(&request, &market)?;

                let base_url = self.rest_endpoint_for_market(&market);
                let url = format!("{}{}", base_url, endpoints::ORDER);

                let data = self
                    .signed_request(url)
                    .method(HttpMethod::Post)
                    .params(
                        payload
                            .as_object()
                            .unwrap()
                            .iter()
                            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                            .collect::<BTreeMap<String, String>>(),
                    )
                    .execute()
                    .await?;

                // 合约响应解析 (ACK/RESULT 智能路由)
                if data.get("status").is_none() {
                    parser::parse_futures_ack_order(&data, &request, &market)
                } else {
                    parser::parse_futures_order(&data, Some(&market))
                }
            }
            _ => Err(Error::invalid_request(format!(
                "Unsupported market type for create_order: {:?}",
                market.market_type
            ))),
        }
    }

    /// Cancel an order.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the cancelled [`Order`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;
        // Use market-type-aware endpoint routing
        let base_url = self.rest_endpoint_for_market(&market);
        let url = format!("{}{}", base_url, endpoints::ORDER);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Delete)
            .param("symbol", &market.id)
            .param("orderId", id)
            .execute()
            .await?;

        parser::parse_order(&data, Some(&market))
    }

    /// Fetch order details.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the [`Order`] information.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;
        // Use market-type-aware endpoint routing
        let base_url = self.rest_endpoint_for_market(&market);
        let url = format!("{}{}", base_url, endpoints::ORDER);

        let data = self
            .signed_request(url)
            .param("symbol", &market.id)
            .param("orderId", id)
            .execute()
            .await?;

        parser::parse_order(&data, Some(&market))
    }

    /// Fetch open (unfilled) orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If `None`, fetches all open orders.
    ///
    /// # Returns
    ///
    /// Returns a vector of open [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        // Use market-type-aware endpoint routing if market is available,
        // otherwise fall back to spot endpoint
        let base_url = match &market {
            Some(m) => self.rest_endpoint_for_market(m),
            None => self.endpoints().rest.spot.to_string(),
        };
        let url = format!("{}{}", base_url, endpoints::OPEN_ORDERS);

        let data = self
            .signed_request(url)
            .optional_param("symbol", market.as_ref().map(|m| &m.id))
            .execute()
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_deref()) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch closed (completed) orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `since` - Optional start timestamp (milliseconds).
    /// * `limit` - Optional limit on number of orders (default 500, max 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of closed [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let all_orders = self.fetch_orders(symbol, since, None).await?;

        let mut closed_orders: Vec<Order> = all_orders
            .into_iter()
            .filter(|order| order.status == ccxt_core::types::OrderStatus::Closed)
            .collect();

        if let Some(l) = limit {
            closed_orders.truncate(l as usize);
        }

        Ok(closed_orders)
    }

    /// Cancel all open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns a vector of cancelled [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        let market = self.base().market(symbol).await?;
        // Use market-type-aware endpoint routing
        let base_url = self.rest_endpoint_for_market(&market);
        let url = format!("{}{}", base_url, endpoints::OPEN_ORDERS);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Delete)
            .param("symbol", &market.id)
            .execute()
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, Some(&market)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Cancel multiple orders.
    ///
    /// # Arguments
    ///
    /// * `ids` - Vector of order IDs to cancel.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns a vector of cancelled [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    pub async fn cancel_orders(&self, ids: Vec<String>, symbol: &str) -> Result<Vec<Order>> {
        let market = self.base().market(symbol).await?;

        let order_ids_json = serde_json::to_string(&ids).map_err(|e| {
            Error::from(ParseError::invalid_format(
                "data",
                format!("Failed to serialize order IDs: {}", e),
            ))
        })?;

        // Use market-type-aware endpoint routing
        let base_url = self.rest_endpoint_for_market(&market);
        let url = format!("{}{}", base_url, endpoints::OPEN_ORDERS);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Delete)
            .param("symbol", &market.id)
            .param("orderIdList", order_ids_json)
            .execute()
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, Some(&market)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch all orders (historical and current).
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol.
    /// * `since` - Optional start timestamp (milliseconds).
    /// * `limit` - Optional limit on number of orders (default 500, max 1000).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Order`] structures.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails or the API request fails.
    pub async fn fetch_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        // Use market-type-aware endpoint routing if market is available,
        // otherwise fall back to spot endpoint
        let base_url = match &market {
            Some(m) => self.rest_endpoint_for_market(m),
            None => self.endpoints().rest.spot.to_string(),
        };
        let url = format!("{}{}", base_url, endpoints::ALL_ORDERS);

        let data = self
            .signed_request(url)
            .optional_param("symbol", market.as_ref().map(|m| &m.id))
            .optional_param("startTime", since)
            .optional_param("limit", limit)
            .execute()
            .await?;

        let orders_array = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_deref()) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    // ==================== OCO Order Methods ====================

    /// Create an OCO (One-Cancels-the-Other) order.
    ///
    /// An OCO order combines a limit order (take profit) and a stop-limit order (stop loss).
    /// When one order is filled, the other is automatically cancelled.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `side` - Order side (Buy/Sell).
    /// * `amount` - Order quantity.
    /// * `price` - Limit order price (take profit).
    /// * `stop_price` - Stop trigger price (stop loss trigger).
    /// * `stop_limit_price` - Stop limit price (stop loss execution price).
    /// * `params` - Optional additional parameters:
    ///   - `listClientOrderId`: Client ID for the OCO order list.
    ///   - `limitClientOrderId`: Client ID for the limit order.
    ///   - `stopClientOrderId`: Client ID for the stop order.
    ///   - `timeInForce`: Time in force for the limit order (default: GTC).
    ///   - `stopIcebergQty`: Iceberg quantity for stop order.
    ///   - `limitIcebergQty`: Iceberg quantity for limit order.
    ///
    /// # Returns
    ///
    /// Returns an [`OcoOrder`] with order details and reports.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::{ExchangeConfig, types::OrderSide};
    /// # use rust_decimal_macros::dec;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Create OCO sell order: take profit at 70000, stop loss at 55000
    /// let oco = binance.create_oco_order(
    ///     "BTC/USDT",
    ///     OrderSide::Sell,
    ///     "0.001",
    ///     "70000",
    ///     "55000",
    ///     "54000",
    ///     None
    /// ).await?;
    /// println!("OCO Order created: list_id={}", oco.order_list_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_oco_order(
        &self,
        symbol: &str,
        side: OrderSide,
        amount: &str,
        price: &str,
        stop_price: &str,
        stop_limit_price: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<OcoOrder> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let url = format!("{}/order/oco", self.endpoints().rest.spot);

        let side_str = match side {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        };

        let mut request = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .param("symbol", market.id.clone())
            .param("side", side_str)
            .param("quantity", amount)
            .param("price", price)
            .param("stopPrice", stop_price)
            .param("stopLimitPrice", stop_limit_price)
            .param("stopLimitTimeInForce", "GTC");

        // Add optional parameters
        if let Some(p) = params {
            for (key, value) in p {
                request = request.param(&key, value);
            }
        }

        let data = request.execute().await?;

        parser::parse_oco_order(&data)
    }

    /// Cancel an OCO order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `order_list_id` - OCO order list ID (optional if client_order_id is provided).
    /// * `client_order_id` - Client order list ID (optional if order_list_id is provided).
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns the cancelled [`OcoOrder`].
    ///
    /// # Errors
    ///
    /// Returns an error if neither order_list_id nor client_order_id is provided,
    /// or if the order cannot be found.
    pub async fn cancel_oco_order(
        &self,
        symbol: &str,
        order_list_id: Option<i64>,
        client_order_id: Option<&str>,
        params: Option<HashMap<String, String>>,
    ) -> Result<OcoOrder> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let url = format!("{}/orderList", self.endpoints().rest.spot);

        let mut request = self
            .signed_request(url)
            .method(HttpMethod::Delete)
            .param("symbol", market.id.clone());

        // Must provide either order_list_id or client_order_id
        if let Some(id) = order_list_id {
            request = request.param("orderListId", id.to_string());
        } else if let Some(client_id) = client_order_id {
            request = request.param("listClientOrderId", client_id.to_string());
        } else {
            return Err(Error::invalid_request(
                "Either order_list_id or client_order_id must be provided".to_string(),
            ));
        }

        // Add optional parameters
        if let Some(p) = params {
            for (key, value) in p {
                request = request.param(&key, value);
            }
        }

        let data = request.execute().await?;

        parser::parse_oco_order(&data)
    }

    /// Fetch OCO order details.
    ///
    /// # Arguments
    ///
    /// * `order_list_id` - OCO order list ID (optional if client_order_id is provided).
    /// * `client_order_id` - Client order list ID (optional if order_list_id is provided).
    ///
    /// # Returns
    ///
    /// Returns the [`OcoOrder`] details.
    ///
    /// # Errors
    ///
    /// Returns an error if neither order_list_id nor client_order_id is provided,
    /// or if the order cannot be found.
    pub async fn fetch_oco_order(
        &self,
        order_list_id: Option<i64>,
        client_order_id: Option<&str>,
    ) -> Result<OcoOrder> {
        let url = format!("{}/orderList", self.endpoints().rest.spot);

        let mut request = self.signed_request(url);

        // Must provide either order_list_id or client_order_id
        if let Some(id) = order_list_id {
            request = request.param("orderListId", id.to_string());
        } else if let Some(client_id) = client_order_id {
            request = request.param("origClientOrderId", client_id.to_string());
        } else {
            return Err(Error::invalid_request(
                "Either order_list_id or client_order_id must be provided".to_string(),
            ));
        }

        let data = request.execute().await?;

        parser::parse_oco_order(&data)
    }

    /// Fetch all OCO orders.
    ///
    /// # Arguments
    ///
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional quantity limit (default: 500, max: 1000).
    /// * `params` - Optional additional parameters:
    ///   - `fromId`: Query from this order list ID.
    ///   - `startTime`: Start timestamp in milliseconds.
    ///   - `endTime`: End timestamp in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`OcoOrder`] records.
    pub async fn fetch_oco_orders(
        &self,
        since: Option<i64>,
        limit: Option<i64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<OcoOrder>> {
        let url = format!("{}/allOrderList", self.endpoints().rest.spot);

        let mut request = self.signed_request(url);

        if let Some(s) = since {
            request = request.param("startTime", s.to_string());
        }

        // Default limit is 500, max is 1000
        let actual_limit = limit.map_or(500, |l| l.min(1000));
        request = request.param("limit", actual_limit.to_string());

        // Add optional parameters
        if let Some(p) = params {
            for (key, value) in p {
                request = request.param(&key, value);
            }
        }

        let data = request.execute().await?;

        parser::parse_oco_orders(&data)
    }

    /// Fetch open OCO orders.
    ///
    /// # Returns
    ///
    /// Returns a vector of open [`OcoOrder`] records.
    pub async fn fetch_open_oco_orders(&self) -> Result<Vec<OcoOrder>> {
        let url = format!("{}/openOrderList", self.endpoints().rest.spot);

        let data = self.signed_request(url).execute().await?;

        parser::parse_oco_orders(&data)
    }

    /// Fetch trades for a specific order.
    ///
    /// Retrieves the trade history (fills) for a specific order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `order_id` - Order ID to fetch trades for.
    /// * `params` - Optional additional parameters.
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] records for the order.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch trades for a specific order
    /// let trades = binance.fetch_order_trades("BTC/USDT", "12345678", None).await?;
    /// for trade in trades {
    ///     println!("Trade: {} @ {}", trade.amount, trade.price);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_order_trades(
        &self,
        symbol: &str,
        order_id: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<ccxt_core::types::Trade>> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let url = format!("{}/myTrades", self.endpoints().rest.spot);

        let mut request = self
            .signed_request(url)
            .param("symbol", market.id.clone())
            .param("orderId", order_id.to_string());

        // Add optional parameters
        if let Some(p) = params {
            for (key, value) in p {
                request = request.param(&key, value);
            }
        }

        let data = request.execute().await?;

        parser::parse_trades(&data, Some(&market))
    }

    /// Fetch canceled orders.
    ///
    /// Retrieves all canceled orders for a symbol within a time range.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT").
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional quantity limit (default: 500, max: 1000).
    /// * `params` - Optional additional parameters:
    ///   - `startTime`: Start timestamp in milliseconds.
    ///   - `endTime`: End timestamp in milliseconds.
    ///
    /// # Returns
    ///
    /// Returns a vector of canceled [`Order`] records.
    ///
    /// # Notes
    ///
    /// - Maximum query range: 24 hours for all orders, 90 days for individual orders.
    /// - If `orderId` is provided in params, fetches orders from that order ID onwards.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ccxt_exchanges::binance::Binance;
    /// # use ccxt_core::ExchangeConfig;
    /// # async fn example() -> ccxt_core::Result<()> {
    /// let binance = Binance::new(ExchangeConfig::default())?;
    ///
    /// // Fetch canceled orders for BTC/USDT
    /// let orders = binance.fetch_canceled_orders("BTC/USDT", None, Some(100), None).await?;
    /// println!("Canceled orders: {}", orders.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_canceled_orders(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<i64>,
        params: Option<HashMap<String, String>>,
    ) -> Result<Vec<Order>> {
        self.load_markets(false).await?;
        let market = self.base().market(symbol).await?;

        let url = format!("{}/allOrders", self.endpoints().rest.spot);

        let mut request = self.signed_request(url).param("symbol", market.id.clone());

        // Add optional parameters
        if let Some(s) = since {
            request = request.param("startTime", s.to_string());
        }

        // Default limit is 500, max is 1000
        let actual_limit = limit.map_or(500, |l| l.min(1000));
        request = request.param("limit", actual_limit.to_string());

        // Add additional params
        if let Some(p) = params {
            for (key, value) in p {
                request = request.param(&key, value);
            }
        }

        let data = request.execute().await?;

        // Parse all orders and filter for canceled ones
        let all_orders = parser::parse_orders(&data, Some(&market))?;

        // Filter for canceled orders
        let canceled_orders: Vec<Order> = all_orders
            .into_iter()
            .filter(|order| {
                matches!(
                    order.status,
                    OrderStatus::Cancelled | OrderStatus::Expired | OrderStatus::Rejected
                )
            })
            .collect();

        Ok(canceled_orders)
    }

    // ==================== Futures Algo Order Methods ====================

    /// Create a futures algo order (conditional order).
    ///
    /// This method handles conditional orders (STOP, TAKE_PROFIT, STOP_MARKET, TAKE_PROFIT_MARKET)
    /// which must use the Algo Order API (/fapi/v1/algoOrder) instead of the regular order API.
    ///
    /// # Arguments
    ///
    /// * `request` - Order request built via [`OrderRequest::builder()`]
    /// * `market` - Market metadata
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails, market is not found, or the API request fails.
    ///
    /// # API Reference
    ///
    /// POST /fapi/v1/algoOrder
    /// https://developers.binance.com/docs/derivatives/usds-margined-futures/trade/rest-api/New-Algo-Order
    pub async fn create_futures_algo_order(&self, request: OrderRequest) -> Result<Order> {
        let market = self.base().market(&request.symbol).await?;
        let payload = order_builder::build_futures_algo_order_payload(&request, &market)?;

        let base_url = self.rest_endpoint_for_market(&market);
        let url = format!("{}{}", base_url, endpoints::ALGO_ORDER);

        let data = self
            .signed_request(url)
            .method(HttpMethod::Post)
            .params(
                payload
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                    .collect::<BTreeMap<String, String>>(),
            )
            .execute()
            .await?;

        // Parse algo order response
        parser::parse_futures_algo_order(&data, &request, &market)
    }
}

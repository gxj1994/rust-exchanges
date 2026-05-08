//! Bybit trading operations.
//!
//! This module contains trading methods including order creation,
//! cancellation, and order management.

use crate::bybit::{Bybit, parser, rest::builder::order_builder, signed_request::HttpMethod};
use ccxt_core::{
    Error, ParseError, Result, Trade,
    types::{Order, OrderRequest},
};
use tracing::warn;

impl Bybit {
    /// Create a new order.
    ///
    /// # Arguments
    ///
    /// * `request` - Order request built via [`OrderRequest::builder()`]
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    ///
    /// _Requirements: 2.2, 2.6_
    pub async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        let market = self.base().market(&request.symbol).await?;

        let path = Self::build_api_path("/order/create");

        // 构建额外参数
        let extra_params = self
            .build_order_extra_params(&request.symbol, market.market_type)
            .await?;

        let body = order_builder::build_create_order_payload(&request, &market, &extra_params)?;

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        // 检查 Bybit API 错误响应
        if crate::bybit::core::error::is_error_response(&response) {
            return Err(crate::bybit::core::error::parse_error(&response));
        }

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        // Bybit create-order 只返回 orderId 和 orderLinkId
        // 需要调用查询接口获取完整订单信息
        let order_id = result
            .get("orderId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?;

        // 调用 fetch_order 获取完整订单详情
        self.fetch_order(order_id, &request.symbol).await
    }

    /// Cancel an existing order.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to cancel.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the canceled [`Order`] structure.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/order/cancel");

        let category = self.category_from_symbol(symbol).await?;
        let body = order_builder::build_cancel_order_payload(id, &market, &category, None);

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        // 检查 Bybit API 错误响应
        if crate::bybit::core::error::is_error_response(&response) {
            return Err(crate::bybit::core::error::parse_error(&response));
        }

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        // Bybit cancel-order 只返回 orderId 和 orderLinkId
        // 需要调用查询接口获取完整订单信息（包括取消后的状态）
        let order_id = result
            .get("orderId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::from(ParseError::missing_field("orderId")))?;

        // 调用 fetch_order 获取完整订单详情（状态应为 Cancelled）
        self.fetch_order(order_id, symbol).await
    }

    /// Fetch a single order by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to fetch.
    /// * `symbol` - Trading pair symbol.
    ///
    /// # Returns
    ///
    /// Returns the [`Order`] structure with current status.
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/order/realtime");

        let response = self
            .signed_request(&path)
            .param("category", self.get_category())
            .param("symbol", &market.id)
            .param("orderId", id)
            .execute()
            .await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let orders = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("110008", "Order not found"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Fetch open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all open orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 50).
    ///
    /// # Returns
    ///
    /// Returns a vector of open [`Order`] structures.
    pub async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = Self::build_api_path("/order/realtime");

        // Bybit maximum limit is 50
        let actual_limit = limit.map_or(50, |l| l.min(50));

        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let mut builder = self
            .signed_request(&path)
            .param("category", self.get_category())
            .param("limit", actual_limit)
            .optional_param("startTime", since);

        if let Some(ref m) = market {
            builder = builder.param("symbol", &m.id);
        }

        let response = builder.execute().await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let orders_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_deref()) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse open order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch closed orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all closed orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 50).
    ///
    /// # Returns
    ///
    /// Returns a vector of closed [`Order`] structures.
    pub async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let path = Self::build_api_path("/order/history");

        // Bybit maximum limit is 50
        let actual_limit = limit.map_or(50, |l| l.min(50));

        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let mut builder = self
            .signed_request(&path)
            .param("category", self.get_category())
            .param("limit", actual_limit)
            .optional_param("startTime", since);

        if let Some(ref m) = market {
            builder = builder.param("symbol", &m.id);
        }

        let response = builder.execute().await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let orders_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of orders",
            ))
        })?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, market.as_deref()) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse closed order");
                }
            }
        }

        Ok(orders)
    }

    /// Cancel all open orders for a trading pair.
    ///
    /// Bybit API: POST /v5/order/cancel-all
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (CCXT standard format, e.g., "BTC/USDT", "BTC/USDT:USDT").
    ///
    /// # Returns
    ///
    /// Returns a list of cancelled [`Order`] structures.
    pub async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/order/cancel-all");

        let mut body_map = serde_json::Map::new();
        let category = self.category_from_symbol(symbol).await?;
        body_map.insert(
            "category".to_string(),
            serde_json::Value::String(category.to_string()),
        );
        body_map.insert(
            "symbol".to_string(),
            serde_json::Value::String(market.id.clone()),
        );

        let body = serde_json::Value::Object(body_map);

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        // 检查 Bybit API 错误响应
        if crate::bybit::core::error::is_error_response(&response) {
            return Err(crate::bybit::core::error::parse_error(&response));
        }

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        // Check success field: "1" = success, "0" = failure
        let success = result
            .get("success")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        if success != "1" {
            // Even if retCode=0, success="0" means cancel-all operation failed
            return Err(Error::invalid_request(format!(
                "Bybit cancel-all orders operation failed: success={}",
                success
            )));
        }

        // Bybit returns list of order IDs that were cancelled
        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let order_ids = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of order IDs",
            ))
        })?;

        // Parse cancelled orders (we need to fetch full order details)
        let mut orders = Vec::new();

        for order_item in order_ids {
            // Each item is an object: {"orderId": "...", "orderLinkId": "..."}
            if let Some(order_id) = order_item.get("orderId").and_then(|v| v.as_str()) {
                // Fetch full order details
                match self.fetch_order(order_id, symbol).await {
                    Ok(order) => orders.push(order),
                    Err(e) => {
                        warn!(order_id = %order_id, error = %e, "Failed to fetch cancelled order details");
                    }
                }
            }
        }

        Ok(orders)
    }

    /// Fetch user's trade history.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of trades (maximum: 100).
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Trade`] structures representing user's trade history.
    pub async fn fetch_account_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;

        let path = Self::build_api_path("/execution/list");

        // Bybit maximum limit is 100
        let actual_limit = limit.map_or(50, |l| l.min(100));

        let response = self
            .signed_request(&path)
            .param("category", self.get_category())
            .param("symbol", &market.id)
            .param("limit", actual_limit)
            .optional_param("startTime", since)
            .execute()
            .await?;

        let result = response
            .get("result")
            .ok_or_else(|| Error::from(ParseError::missing_field("result")))?;

        let list = result
            .get("list")
            .ok_or_else(|| Error::from(ParseError::missing_field("list")))?;

        let trades_array = list.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of trades",
            ))
        })?;

        let mut trades = Vec::new();
        for trade_data in trades_array {
            match parser::parse_trade(trade_data, Some(&market)) {
                Ok(trade) => trades.push(trade),
                Err(e) => {
                    warn!(error = %e, "Failed to parse my trade");
                }
            }
        }

        Ok(trades)
    }

    /// 构建订单的额外参数（category 和 hedged_mode）
    ///
    /// 对于合约订单，查询持仓模式以正确设置 positionIdx：
    /// - 单向模式 (hedged_mode=false): positionIdx=0
    /// - 双向模式 (hedged_mode=true): positionIdx=1 (Long) 或 2 (Short)
    async fn build_order_extra_params(
        &self,
        symbol: &str,
        market_type: ccxt_core::types::market::MarketType,
    ) -> Result<std::collections::HashMap<String, String>> {
        use std::collections::HashMap;
        let mut extra_params = HashMap::new();

        // 添加 category
        let category = self.category_from_symbol(symbol).await?;
        extra_params.insert("category".to_string(), category.clone());

        // 对于合约订单，查询持仓模式以正确设置 positionIdx
        if matches!(
            market_type,
            ccxt_core::types::market::MarketType::Swap
                | ccxt_core::types::market::MarketType::Futures
        ) {
            match self.fetch_positions(Some(symbol.to_string()), None).await {
                Ok(positions) => {
                    if let Some(pos) = positions.first() {
                        let hedged = pos.hedged.or(pos.dual_side_position).unwrap_or(false);
                        extra_params.insert("hedged_mode".to_string(), hedged.to_string());
                        tracing::debug!(
                            symbol = %symbol,
                            hedged = hedged,
                            "Detected position mode from existing position"
                        );
                    } else {
                        extra_params.insert("hedged_mode".to_string(), "false".to_string());
                        tracing::debug!(
                            symbol = %symbol,
                            "No position found, defaulting to one-way mode"
                        );
                    }
                }
                Err(e) => {
                    extra_params.insert("hedged_mode".to_string(), "false".to_string());
                    tracing::warn!(
                        error = %e,
                        symbol = %symbol,
                        "Failed to fetch positions, defaulting to one-way mode"
                    );
                }
            }
        }

        Ok(extra_params)
    }
}

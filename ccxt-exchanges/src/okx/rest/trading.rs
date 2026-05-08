//! Trading operations for OKX REST API.

use super::super::{Okx, core::error as okx_error, parser};
use super::builder::order_builder;
use crate::okx::signed_request::HttpMethod;
use ccxt_core::{
    Error, ParseError, Result,
    types::{Order, OrderRequest, OrderStatus},
};
use std::collections::HashMap;
use tracing::warn;

impl Okx {
    /// Create a new order.
    pub async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        let market = self.base().market(&request.symbol).await?;

        // 构建 extra_params
        let extra_params = self.build_order_extra_params(&market).await?;

        // 使用 order_builder 构建请求体
        let body = order_builder::build_create_order_payload(&request, &market, &extra_params)?;

        let path = Self::build_api_path("/trade/order");
        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        // 检查顶层错误码 (使用 okx_error 模块)
        if okx_error::is_error_response(&response) {
            return Err(okx_error::parse_error(&response));
        }

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("-1", "No order data returned"));
        }

        let order_data = &orders[0];

        // 检查单个订单的成功码 (sCode)
        // OKX 下单API每个订单项都有独立的 sCode/sMsg
        let s_code = order_data
            .get("sCode")
            .and_then(|v| v.as_str())
            .unwrap_or("-1");

        if s_code != "0" {
            // 构造错误响应格式,复用 okx_error::parse_error
            let error_response = serde_json::json!({
                "code": s_code,
                "msg": order_data.get("sMsg").and_then(|v| v.as_str()).unwrap_or("Unknown order error")
            });
            return Err(okx_error::parse_error(&error_response));
        }

        let order = parser::parse_order(order_data, Some(&market))?;

        // OKX 下单API返回极简响应(只有 ordId),需要自动 fetch_order 补全
        if order.status == OrderStatus::Pending {
            let symbol = request.symbol.clone();
            return self.fetch_order(&order.id, &symbol).await;
        }

        Ok(order)
    }

    /// 构建订单额外参数 (td_mode, pos_side_mode 等)
    async fn build_order_extra_params(
        &self,
        _market: &ccxt_core::types::market::Market,
    ) -> Result<HashMap<String, String>> {
        let mut params = HashMap::new();

        // 获取 tdMode (必填)
        // OKX 规则:
        // - 现货: cash
        // - 杠杆: cross (全仓) / isolated (逐仓)
        // - 合约: cross (全仓) / isolated (逐仓)
        params.insert("td_mode".to_string(), self.options().account_mode.clone());

        // 注意: OKX 的持仓模式 (posSide) 由用户通过 request.position_side 直接指定
        // 不需要像 Bybit 那样查询 hedged_mode

        Ok(params)
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

        let path = Self::build_api_path("/trade/cancel-order");

        let mut map = serde_json::Map::new();
        map.insert(
            "instId".to_string(),
            serde_json::Value::String(market.id.clone()),
        );
        map.insert(
            "ordId".to_string(),
            serde_json::Value::String(id.to_string()),
        );
        let body = serde_json::Value::Object(map);

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        // 检查顶层错误码 (使用 okx_error 模块)
        if okx_error::is_error_response(&response) {
            return Err(okx_error::parse_error(&response));
        }

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("-1", "No order data returned"));
        }

        // OKX cancel-order 只返回取消确认信息,不包含完整订单状态
        // 需要调用 fetch_order 获取完整订单详情(状态应为 Cancelled)
        self.fetch_order(id, symbol).await
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

        let path = Self::build_api_path("/trade/order");

        let response = self
            .signed_request(&path)
            .param("instId", &market.id)
            .param("ordId", id)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of orders",
            ))
        })?;

        if orders.is_empty() {
            return Err(Error::exchange("51400", "Order not found"));
        }

        parser::parse_order(&orders[0], Some(&market))
    }

    /// Fetch open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If None, fetches all open orders.
    /// * `since` - Optional start timestamp in milliseconds.
    /// * `limit` - Optional limit on number of orders (maximum: 100).
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
        let path = Self::build_api_path("/trade/orders-pending");

        let actual_limit = limit.map_or(100, |l| l.min(100));

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            Some(m)
        } else {
            None
        };

        let mut builder = self
            .signed_request(&path)
            .param("instType", self.get_inst_type())
            .param("limit", actual_limit);

        if let Some(ref m) = market {
            builder = builder.param("instId", &m.id);
        }

        if let Some(start_time) = since {
            builder = builder.param("begin", start_time);
        }

        let response = builder.execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

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
    /// * `limit` - Optional limit on number of orders (maximum: 100).
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
        let path = Self::build_api_path("/trade/orders-history");

        let actual_limit = limit.map_or(100, |l| l.min(100));

        let market = if let Some(sym) = symbol {
            let m = self.base().market(sym).await?;
            Some(m)
        } else {
            None
        };

        let mut builder = self
            .signed_request(&path)
            .param("instType", self.get_inst_type())
            .param("limit", actual_limit);

        if let Some(ref m) = market {
            builder = builder.param("instId", &m.id);
        }

        if let Some(start_time) = since {
            builder = builder.param("begin", start_time);
        }

        let response = builder.execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

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
                    warn!(error = %e, "Failed to parse closed order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch all orders (both open and closed).

    /// Cancel all open orders for a trading pair.
    ///
    /// OKX API: POST /api/v5/trade/cancel-batch-orders
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

        // First, fetch all open orders for this symbol
        let open_orders = self.fetch_open_orders(Some(symbol), None, None).await?;

        if open_orders.is_empty() {
            return Ok(Vec::new());
        }

        // Build batch cancel request
        let path = Self::build_api_path("/trade/cancel-batch-orders");

        // Build order data array for batch cancellation
        let order_data: Vec<serde_json::Value> = open_orders
            .iter()
            .map(|order| {
                let mut map = serde_json::Map::new();
                map.insert(
                    "instId".to_string(),
                    serde_json::Value::String(market.id.clone()),
                );
                map.insert(
                    "ordId".to_string(),
                    serde_json::Value::String(order.id.clone()),
                );
                serde_json::Value::Object(map)
            })
            .collect();

        let body = serde_json::Value::Array(order_data);

        let response = self
            .signed_request(&path)
            .method(HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        // 检查顶层错误码 (使用 okx_error 模块)
        if okx_error::is_error_response(&response) {
            return Err(okx_error::parse_error(&response));
        }

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let cancelled_orders = data.as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data",
                "Expected array of cancelled orders",
            ))
        })?;

        // Parse cancelled orders
        let mut orders = Vec::new();
        for order_data in cancelled_orders {
            match parser::parse_order(order_data, Some(&market)) {
                Ok(order) => orders.push(order),
                Err(e) => {
                    warn!(error = %e, "Failed to parse cancelled order");
                }
            }
        }

        Ok(orders)
    }
}

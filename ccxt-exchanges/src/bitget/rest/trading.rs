//! Trading operations for Bitget REST API.

use crate::bitget::{BitgetSymbolConverter, rest::builder::order_builder};

use super::super::{Bitget, parser, signed_request::HttpMethod};
use ccxt_core::{
    Error, MarketType, ParseError, Result, Trade,
    types::{Order, OrderRequest, OrderStatus},
};
use tracing::warn;

impl Bitget {
    /// Create a new order.
    /// Uses V3 unified API: POST /api/v3/trade/place-order
    pub async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        let market = self.base().market(&request.symbol).await?;

        // V3 统一接口：/api/v3/trade/place-order
        let path = "/api/v3/trade/place-order".to_string();

        // 构建额外参数
        let contract_params = self
            .build_order_extra_params(&request.symbol, market.market_type)
            .await?;

        // 使用辅助函数构建请求体 (带验证)
        let body =
            order_builder::build_create_order_payload(&request, &market, contract_params.as_ref())?;
        let response = self
            .signed_request(&path)
            .method(crate::bitget::signed_request::HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let order = parser::parse_order(data, Some(&market))?;

        // Auto-fetch complete order details if response was minimal (Pending status)
        // Bitget's place-order API only returns {orderId, clientOid}
        // If fetch_order fails (e.g., order already filled), return the minimal order
        if order.status == OrderStatus::Pending {
            let symbol = request.symbol.clone();
            match self.fetch_order(&order.id, &symbol).await {
                Ok(full_order) => return Ok(full_order),
                Err(_e) => {
                    // fetch_order failed - could be:
                    // 1. Order already filled (market order)
                    // 2. Network error
                    // 3. Order not found in open orders
                    // Return minimal order instead of failing
                    return Ok(order);
                }
            }
        }

        Ok(order)
    }

    /// Cancel an existing order.
    /// Uses V3 unified API: POST /api/v3/trade/cancel-order
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        // V3 统一接口：/api/v3/trade/cancel-order
        let path = "/api/v3/trade/cancel-order".to_string();

        // 构建请求体：V3 只需要 orderId 或 clientOid
        let mut body_map = serde_json::Map::new();
        body_map.insert(
            "orderId".to_string(),
            serde_json::Value::String(id.to_string()),
        );

        // 可选：添加 category 参数
        let product_type =
            crate::bitget::core::symbol::BitgetSymbolConverter::product_type_from_symbol(
                &market.symbol,
            );
        body_map.insert(
            "category".to_string(),
            serde_json::Value::String(product_type.to_string()),
        );

        let body = serde_json::Value::Object(body_map);

        let response = self
            .signed_request(&path)
            .method(crate::bitget::signed_request::HttpMethod::Post)
            .body(body)
            .execute()
            .await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // V3 响应: {"data": {"orderId": "...", "clientOid": "..."}}
        // 解析为 Order 对象
        parser::parse_order(data, Some(&market))
    }

    /// Fetch a single order by ID.
    /// Uses V3 unified API: GET /api/v3/trade/order-info
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;

        // V3 统一接口：/api/v3/trade/order-info
        let path = "/api/v3/trade/order-info".to_string();

        // 构建请求：V3 接口只需要 orderId 或 clientOid
        let request = self.signed_request(&path).param("orderId", id);

        // Try to fetch order
        match request.execute().await {
            Ok(response) => {
                if let Some(data) = response.get("data") {
                    // V3 响应: {"data": {...}} 单个对象
                    return parser::parse_order(data, Some(&market));
                }
            }
            Err(e) => {
                return Err(e);
            }
        }

        Err(Error::exchange("40007", "Order not found"))
    }

    /// Fetch open orders.
    /// Uses V3 unified API: GET /api/v3/trade/unfilled-orders
    pub async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // V3 统一接口：/api/v3/trade/unfilled-orders
        let path = "/api/v3/trade/unfilled-orders".to_string();

        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let actual_limit = limit.map_or(100, |l| l.min(100));

        let mut builder = self.signed_request(&path).param("limit", actual_limit);

        if let Some(m) = &market {
            builder = builder.param("symbol", &m.id);

            // 添加 category 参数 (V3 接口)
            let product_type =
                crate::bitget::core::symbol::BitgetSymbolConverter::product_type_from_symbol(
                    &m.symbol,
                );
            builder = builder.param("category", product_type);
        }

        if let Some(start_time) = since {
            builder = builder.param("startTime", start_time);
        }

        let response = builder.execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // V3 响应结构: {"data": {"list": [...], "cursor": "..."}}
        let orders_array = if let Some(list) = data.get("list") {
            list.as_array().ok_or_else(|| {
                Error::from(ParseError::invalid_format(
                    "list",
                    "Expected array of orders",
                ))
            })?
        } else {
            return Err(Error::from(ParseError::invalid_format(
                "data",
                "Expected object with list",
            )));
        };

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
    /// Uses V3 unified API: GET /api/v3/trade/history-orders
    pub async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        // V3 统一接口：/api/v3/trade/history-orders
        let path = "/api/v3/trade/history-orders".to_string();

        let market = if let Some(sym) = symbol {
            Some(self.base().market(sym).await?)
        } else {
            None
        };

        let actual_limit = limit.map_or(100, |l| l.min(100));

        let mut builder = self.signed_request(&path).param("limit", actual_limit);

        if let Some(m) = &market {
            builder = builder.param("symbol", &m.id);

            // 添加 category 参数 (V3 接口)
            let product_type =
                crate::bitget::core::symbol::BitgetSymbolConverter::product_type_from_symbol(
                    &m.symbol,
                );
            builder = builder.param("category", product_type);
        }

        if let Some(start_time) = since {
            builder = builder.param("startTime", start_time);
        }

        let response = builder.execute().await?;

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;
        println!("[DEBUG] fetch_history_orders V3 response: {:?}", data);

        // V3 响应结构: {"data": {"list": [...], "cursor": "..."}}
        let orders_array = if let Some(list) = data.get("list") {
            list.as_array().ok_or_else(|| {
                Error::from(ParseError::invalid_format(
                    "list",
                    "Expected array of orders",
                ))
            })?
        } else {
            return Err(Error::from(ParseError::invalid_format(
                "data",
                "Expected object with list",
            )));
        };

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
    /// Uses V3 unified API: POST /api/v3/trade/cancel-symbol-order
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

        // V3 统一接口：/api/v3/trade/cancel-symbol-order
        let path = "/api/v3/trade/cancel-symbol-order".to_string();

        // 构建请求体：category 必填，symbol 可选
        let product_type =
            crate::bitget::core::symbol::BitgetSymbolConverter::product_type_from_symbol(
                &market.symbol,
            );

        let mut body_map = serde_json::Map::new();
        body_map.insert(
            "category".to_string(),
            serde_json::Value::String(product_type.to_string()),
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

        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        // V3 响应: {"data": {"list": [{"orderId", "clientOid", "code", "msg"}]}}
        let list = data.get("list").and_then(|v| v.as_array()).ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "list",
                "Expected array of cancelled orders",
            ))
        })?;

        // 解析撤单结果
        let mut cancelled_orders = Vec::new();
        for item in list {
            if let Some(_order_id) = item.get("orderId").and_then(|v| v.as_str()) {
                // 如果 code 为空或 00000，表示撤单成功
                let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("");
                if code.is_empty() || code == "00000" {
                    // 构造 Order 对象
                    let order = parser::parse_order(item, Some(&market))?;
                    cancelled_orders.push(order);
                }
            }
        }

        Ok(cancelled_orders)
    }

    /// Fetch user's trade history.
    ///
    /// Uses Bitget GET `/api/v3/trade/fills` (private endpoint).
    ///
    /// Note: V3 API requires `category` parameter and supports:
    /// - Maximum 100 records per request
    /// - Time range: 90 days max, 30 days per query
    pub async fn fetch_account_trades(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market = self.base().market(symbol).await?;
        let category = BitgetSymbolConverter::product_type_from_symbol(symbol);

        // V3 limit: max 100, default 100
        let actual_limit = limit.map_or(100, |l| l.min(100));

        let mut builder = self
            .signed_request("/api/v3/trade/fills")
            .param("category", category)
            .param("symbol", &market.id)
            .param("limit", actual_limit);

        if let Some(start_time) = since {
            builder = builder.param("startTime", start_time);
        }

        let response = builder.execute().await?;

        // V3 response structure: data.list[]
        let data = response
            .get("data")
            .ok_or_else(|| Error::from(ParseError::missing_field("data")))?;

        let trades_array = data["list"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data.list",
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

    /// 构建订单的额外参数（category 和 hold_mode）
    ///
    /// - 合约订单：查询账户设置获取 holdMode
    /// - 现货订单：只设置 category=SPOT
    async fn build_order_extra_params(
        &self,
        symbol: &str,
        market_type: MarketType,
    ) -> Result<Option<std::collections::HashMap<String, String>>> {
        use std::collections::HashMap;

        if matches!(market_type, MarketType::Swap | MarketType::Futures) {
            // 合约订单
            let mut params = HashMap::new();

            // 获取产品类型 (V3 使用 category)
            let product_type =
                crate::bitget::core::symbol::BitgetSymbolConverter::product_type_from_symbol(
                    symbol,
                );
            params.insert("category".to_string(), product_type.to_string());

            // 查询持仓模式 - 使用账户设置 API，不需要有持仓也能获取
            // V3 API: /api/v3/account/settings 返回 holdMode 字段
            let hold_mode = match self.get_account_settings().await {
                Ok(settings) => settings["holdMode"]
                    .as_str()
                    .unwrap_or("one_way_mode")
                    .to_string(),
                Err(e) => {
                    // 查询失败，默认 one_way_mode
                    tracing::warn!(error = %e, "Failed to get account settings, defaulting to one_way_mode");
                    "one_way_mode".to_string()
                }
            };
            params.insert("hold_mode".to_string(), hold_mode);

            Ok(Some(params))
        } else {
            // 现货订单也需要 category
            let mut params = HashMap::new();
            params.insert("category".to_string(), "SPOT".to_string());
            Ok(Some(params))
        }
    }
}

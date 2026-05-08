//! HyperLiquid trading operations.
//!
//! This module contains trading methods including order creation,
//! cancellation, and order management.

use crate::hyperliquid::{HyperLiquid, parser};
use ccxt_core::{
    Error, ParseError, Result,
    types::{Order, OrderRequest, OrderSide, OrderStatus, OrderType},
};
use rust_decimal::Decimal;
use serde_json::Map;
use tracing::{info, warn};

impl HyperLiquid {
    /// Create a new order.
    ///
    /// # Asset Index Calculation
    ///
    /// According to HyperLiquid API documentation:
    /// - Perpetuals: use the index in the `meta.universe`
    /// - Spot: use `10000 + index` where index is in `spotMeta.universe`
    ///
    /// The market.id field already contains the correct asset_index:
    /// - Perpetuals: "0", "1", "2", ...
    /// - Spot: "10000", "10001", "10002", ...
    ///
    /// # Arguments
    ///
    /// * `request` - Order request built via [`OrderRequest::builder()`]
    ///
    /// # Returns
    ///
    /// Returns the created [`Order`] structure with order details.
    pub async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        let market = self.base().market(&request.symbol).await?;

        // 使用辅助函数构建请求体
        let action =
            crate::hyperliquid::rest::builder::build_create_order_payload(&request, &market)?;

        println!("Creating order: {:?}", action);
        // 使用 signed_action 发送（HyperLiquid 特殊签名方法）
        let response = self.signed_action(action).execute().await?;
        println!("Order response: {:?}", response);

        // 解析响应
        if let Some(statuses) = response["response"]["data"]["statuses"].as_array() {
            if let Some(status) = statuses.first() {
                if let Some(resting) = status.get("resting") {
                    return parser::parse_order(resting, Some(&market));
                }
                if let Some(filled) = status.get("filled") {
                    return parser::parse_order(filled, Some(&market));
                }
            }
        }

        Err(Error::exchange("-1", "Failed to parse order response"))
    }

    /// Cancel an order.
    pub async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market = self.base().market(symbol).await?;
        let asset_index: u32 = market
            .id
            .parse()
            .map_err(|_| Error::invalid_request("Invalid asset index format"))?;
        let order_id: u64 = id
            .parse()
            .map_err(|_| Error::invalid_request("Invalid order ID format"))?;

        // 使用辅助函数构建取消请求
        let action =
            crate::hyperliquid::rest::builder::build_cancel_order_payload(order_id, asset_index);

        let _response = self.signed_action(action).execute().await?;

        // 返回取消的订单对象
        Ok(Order::new(
            id.to_string(),
            market.symbol.clone(),
            OrderType::Limit,
            OrderSide::Buy, // Side is unknown for cancel response
            Decimal::ZERO,
            None,
            OrderStatus::Cancelled,
        ))
    }

    /// Cancel all orders for a symbol.
    pub async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        let _address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to cancel orders"))?;

        // First fetch open orders
        let open_orders = self.fetch_open_orders(symbol, None, None).await?;

        if open_orders.is_empty() {
            return Ok(Vec::new());
        }

        // Build cancel requests for all orders
        let mut cancels = Vec::new();
        for order in &open_orders {
            let market = self.base().market(&order.symbol).await?;
            let asset_index: u32 = market
                .id
                .parse()
                .map_err(|_| Error::invalid_request("Invalid asset index format"))?;
            let order_id: u64 = order.id.parse().unwrap_or(0);

            // 使用辅助函数构建每个取消请求
            let cancel_action = crate::hyperliquid::rest::builder::build_cancel_order_payload(
                order_id,
                asset_index,
            );
            // 提取 cancels 数组中的第一个元素
            if let Some(cancel_list) = cancel_action["cancels"].as_array() {
                if let Some(cancel) = cancel_list.first() {
                    cancels.push(cancel.clone());
                }
            }
        }

        let action = {
            let mut map = Map::new();
            map.insert(
                "type".to_string(),
                serde_json::Value::String("cancel".to_string()),
            );
            map.insert("cancels".to_string(), serde_json::Value::Array(cancels));
            serde_json::Value::Object(map)
        };

        let _response = self.signed_action(action).execute().await?;

        // Return the orders that were canceled
        let canceled_orders: Vec<Order> = open_orders
            .into_iter()
            .map(|mut o| {
                o.status = ccxt_core::types::OrderStatus::Cancelled;
                o
            })
            .collect();

        info!("Canceled {} orders", canceled_orders.len());

        Ok(canceled_orders)
    }

    /// Fetch open positions.
    pub async fn fetch_positions(
        &self,
        symbols: Option<Vec<String>>,
    ) -> Result<Vec<ccxt_core::types::Position>> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch positions"))?;

        let response = self
            .info_request("clearinghouseState", {
                let mut map = Map::new();
                map.insert(
                    "user".to_string(),
                    serde_json::Value::String(address.to_string()),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        let asset_positions = response["assetPositions"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("assetPositions")))?;

        // Get timestamp from response (HyperLiquid provides "time" field)
        let response_timestamp = response.get("time").and_then(|v| v.as_i64());

        let mut positions = Vec::new();
        for pos_data in asset_positions {
            let position = pos_data.get("position").unwrap_or(pos_data);

            let coin = position["coin"].as_str().unwrap_or("");
            let symbol = format!("{}/USDC:USDC", coin);

            // Filter by symbols if provided
            if let Some(ref syms) = symbols {
                if !syms.contains(&symbol) {
                    continue;
                }
            }

            let szi = position["szi"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            // Skip zero positions
            if szi.abs() < 1e-10 {
                continue;
            }

            let entry_px = position["entryPx"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());
            let liquidation_px = position["liquidationPx"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());
            let unrealized_pnl = position["unrealizedPnl"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());
            let margin_used = position["marginUsed"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok());

            let leverage_info = position.get("leverage");
            let leverage = leverage_info
                .and_then(|l| l["value"].as_str())
                .and_then(|s| s.parse::<f64>().ok());
            let margin_mode = leverage_info
                .and_then(|l| l["type"].as_str())
                .map(|t| if t == "cross" { "cross" } else { "isolated" }.to_string());

            let side = if szi > 0.0 { "long" } else { "short" };

            positions.push(ccxt_core::types::Position {
                info: pos_data.clone(),
                id: None,
                symbol,
                side: Some(side.to_string()),
                position_side: None,
                dual_side_position: None,
                contracts: Some(szi.abs()),
                contract_size: Some(1.0),
                entry_price: entry_px,
                mark_price: None,
                notional: None,
                leverage,
                collateral: margin_used,
                initial_margin: margin_used,
                initial_margin_percentage: None,
                maintenance_margin: None,
                maintenance_margin_percentage: None,
                unrealized_pnl,
                realized_pnl: None,
                liquidation_price: liquidation_px,
                margin_ratio: None,
                margin_mode,
                hedged: None,
                percentage: None,
                timestamp: response_timestamp,
                datetime: None,
            });
        }

        Ok(positions)
    }

    /// Fetch open orders.
    pub async fn fetch_open_orders(
        &self,
        symbol: Option<&str>,
        _since: Option<i64>,
        _limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch orders"))?;

        let response = self
            .info_request("openOrders", {
                let mut map = Map::new();
                map.insert(
                    "user".to_string(),
                    serde_json::Value::String(address.to_string()),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        let orders_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        let mut orders = Vec::new();
        for order_data in orders_array {
            match parser::parse_order(order_data, None) {
                Ok(order) => {
                    // Filter by symbol if provided
                    if let Some(sym) = symbol {
                        if order.symbol.as_str() != sym {
                            continue;
                        }
                    }
                    orders.push(order);
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse order");
                }
            }
        }

        Ok(orders)
    }

    /// Fetch a specific order by ID.
    ///
    /// Uses orderStatus endpoint to query order details.
    /// Note: HyperLiquid requires both user address and order ID.
    pub async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch order"))?;

        let market = self.base().market(symbol).await?;

        let response = self
            .info_request("orderStatus", {
                let mut map = Map::new();
                map.insert(
                    "user".to_string(),
                    serde_json::Value::String(address.to_string()),
                );
                map.insert(
                    "oid".to_string(),
                    serde_json::Value::Number(
                        id.parse()
                            .map_err(|_| Error::invalid_request("Invalid order ID format"))?,
                    ),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        // Parse the order status response
        if let Some(status_data) = response.get("order") {
            return parser::parse_order(status_data, Some(&market));
        }

        Err(Error::exchange(
            "ORDER_NOT_FOUND",
            &format!("Order {} not found for symbol {}", id, symbol),
        ))
    }

    /// Fetch closed orders (order history with fills).
    ///
    /// Uses userFills endpoint to query historical fills and reconstructs orders.
    /// Note: HyperLiquid doesn't have a direct "closed orders" endpoint,
    /// so we query fills and group them by order ID.
    pub async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        let address = self
            .wallet_address()
            .ok_or_else(|| Error::authentication("Private key required to fetch closed orders"))?;

        let response = self
            .info_request("userFills", {
                let mut map = Map::new();
                map.insert(
                    "user".to_string(),
                    serde_json::Value::String(address.to_string()),
                );
                serde_json::Value::Object(map)
            })
            .await?;

        let fills_array = response
            .as_array()
            .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

        // Group fills by order ID to reconstruct orders
        use std::collections::HashMap;
        let mut orders_map: HashMap<String, Order> = HashMap::new();

        for fill_data in fills_array {
            let order_id = fill_data["oid"]
                .as_u64()
                .map(|n| n.to_string())
                .unwrap_or_default();

            if order_id.is_empty() {
                continue;
            }

            // Parse the fill timestamp for filtering
            let fill_timestamp = fill_data["time"].as_i64();

            // Filter by time range if specified
            if let Some(since_ts) = since {
                if let Some(ts) = fill_timestamp {
                    if ts < since_ts {
                        continue;
                    }
                }
            }

            // Get or create order entry
            let order = orders_map.entry(order_id.clone()).or_insert_with(|| {
                // Try to parse basic order info from fill data
                let coin = fill_data["coin"].as_str().unwrap_or("");
                let symbol_str = if coin.is_empty() {
                    symbol.map(|s| s.to_string()).unwrap_or_default()
                } else {
                    format!("{}/USDC:USDC", coin)
                };

                Order {
                    id: order_id,
                    client_order_id: None,
                    timestamp: fill_timestamp,
                    datetime: fill_timestamp.and_then(|ts| {
                        chrono::DateTime::from_timestamp_millis(ts).map(|dt| dt.to_rfc3339())
                    }),
                    last_trade_timestamp: fill_timestamp,
                    status: ccxt_core::types::OrderStatus::Closed,
                    symbol: ccxt_core::types::Symbol::new_unchecked(&symbol_str),
                    order_type: ccxt_core::types::OrderType::Limit,
                    time_in_force: None,
                    side: match fill_data["side"].as_str() {
                        Some("B") => ccxt_core::types::OrderSide::Buy,
                        _ => ccxt_core::types::OrderSide::Sell,
                    },
                    price: None,
                    average: None,
                    amount: Decimal::ZERO,
                    filled: None,
                    remaining: None,
                    cost: None,
                    trades: None,
                    fee: None,
                    post_only: None,
                    reduce_only: None,
                    trigger_price: None,
                    stop_price: None,
                    take_profit_price: None,
                    stop_loss_price: None,
                    trailing_delta: None,
                    trailing_percent: None,
                    activation_price: None,
                    callback_rate: None,
                    working_type: None,
                    fees: Some(Vec::new()),
                    info: std::collections::HashMap::new(),
                }
            });

            // Aggregate fill data
            let fill_sz = fill_data["sz"]
                .as_str()
                .and_then(|s| s.parse::<Decimal>().ok())
                .unwrap_or(Decimal::ZERO);
            let fill_px = fill_data["px"]
                .as_str()
                .and_then(|s| s.parse::<Decimal>().ok())
                .unwrap_or(Decimal::ZERO);

            order.filled = Some(order.filled.unwrap_or(Decimal::ZERO) + fill_sz);
            order.cost = Some(order.cost.unwrap_or(Decimal::ZERO) + (fill_sz * fill_px));
        }

        // Convert map to vector
        let mut orders: Vec<Order> = orders_map.into_values().collect();

        // Filter by symbol if provided
        if let Some(sym) = symbol {
            orders.retain(|o| o.symbol.as_str() == sym);
        }

        // Sort by timestamp (newest first)
        orders.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        if let Some(limit) = limit {
            orders.truncate(limit as usize);
        }

        // Calculate average price and remaining for each order
        for order in &mut orders {
            if order.filled.unwrap_or(Decimal::ZERO) > Decimal::ZERO {
                order.amount = order.filled.unwrap_or(Decimal::ZERO); // For closed orders, amount = filled
                order.remaining = Some(Decimal::ZERO);
                if let (Some(cost), Some(filled)) = (order.cost, order.filled) {
                    order.average = Some(cost / filled);
                }
            }
        }

        Ok(orders)
    }
}

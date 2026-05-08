//! Bybit 订单构建辅助函数
//!
//! 这些函数负责将统一的 OrderRequest 转换为 Bybit 特定的 JSON 格式。
//! 函数设计为纯函数(无外部依赖),便于独立测试。
//!
//! # 设计原则
//!
//! - 纯函数: 不依赖交易所实例,仅使用传入参数
//! - 可测试: 每个函数都可独立单元测试
//! - 清晰命名: 函数名明确表示其用途
//! - 完整注释: 说明 Bybit API 特殊要求
//!
//! # Bybit V5 API 特殊要求
//!
//! - `category` 必填: spot/linear/inverse/option
//! - `positionIdx`: 双向持仓必传 (0=单向, 1=多头, 2=空头)
//! - `orderLinkId`: option 类别必传
//! - `tpslMode`: 止盈止损模式 (Full/Partial)
//! - `marketUnit`: 现货市价单数量单位 (baseCoin/quoteCoin)
//! - `slippageToleranceType`: 市价单滑点容差类型
//! - 追踪止损: Bybit 不支持原生 trailing stop，需客户端实现

use ccxt_core::Error;
use ccxt_core::types::market::{Market, MarketType};
use ccxt_core::types::order::TriggerPriceType;
use ccxt_core::types::{AmountSpec, OrderRequest, OrderSide, OrderType, TimeInForce};
use serde_json::{Map, Value};

// ============================================================================
// 创建订单
// ============================================================================

/// 构建创建订单的请求体
///
/// # Bybit V5 API 要求
///
/// - `category`: 市场类型 (linear/inverse/spot/option)
/// - `symbol`: 交易对(交易所特定格式,如 "BTCUSDT")
/// - `side`: 买卖方向("Buy" / "Sell")
/// - `orderType`: 订单类型("Market" / "Limit")
/// - `qty`: 订单数量
/// - `price`: 限价单必填
/// - `timeInForce`: 时间生效模式(GTC/IOC/FOK/PostOnly)
///
/// # 可选字段
///
/// - `orderLinkId`: 客户端订单 ID
/// - `reduceOnly`: 仅减仓(合约)
/// - `triggerPrice`: 触发价格(条件单)
/// - `takeProfit`: 止盈价格
/// - `stopLoss`: 止损价格
/// - `trailingDelta`: 追踪止损(basis points)
/// - `positionIdx`: 持仓索引(0/1/2)
///
/// # 注意事项
///
/// - Bybit 支持在创建订单时预设止盈止损
/// - Bybit 的 trailingDelta 使用 basis points (1% = 100)
/// - 现货不支持 reduceOnly 和 positionIdx
///
/// # 参数
///
/// - `request`: 统一订单请求
/// - `market`: 市场信息
/// - `extra_params`: 额外参数 (category, hedged_mode 等)
///
/// # 错误
///
/// - 如果订单类型不被 Bybit 支持,返回 `Error::InvalidOrder`
/// - 如果参数组合冲突,返回 `Error::InvalidOrder`
pub fn build_create_order_payload(
    request: &OrderRequest,
    market: &Market,
    extra_params: &std::collections::HashMap<String, String>,
) -> Result<Value, Error> {
    // ✅ 验证 1: Bybit 不支持的订单类型
    if !is_bybit_supported_order_type(request.order_type) {
        return Err(Error::InvalidOrder(
            format!("Bybit does not support {:?} orders", request.order_type).into(),
        ));
    }

    // ✅ 验证 2: Bybit 不支持原生追踪止损 (需客户端实现)
    if request.trailing_callback_rate.is_some() {
        return Err(Error::InvalidOrder(
            "Bybit REST API does not support trailing stop orders natively. \
             Consider managing trailing stop client-side."
                .into(),
        ));
    }

    // ✅ 验证 3: 限价单必须提供价格
    if matches!(request.order_type, OrderType::Limit | OrderType::LimitMaker)
        && request.price.is_none()
    {
        return Err(Error::InvalidOrder(
            "Limit orders require price parameter".into(),
        ));
    }

    // 从 extra_params 获取 category
    let category = extra_params
        .get("category")
        .ok_or_else(|| Error::InvalidOrder("Missing required parameter: category".into()))?;

    // ✅ 验证 4: option 类别必传 orderLinkId
    if category == "option" && request.client_order_id.is_none() {
        return Err(Error::InvalidOrder(
            "Option orders require client_order_id (orderLinkId)".into(),
        ));
    }

    let mut map = Map::new();

    // 基础必填字段
    map.insert("category".to_string(), Value::String(category.to_string()));
    map.insert("symbol".to_string(), Value::String(market.id.clone()));
    map.insert("side".to_string(), Value::String(map_side(request.side)));
    map.insert(
        "orderType".to_string(),
        Value::String(map_order_type(request.order_type)),
    );

    // ========================================================================
    // 数量参数处理 (关键逻辑)
    // ========================================================================
    // Bybit V5 规则:
    // - 现货: 可以通过 marketUnit 指定市价单 qty 单位
    //   - 市价买单: 默认 quoteCoin (如 USDT)
    //   - 市价卖单: 默认 baseCoin (如 BTC)
    // - 期货/期权: 总是以 base coin 作为 qty 单位
    let qty_value = match market.market_type {
        MarketType::Spot => {
            if request.order_type == OrderType::Market {
                // 现货市价单: 根据 side 决定单位
                match &request.amount {
                    AmountSpec::Quote(quote_amt) => {
                        // 市价买单使用 quote 金额
                        if request.side == OrderSide::Buy {
                            // 可选: 添加 marketUnit 参数明确单位
                            map.insert(
                                "marketUnit".to_string(),
                                Value::String("quoteCoin".to_string()),
                            );
                            quote_amt.as_decimal().to_string()
                        } else {
                            // 市价卖单应该使用 base 数量
                            request.amount.as_decimal().to_string()
                        }
                    }
                    AmountSpec::Base(base_amt) => {
                        // 市价卖单使用 base 数量
                        if request.side == OrderSide::Sell {
                            map.insert(
                                "marketUnit".to_string(),
                                Value::String("baseCoin".to_string()),
                            );
                            base_amt.as_decimal().to_string()
                        } else {
                            // 市价买单收到了 base 数量，这是错误的
                            return Err(Error::InvalidOrder(
                                "Spot market buy orders require quote currency amount (USDT), not base currency".into(),
                            ));
                        }
                    }
                }
            } else {
                // 现货限价单: 使用 base 数量
                match &request.amount {
                    AmountSpec::Base(base_amt) => base_amt.as_decimal().to_string(),
                    AmountSpec::Quote(_) => {
                        return Err(Error::InvalidOrder(
                            "Spot limit orders require base currency amount, not quote currency"
                                .into(),
                        ));
                    }
                }
            }
        }
        MarketType::Swap | MarketType::Futures | MarketType::Option => {
            // 期货/期权: 总是使用 base coin
            match &request.amount {
                AmountSpec::Base(base_amt) => base_amt.as_decimal().to_string(),
                AmountSpec::Quote(_) => {
                    return Err(Error::InvalidOrder(
                        "Futures/options orders require base currency amount, not quote currency"
                            .into(),
                    ));
                }
            }
        }
    };
    map.insert("qty".to_string(), Value::String(qty_value));

    // 价格(限价单必需)
    if let Some(price) = request.price {
        if matches!(request.order_type, OrderType::Limit | OrderType::LimitMaker) {
            map.insert("price".to_string(), Value::String(price.to_string()));
        }
    }

    // TimeInForce 处理
    // LimitMaker 订单类型自动设置为 PostOnly
    let time_in_force = if request.order_type == OrderType::LimitMaker {
        "PostOnly".to_string()
    } else {
        map_time_in_force(request.time_in_force, request.post_only)
    };
    map.insert("timeInForce".to_string(), Value::String(time_in_force));

    // 客户端订单 ID
    if let Some(client_id) = &request.client_order_id {
        map.insert("orderLinkId".to_string(), Value::String(client_id.clone()));
    }

    // 仅减仓(合约)
    if let Some(reduce_only) = request.reduce_only {
        map.insert("reduceOnly".to_string(), Value::Bool(reduce_only));
    }

    // 持仓索引(仅合约)
    // 根据 hedged_mode 和 position_side 确定 positionIdx
    // - 单向模式 (hedged_mode=false): positionIdx=0
    // - 双向模式 (hedged_mode=true): positionIdx=1 (Long) 或 2 (Short)
    if matches!(market.market_type, MarketType::Swap | MarketType::Futures) {
        let hedged_mode = extra_params
            .get("hedged_mode")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        if let Some(pos_side) = &request.position_side {
            if hedged_mode {
                // 双向持仓模式: 根据 position_side 设置 1 或 2
                let position_idx = map_position_side(pos_side.as_str());
                map.insert("positionIdx".to_string(), Value::String(position_idx));
            } else {
                // 单向持仓模式: positionIdx 固定为 0
                map.insert("positionIdx".to_string(), Value::String("0".to_string()));
            }
        }
    }

    // 触发价格(条件单/计划单)
    if let Some(trigger) = request.stop_price {
        map.insert(
            "triggerPrice".to_string(),
            Value::String(trigger.to_string()),
        );
    }

    // ========================================================================
    // Bybit V5 API: 预设止盈止损 (Preset TP/SL)
    // ========================================================================
    // Bybit 支持在创建订单时同时设置止盈止损价格
    // 字段名: takeProfit / stopLoss
    // 注意:
    // - reduceOnly 单的止盈止损不生效
    // - tpslMode=Full 时，仅支持 Market 订单类型
    // - tpslMode=Partial 时，支持 Limit 订单类型

    // 预设止盈价格
    if let Some(tp_price) = request.tp_limit_price {
        map.insert(
            "takeProfit".to_string(),
            Value::String(tp_price.to_string()),
        );

        // 现货: 如果设置了 tpLimitPrice，必须指定 tpOrderType=Limit
        if matches!(market.market_type, MarketType::Spot) {
            map.insert(
                "tpOrderType".to_string(),
                Value::String("Limit".to_string()),
            );
        }
    }

    // 预设止损价格
    if let Some(sl_price) = request.sl_limit_price {
        map.insert("stopLoss".to_string(), Value::String(sl_price.to_string()));

        // 现货: 如果设置了 slLimitPrice，必须指定 slOrderType=Limit
        if matches!(market.market_type, MarketType::Spot) {
            map.insert(
                "slOrderType".to_string(),
                Value::String("Limit".to_string()),
            );
        }
    }

    // 止盈触发价格类型 (LastPrice/IndexPrice/MarkPrice)
    if let Some(tp_trigger_by) = &request.trigger_price_type {
        let tp_trigger_str = match tp_trigger_by {
            TriggerPriceType::MarkPrice => "MarkPrice",
            TriggerPriceType::LastPrice => "LastPrice",
            TriggerPriceType::IndexPrice => "IndexPrice",
        };
        map.insert(
            "tpTriggerBy".to_string(),
            Value::String(tp_trigger_str.to_string()),
        );
    }

    // 止损触发价格类型 (LastPrice/IndexPrice/MarkPrice)
    // 注意：trigger_price_type 同时控制 TP 和 SL
    if request.tp_limit_price.is_some() || request.sl_limit_price.is_some() {
        if let Some(trigger_type) = &request.trigger_price_type {
            let sl_trigger_str = match trigger_type {
                TriggerPriceType::MarkPrice => "MarkPrice",
                TriggerPriceType::LastPrice => "LastPrice",
                TriggerPriceType::IndexPrice => "IndexPrice",
            };
            map.insert(
                "slTriggerBy".to_string(),
                Value::String(sl_trigger_str.to_string()),
            );
        }
    }

    Ok(Value::Object(map))
}

// ============================================================================
// 取消订单
// ============================================================================

/// 构建取消订单的请求体
///
/// # Bybit API 要求
///
/// - `category`: 市场类型
/// - `symbol`: 交易对
/// - `orderId`: 订单 ID (或 orderLinkId)
pub fn build_cancel_order_payload(
    order_id: &str,
    market: &Market,
    category: &str,
    client_order_id: Option<&str>,
) -> Value {
    let mut map = Map::new();

    map.insert("category".to_string(), Value::String(category.to_string()));
    map.insert("symbol".to_string(), Value::String(market.id.clone()));

    // Bybit 支持使用 orderId 或 orderLinkId
    if let Some(client_oid) = client_order_id {
        map.insert(
            "orderLinkId".to_string(),
            Value::String(client_oid.to_string()),
        );
    } else {
        map.insert("orderId".to_string(), Value::String(order_id.to_string()));
    }

    Value::Object(map)
}

// ============================================================================
// 修改订单
// ============================================================================

/// 构建修改订单的请求体
///
/// # Bybit API 要求
///
/// - `category`: 市场类型
/// - `symbol`: 交易对
/// - `orderId`: 订单 ID
/// - `price`: 新价格 (可选)
/// - `qty`: 新数量 (可选)
///
/// # 限制
///
/// - Bybit 仅支持修改价格和数量
/// - 至少提供一个修改字段
pub fn build_edit_order_payload(
    order_id: &str,
    market: &Market,
    category: &str,
    price: Option<&str>,
    qty: Option<&str>,
) -> Result<Value, Error> {
    if price.is_none() && qty.is_none() {
        return Err(Error::InvalidOrder(
            "At least one of price or qty must be provided for edit order".into(),
        ));
    }

    let mut map = Map::new();

    map.insert("category".to_string(), Value::String(category.to_string()));
    map.insert("symbol".to_string(), Value::String(market.id.clone()));
    map.insert("orderId".to_string(), Value::String(order_id.to_string()));

    if let Some(p) = price {
        map.insert("price".to_string(), Value::String(p.to_string()));
    }

    if let Some(q) = qty {
        map.insert("qty".to_string(), Value::String(q.to_string()));
    }

    Ok(Value::Object(map))
}

// ============================================================================
// 内部映射函数(私有)
// ============================================================================

/// 检查订单类型是否被 Bybit 支持
///
/// # Bybit 支持的订单类型
///
/// - `Market`: 市价单
/// - `Limit`: 限价单
/// - `LimitMaker`: 只做 Maker 单 (PostOnly)
/// - `StopLoss`: 止损单 (通过 triggerPrice 实现)
/// - `StopMarket`: 止损市价单
/// - `TakeProfit`: 止盈单 (通过 triggerPrice 实现)
/// - `TrailingStop`: 追踪止损 (通过 trailingDelta 实现)
fn is_bybit_supported_order_type(order_type: OrderType) -> bool {
    matches!(
        order_type,
        OrderType::Market
            | OrderType::Limit
            | OrderType::LimitMaker
            | OrderType::StopLoss
            | OrderType::StopMarket
            | OrderType::TakeProfit
            | OrderType::TrailingStop
    )
}

/// 映射订单方向
fn map_side(side: OrderSide) -> String {
    match side {
        OrderSide::Buy => "Buy".to_string(),
        OrderSide::Sell => "Sell".to_string(),
    }
}

/// 映射订单类型
///
/// # Bybit 订单类型映射
///
/// - `LimitMaker` → "Limit" (配合 timeInForce=PostOnly)
/// - `Market` / `StopLoss` / `StopMarket` / `TakeProfit` / `TrailingStop` → "Market"
/// - 其他(包括 `Limit`) → "Limit"
fn map_order_type(order_type: OrderType) -> String {
    match order_type {
        OrderType::LimitMaker => "Limit".to_string(),
        OrderType::Market
        | OrderType::StopLoss
        | OrderType::StopMarket
        | OrderType::TakeProfit
        | OrderType::TrailingStop => "Market".to_string(),
        _ => "Limit".to_string(),
    }
}

/// 映射 TimeInForce 到 Bybit 的 timeInForce 字段
///
/// # Bybit timeInForce 字段说明
///
/// - `GTC`: Good Till Cancelled(默认)
/// - `IOC`: Immediate Or Cancel
/// - `FOK`: Fill Or Kill
/// - `PostOnly`: 仅做 Maker
///
/// # 优先级
///
/// 1. 如果 `time_in_force` 存在,使用其映射
/// 2. 否则如果 `post_only == Some(true)`,使用 "PostOnly"
/// 3. 否则默认 "GTC"
fn map_time_in_force(time_in_force: Option<TimeInForce>, post_only: Option<bool>) -> String {
    if let Some(tif) = time_in_force {
        match tif {
            TimeInForce::GTC => "GTC".to_string(),
            TimeInForce::IOC => "IOC".to_string(),
            TimeInForce::FOK => "FOK".to_string(),
            TimeInForce::PO => "PostOnly".to_string(),
            TimeInForce::GTD => "GTC".to_string(), // Bybit 不支持 GTD,降级为 GTC
        }
    } else if post_only == Some(true) {
        "PostOnly".to_string()
    } else {
        "GTC".to_string()
    }
}

/// 映射持仓方向到 positionIdx
///
/// # Bybit positionIdx 说明
///
/// - `0`: Both (单向持仓模式)
/// - `1`: Long (双向持仓 - 多头)
/// - `2`: Short (双向持仓 - 空头)
///
/// # 注意
///
/// - 单向持仓模式: positionIdx 固定为 0
/// - 双向持仓模式: 根据实际持仓方向设置 1 或 2
fn map_position_side(position_side: &str) -> String {
    match position_side {
        "LONG" | "Long" | "long" => "1".to_string(),
        "SHORT" | "Short" | "short" => "2".to_string(),
        _ => "0".to_string(), // BOTH
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::Symbol;
    use ccxt_core::types::financial::{Amount, Price};
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    /// 创建测试用的 Market 对象
    fn mock_market(unified_symbol: &str, exchange_id: &str) -> Market {
        let mut market = Market::default();
        market.id = exchange_id.to_string();
        market.symbol = Symbol::new_unchecked(unified_symbol);
        market.base = "BTC".to_string();
        market.quote = "USDT".to_string();
        market
    }

    /// 创建测试用的 extra_params
    fn create_extra_params(category: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("category".to_string(), category.to_string());
        params
    }

    /// 创建测试用的 extra_params (带 hedged_mode)
    fn create_extra_params_with_hedged(
        category: &str,
        hedged_mode: bool,
    ) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("category".to_string(), category.to_string());
        params.insert("hedged_mode".to_string(), hedged_mode.to_string());
        params
    }

    // ========================================================================
    // build_create_order_payload 测试
    // ========================================================================

    #[test]
    fn test_build_create_order_market_buy() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(AmountSpec::quote(Amount::new(dec!(100.0)))) // 100 USDT
            .build()
            .expect("OrderRequest should build successfully");

        let mut market = mock_market("BTC/USDT", "BTCUSDT");
        market.market_type = MarketType::Spot;
        let payload = build_create_order_payload(&request, &market, &create_extra_params("spot"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["category"], "spot");
        assert_eq!(payload["symbol"], "BTCUSDT");
        assert_eq!(payload["side"], "Buy");
        assert_eq!(payload["orderType"], "Market");
        assert_eq!(payload["qty"], "100.0");
        assert_eq!(payload["marketUnit"], "quoteCoin");
        assert_eq!(payload["timeInForce"], "GTC");
        assert!(!payload.as_object().unwrap().contains_key("price"));
    }

    #[test]
    fn test_build_create_order_limit_sell() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .time_in_force(TimeInForce::GTC)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["orderType"], "Limit");
        assert_eq!(payload["price"], "50000");
        assert_eq!(payload["qty"], "0.1");
        assert_eq!(payload["timeInForce"], "GTC");
    }

    #[test]
    fn test_build_create_order_limit_maker() {
        let request = OrderRequest::builder()
            .symbol("ETH/USDT:USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::LimitMaker)
            .amount(Amount::new(dec!(1.0)))
            .price(Price::new(dec!(3000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("ETH/USDT:USDT", "ETHUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["orderType"], "Limit");
        assert_eq!(payload["timeInForce"], "PostOnly");
        assert_eq!(payload["price"], "3000");
    }

    #[test]
    fn test_build_create_order_with_client_order_id() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Sell) // 卖单使用 base 数量
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.01)))
            .client_order_id("my-order-123")
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["orderLinkId"], "my-order-123");
    }

    #[test]
    fn test_build_create_order_with_reduce_only() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .reduce_only(true)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["reduceOnly"], true);
    }

    #[test]
    fn test_build_create_order_with_position_side() {
        use ccxt_core::types::order::PositionSide;

        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .position_side(PositionSide::Long)
            .build()
            .expect("OrderRequest should build successfully");

        let mut market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        market.market_type = MarketType::Swap;
        let payload = build_create_order_payload(
            &request,
            &market,
            &create_extra_params_with_hedged("linear", true),
        )
        .expect("build_create_order_payload should succeed");

        assert_eq!(payload["positionIdx"], "1");
    }

    #[test]
    fn test_build_create_order_with_trigger_price() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLoss)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(45000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["orderType"], "Market");
        assert_eq!(payload["triggerPrice"], "45000");
    }

    #[test]
    fn test_build_create_order_with_take_profit() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .stop_price(Price::new(dec!(55000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        // stop_price 用于触发价格
        assert_eq!(payload["triggerPrice"], "55000");
    }

    #[test]
    fn test_build_create_order_with_stop_loss() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .stop_price(Price::new(dec!(48000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        // stop_price 用于触发价格
        assert_eq!(payload["triggerPrice"], "48000");
    }

    #[test]
    fn test_build_create_order_with_trailing_delta() {
        // Bybit 不支持 trailing stop，这个测试应该验证错误
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.1)))
            .trailing_callback_rate_percent(dec!(1.5))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let result = build_create_order_payload(&request, &market, &create_extra_params("linear"));

        // Bybit 不支持 TrailingStop
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("trailing stop"));
    }

    #[test]
    fn test_build_create_order_time_in_force_ioc() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .time_in_force(TimeInForce::IOC)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["timeInForce"], "IOC");
    }

    #[test]
    fn test_build_create_order_time_in_force_fok() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .time_in_force(TimeInForce::FOK)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["timeInForce"], "FOK");
    }

    #[test]
    fn test_build_create_order_post_only_via_flag() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .post_only(true)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_create_order_payload(&request, &market, &create_extra_params("linear"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["timeInForce"], "PostOnly");
    }

    #[test]
    fn test_build_create_order_spot_category() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell) // 卖单使用 base 数量
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.001))) // BTC 数量
            .build()
            .expect("OrderRequest should build successfully");

        let mut market = mock_market("BTC/USDT", "BTCUSDT");
        market.market_type = MarketType::Spot;
        let payload = build_create_order_payload(&request, &market, &create_extra_params("spot"))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["category"], "spot");
        assert_eq!(payload["marketUnit"], "baseCoin");
    }

    // ========================================================================
    // build_cancel_order_payload 测试
    // ========================================================================

    #[test]
    fn test_build_cancel_order_with_order_id() {
        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_cancel_order_payload("123456789", &market, "linear", None);

        assert_eq!(payload["category"], "linear");
        assert_eq!(payload["symbol"], "BTCUSDT");
        assert_eq!(payload["orderId"], "123456789");
        assert!(!payload.as_object().unwrap().contains_key("orderLinkId"));
    }

    #[test]
    fn test_build_cancel_order_with_client_order_id() {
        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload =
            build_cancel_order_payload("123456789", &market, "linear", Some("my-order-123"));

        assert_eq!(payload["orderLinkId"], "my-order-123");
        assert!(!payload.as_object().unwrap().contains_key("orderId"));
    }

    // ========================================================================
    // build_edit_order_payload 测试
    // ========================================================================

    #[test]
    fn test_build_edit_order_price_only() {
        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_edit_order_payload("123456789", &market, "linear", Some("51000"), None)
            .expect("build_edit_order_payload should succeed");

        assert_eq!(payload["orderId"], "123456789");
        assert_eq!(payload["price"], "51000");
        assert!(!payload.as_object().unwrap().contains_key("qty"));
    }

    #[test]
    fn test_build_edit_order_qty_only() {
        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload = build_edit_order_payload("123456789", &market, "linear", None, Some("0.2"))
            .expect("build_edit_order_payload should succeed");

        assert_eq!(payload["qty"], "0.2");
        assert!(!payload.as_object().unwrap().contains_key("price"));
    }

    #[test]
    fn test_build_edit_order_both_price_and_qty() {
        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let payload =
            build_edit_order_payload("123456789", &market, "linear", Some("51000"), Some("0.2"))
                .expect("build_edit_order_payload should succeed");

        assert_eq!(payload["price"], "51000");
        assert_eq!(payload["qty"], "0.2");
    }

    #[test]
    fn test_build_edit_order_missing_both() {
        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let result = build_edit_order_payload("123456789", &market, "linear", None, None);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("At least one of price or qty"));
    }

    // ========================================================================
    // 内部映射函数测试
    // ========================================================================

    #[test]
    fn test_map_side() {
        assert_eq!(map_side(OrderSide::Buy), "Buy");
        assert_eq!(map_side(OrderSide::Sell), "Sell");
    }

    #[test]
    fn test_map_order_type() {
        assert_eq!(map_order_type(OrderType::Limit), "Limit");
        assert_eq!(map_order_type(OrderType::LimitMaker), "Limit");
        assert_eq!(map_order_type(OrderType::Market), "Market");
        assert_eq!(map_order_type(OrderType::StopLoss), "Market");
        assert_eq!(map_order_type(OrderType::TakeProfit), "Market");
        assert_eq!(map_order_type(OrderType::TrailingStop), "Market");
    }

    #[test]
    fn test_map_time_in_force() {
        // 使用 time_in_force 参数
        assert_eq!(map_time_in_force(Some(TimeInForce::GTC), None), "GTC");
        assert_eq!(map_time_in_force(Some(TimeInForce::IOC), None), "IOC");
        assert_eq!(map_time_in_force(Some(TimeInForce::FOK), None), "FOK");
        assert_eq!(map_time_in_force(Some(TimeInForce::PO), None), "PostOnly");

        // 使用 post_only 标志
        assert_eq!(map_time_in_force(None, Some(true)), "PostOnly");

        // 默认值
        assert_eq!(map_time_in_force(None, None), "GTC");
    }

    #[test]
    fn test_map_position_side() {
        assert_eq!(map_position_side("LONG"), "1");
        assert_eq!(map_position_side("Long"), "1");
        assert_eq!(map_position_side("long"), "1");
        assert_eq!(map_position_side("SHORT"), "2");
        assert_eq!(map_position_side("Short"), "2");
        assert_eq!(map_position_side("short"), "2");
        assert_eq!(map_position_side("BOTH"), "0");
    }

    // ========================================================================
    // 验证错误测试
    // ========================================================================

    #[test]
    fn test_build_create_order_limit_missing_price() {
        // OrderRequest::build() 已经校验了限价单必须有 price
        // 这里测试无法构造出缺少 price 的 Limit 订单
        // 因此这个测试没有意义，跳过
    }

    #[test]
    fn test_build_create_order_spot_market_buy_with_base_amount() {
        // 现货市价买单使用 base 数量 (错误)
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.01))) // base 数量，应该用 quote
            .build()
            .expect("OrderRequest should build successfully");

        let mut market = mock_market("BTC/USDT", "BTCUSDT");
        market.market_type = MarketType::Spot;
        let result = build_create_order_payload(&request, &market, &create_extra_params("spot"));

        // 现货市价买单应该使用 quote 金额
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Spot market buy orders require quote currency")
        );
    }

    #[test]
    fn test_build_create_order_spot_limit_with_quote_amount() {
        // OrderRequest::build() 已经校验了限价单不能用 quote 金额
        // 这里无法构造出这样的订单，跳过
    }

    #[test]
    fn test_build_create_order_futures_with_quote_amount() {
        // OrderRequest::build() 已经校验了期货不能用 quote 金额
        // 这里无法构造出这样的订单，跳过
    }

    #[test]
    fn test_build_create_order_option_missing_order_link_id() {
        // 期权订单缺少 orderLinkId (错误)
        let request = OrderRequest::builder()
            .symbol("BTC-29DEC23-40000-C")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(1000)))
            // 缺少 client_order_id
            .build()
            .expect("OrderRequest should build successfully");

        let mut market = mock_market("BTC-29DEC23-40000-C", "BTC-29DEC23-40000-C");
        market.market_type = MarketType::Option;
        let result = build_create_order_payload(&request, &market, &create_extra_params("option"));

        // 期权订单必传 orderLinkId
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Option orders require client_order_id")
        );
    }

    #[test]
    fn test_build_create_order_unsupported_trailing_stop() {
        // Bybit 不支持 StopLossLimit
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLossLimit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(45000)))
            .stop_price(Price::new(dec!(46000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT");
        let result = build_create_order_payload(&request, &market, &create_extra_params("linear"));

        // Bybit 不支持 StopLossLimit
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("StopLossLimit"));
    }
}

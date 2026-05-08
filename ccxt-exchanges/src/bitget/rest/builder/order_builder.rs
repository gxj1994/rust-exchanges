//! Bitget 订单构建辅助函数
//!
//! 这些函数负责将统一的 OrderRequest 转换为 Bitget 特定的 JSON 格式。
//! 函数设计为纯函数(无外部依赖),便于独立测试。
//!
//! # 设计原则
//!
//! - 纯函数: 不依赖交易所实例,仅使用传入参数
//! - 可测试: 每个函数都可独立单元测试
//! - 清晰命名: 函数名明确表示其用途
//! - 完整注释: 说明 Bitget API 特殊要求

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
/// # Bitget API 要求
///
/// - `symbol`: 交易对(交易所特定格式,如 "BTCUSDT")
/// - `side`: 买卖方向("buy" / "sell")
/// - `orderType`: 订单类型("limit" / "market" / "limit_maker")
/// - `size`: 订单数量
/// - `force`: 时间生效模式("gtc" / "ioc" / "fok" / "post_only")
/// - `price`: 限价单必填
///
/// # 可选字段
///
/// - `clientOid`: 客户端订单 ID
/// - `reduceOnly`: 仅减仓(合约)
/// - `triggerPrice`: 触发价格(计划单)
/// - `presetTakeProfitPrice`: 预设止盈价格
/// - `presetStopLossPrice`: 预设止损价格
///
/// # 注意事项
///
/// - Bitget 不支持在创建订单时设置追踪止损
/// - Bitget 的 place-order API 只返回极简响应({orderId, clientOid})
///   需要额外调用 fetch_order 获取完整订单信息
///
/// # 参数
///
/// - `request`: 统一订单请求
/// - `market`: 市场信息
/// - `contract_params`: 合约订单额外参数（可选）
///   - `category`: 产品类型 (USDT-FUTURES/COIN-FUTURES/SPOT)
///   - `hold_mode`: 持仓模式 (one_way_mode/hedge_mode)
///
/// # 错误
///
/// - 如果订单类型不被 Bitget 支持,返回 `Error::InvalidOrder`
/// - 如果参数组合冲突,返回 `Error::InvalidOrder`
pub fn build_create_order_payload(
    request: &OrderRequest,
    market: &Market,
    contract_params: Option<&std::collections::HashMap<String, String>>,
) -> Result<Value, Error> {
    // ✅ 验证 1: Bitget 不支持的订单类型
    if !is_bitget_supported_order_type(request.order_type) {
        return Err(Error::InvalidOrder(
            format!("Bitget does not support {:?} orders", request.order_type).into(),
        ));
    }

    // ✅ 验证 2: Bitget 不支持追踪止损
    if request.trailing_callback_rate.is_some() {
        return Err(Error::InvalidOrder(
            "Bitget REST API does not support trailing stop orders. \
             Consider managing trailing stop client-side."
                .into(),
        ));
    }

    let mut map = Map::new();

    // 基础必填字段
    map.insert("symbol".to_string(), Value::String(market.id.clone()));
    map.insert("side".to_string(), Value::String(map_side(request.side)));
    map.insert(
        "orderType".to_string(),
        Value::String(map_order_type(request.order_type)),
    );

    // 根据市场类型和订单类型智能选择数量参数
    let size_params = build_size_parameter(request, &market.market_type);

    // 插入所有数量参数
    for (key, value) in size_params {
        map.insert(key, Value::String(value));
    }

    // TimeInForce / force 字段
    // LimitMaker 订单类型自动设置为 post_only
    // 注意：合约订单不需要 force 参数
    let force = if request.order_type == OrderType::LimitMaker {
        "post_only".to_string()
    } else {
        map_time_in_force(request.time_in_force, request.post_only)
    };

    // 只为现货市场添加 force 参数
    if matches!(market.market_type, MarketType::Spot) {
        map.insert("force".to_string(), Value::String(force));
    }

    // 价格(限价单必需)
    if let Some(price) = request.price {
        if matches!(request.order_type, OrderType::Limit | OrderType::LimitMaker) {
            map.insert("price".to_string(), Value::String(price.to_string()));
        }
    }

    // 客户端订单 ID
    if let Some(client_id) = &request.client_order_id {
        map.insert("clientOid".to_string(), Value::String(client_id.clone()));
    }

    // 仅减仓(合约)
    // V3 接口：reduceOnly 使用字符串 "yes"/"no"
    if let Some(reduce_only) = request.reduce_only {
        let reduce_only_str = if reduce_only { "yes" } else { "no" };
        map.insert(
            "reduceOnly".to_string(),
            Value::String(reduce_only_str.to_string()),
        );
    }

    // 合约订单必填参数校验和组装
    if matches!(market.market_type, MarketType::Swap | MarketType::Futures) {
        let params = contract_params.ok_or_else(|| {
            Error::InvalidOrder(
                "Contract orders require contract_params with category and hold_mode".into(),
            )
        })?;

        // V3 接口验证必填参数
        let category = params.get("category").ok_or_else(|| {
            Error::InvalidOrder("Contract orders require category in contract_params".into())
        })?;
        let hold_mode = params.get("hold_mode").ok_or_else(|| {
            Error::InvalidOrder("Contract orders require hold_mode in contract_params".into())
        })?;

        // V3 接口：添加 category 参数
        map.insert("category".to_string(), Value::String(category.clone()));

        // V3 接口：根据持仓模式决定是否传 posSide
        // 双向持仓：posSide 必填 (long/short)
        // 单向持仓：不要传 posSide，平仓用 reduceOnly
        if hold_mode.as_str() == "hedge_mode" {
            // 双向模式：根据 reduceOnly 决定 posSide
            let pos_side = if request.reduce_only == Some(true) {
                // 平仓：根据 side 决定
                if request.side == OrderSide::Buy {
                    "short" // 平空
                } else {
                    "long" // 平多
                }
            } else {
                // 开仓：根据 side 决定
                if request.side == OrderSide::Buy {
                    "long" // 开多
                } else {
                    "short" // 开空
                }
            };
            map.insert("posSide".to_string(), Value::String(pos_side.to_string()));
        } else {
            // 单向模式：不传 posSide
            // reduceOnly 已经在上面处理
        }
    } else if let Some(params) = contract_params {
        // 现货订单：添加 category 参数
        if let Some(category) = params.get("category") {
            map.insert("category".to_string(), Value::String(category.clone()));
        }
    }

    // 触发价格(计划单/条件单)
    if let Some(trigger) = request.stop_price {
        map.insert(
            "triggerPrice".to_string(),
            Value::String(trigger.to_string()),
        );
    }

    // ========================================================================
    // Bitget V3 API: 预设止盈止损 (Preset TP/SL)
    // ========================================================================
    // V3 API 字段名（与 V2 不同）：
    // - takeProfit: 止盈触发价格
    // - stopLoss: 止损触发价格
    // - tpTriggerBy/slTriggerBy: 触发价格类型 (market/mark)
    // - tpOrderType/slOrderType: 触发后的订单类型 (limit/market)
    // - tpLimitPrice/slLimitPrice: 限价单执行价格

    // 预设止盈触发价格
    if let Some(tp_trigger_price) = request.tp_limit_price {
        map.insert(
            "takeProfit".to_string(),
            Value::String(tp_trigger_price.to_string()),
        );
    }

    // 预设止损触发价格
    if let Some(sl_trigger_price) = request.sl_limit_price {
        map.insert(
            "stopLoss".to_string(),
            Value::String(sl_trigger_price.to_string()),
        );
    }

    // 止盈触发价格类型 (market 或 mark)
    if let Some(tp_trigger_by) = &request.trigger_price_type {
        // TriggerPriceType 枚举映射到 Bitget V3 API 值
        let tp_trigger_str = match tp_trigger_by {
            TriggerPriceType::MarkPrice => "mark",
            TriggerPriceType::LastPrice => "market",
            TriggerPriceType::IndexPrice => "market", // Bitget 不支持 index，使用 market
        };
        map.insert(
            "tpTriggerBy".to_string(),
            Value::String(tp_trigger_str.to_string()),
        );
    }

    // 止损触发价格类型 (market 或 mark)
    // 注意：trigger_price_type 同时控制 TP 和 SL，如果需要分别控制，需要使用 extra 字段
    if request.tp_limit_price.is_some() || request.sl_limit_price.is_some() {
        if let Some(trigger_type) = &request.trigger_price_type {
            let sl_trigger_str = match trigger_type {
                TriggerPriceType::MarkPrice => "mark",
                TriggerPriceType::LastPrice => "market",
                TriggerPriceType::IndexPrice => "market",
            };
            map.insert(
                "slTriggerBy".to_string(),
                Value::String(sl_trigger_str.to_string()),
            );
        }
    }

    // 止盈触发后的订单类型 (limit 或 market)
    // Bitget V3 API 规则：
    // - 如果 tpOrderType=limit，必须提供 tpLimitPrice（执行价格）
    // - 如果 tpOrderType=market，不需要 tpLimitPrice
    // 默认使用 market（更简单，不需要额外的执行价格）
    if request.tp_limit_price.is_some() {
        // 如果用户明确设置了 tp_limit_price，默认使用 market 触发
        // 这样不需要额外的执行价格
        map.insert(
            "tpOrderType".to_string(),
            Value::String("market".to_string()),
        );
    }

    // 止损触发后的订单类型 (limit 或 market)
    // 默认使用 market
    if request.sl_limit_price.is_some() {
        map.insert(
            "slOrderType".to_string(),
            Value::String("market".to_string()),
        );
    }

    Ok(Value::Object(map))
}

// ============================================================================
// 取消订单
// ============================================================================

/// 构建取消订单的请求体
///
/// # Bitget API 要求
///
/// - `symbol`: 交易对
/// - `orderId`: 订单 ID
pub fn build_cancel_order_payload(order_id: &str, market: &Market) -> Value {
    let mut map = Map::new();

    map.insert("symbol".to_string(), Value::String(market.id.clone()));
    map.insert("orderId".to_string(), Value::String(order_id.to_string()));

    Value::Object(map)
}

// ============================================================================
// 修改订单
// ============================================================================

/// 构建修改订单的请求体
///
/// # Bitget API 要求
///
/// - `symbol`: 交易对
/// - `orderId`: 订单 ID
/// - `newPrice`: 新价格(Bitget 仅支持修改价格)
///
/// # 限制
///
/// - Bitget 仅支持修改价格,不支持修改数量
/// - 如果 price 为 None,应在使用前返回错误
pub fn build_edit_order_payload(order_id: &str, market: &Market, price: &str) -> Value {
    let mut map = Map::new();

    map.insert("symbol".to_string(), Value::String(market.id.clone()));
    map.insert("orderId".to_string(), Value::String(order_id.to_string()));
    map.insert("newPrice".to_string(), Value::String(price.to_string()));

    Value::Object(map)
}

// ============================================================================
// 内部映射函数(私有)
// ============================================================================

/// 检查订单类型是否被 Bitget 支持
///
/// # Bitget 支持的订单类型
///
/// - `Market`: 市价单
/// - `Limit`: 限价单
/// - `LimitMaker`: 只做 Maker 单
/// - `StopLoss`: 止损单 (通过 triggerPrice 实现)
/// - `TakeProfit`: 止盈单 (通过 triggerPrice 实现)
///
/// # 不支持的订单类型
///
/// - `TrailingStop`: 追踪止损 (Bitget REST API 不支持)
/// - `StopLossLimit`, `TakeProfitLimit`: 限价止损/止盈 (Bitget 不支持)
fn is_bitget_supported_order_type(order_type: OrderType) -> bool {
    matches!(
        order_type,
        OrderType::Market
            | OrderType::Limit
            | OrderType::LimitMaker
            | OrderType::StopLoss
            | OrderType::TakeProfit
    )
}

/// 构建数量参数
///
/// # Bitget API 规则
///
/// ## 现货 (Spot)
/// - 限价单/市价卖单: `size` = 基础货币数量 (如 BTC)
/// - 市价买单: `size` = 报价货币金额 (如 USDT)
///
/// ## 合约 (Swap/Futures)
/// - 所有订单: `size` = 合约张数或基础货币数量
///
/// # 返回值
///
/// 返回 Vec<(参数名, 参数值)>
fn build_size_parameter(request: &OrderRequest, market_type: &MarketType) -> Vec<(String, String)> {
    match market_type {
        MarketType::Spot => {
            // V3 接口：现货使用 qty
            // 根据官方文档：
            // - 市价买单: qty = quote 金额 (USDT)
            // - 限价卖单/市价卖单: qty = base 数量 (BTC)
            if request.order_type == OrderType::Market && request.side == OrderSide::Buy {
                // 现货市价买单：必须使用 Quote 金额
                match &request.amount {
                    AmountSpec::Quote(quote_amt) => {
                        vec![("qty".to_string(), quote_amt.as_decimal().to_string())]
                    }
                    AmountSpec::Base(_) => {
                        // 错误：现货市价买单必须使用 quote 金额
                        // 这里不应该发生，因为 OrderRequest::build() 应该已经校验了
                        vec![("qty".to_string(), request.amount.as_decimal().to_string())]
                    }
                }
            } else {
                // 现货限价单/市价卖单：必须使用 Base 数量
                match &request.amount {
                    AmountSpec::Base(base_amt) => {
                        vec![("qty".to_string(), base_amt.as_decimal().to_string())]
                    }
                    AmountSpec::Quote(_) => {
                        // 错误：现货限价/卖单必须使用 base 数量
                        // 这里不应该发生，因为 OrderRequest::build() 应该已经校验了
                        vec![("qty".to_string(), request.amount.as_decimal().to_string())]
                    }
                }
            }
        }
        MarketType::Swap | MarketType::Futures | MarketType::Option => {
            // V3 接口：合约使用 qty
            // 根据官方文档：
            // - USDT/USDC 合约: qty = base 数量 (BTC)
            // - 币本位合约: qty = quote 金额 (USD)
            // 注意：当前实现假设是 USDT/USDC 合约，使用 base 数量
            // TODO: 如果是币本位合约，需要使用 quote 金额
            match &request.amount {
                ccxt_core::types::AmountSpec::Base(base_amt) => {
                    vec![("qty".to_string(), base_amt.as_decimal().to_string())]
                }
                ccxt_core::types::AmountSpec::Quote(_) => {
                    // 警告：USDT/USDC 合约应该使用 base 数量，但这里收到了 quote 金额
                    // 这里不应该发生，因为 OrderRequest::build() 应该已经校验了
                    vec![("qty".to_string(), request.amount.as_decimal().to_string())]
                }
            }
        }
    }
}

/// 映射订单方向
fn map_side(side: OrderSide) -> String {
    match side {
        OrderSide::Buy => "buy".to_string(),
        OrderSide::Sell => "sell".to_string(),
    }
}

/// 映射订单类型
///
/// # Bitget 订单类型映射
///
/// - `LimitMaker` → "limit_maker"
/// - `Market` / `StopLoss` / `StopMarket` / `TakeProfit` / `TrailingStop` → "market"
/// - 其他(包括 `Limit`) → "limit"
fn map_order_type(order_type: OrderType) -> String {
    match order_type {
        OrderType::LimitMaker => "limit_maker".to_string(),
        OrderType::Market
        | OrderType::StopLoss
        | OrderType::StopMarket
        | OrderType::TakeProfit
        | OrderType::TrailingStop => "market".to_string(),
        _ => "limit".to_string(),
    }
}

/// 映射 TimeInForce 到 Bitget 的 force 字段
///
/// # Bitget force 字段说明
///
/// - `gtc`: Good Till Cancelled(默认)
/// - `ioc`: Immediate Or Cancel
/// - `fok`: Fill Or Kill
/// - `post_only`: 仅做 Maker
///
/// # 优先级
///
/// 1. 如果 `time_in_force` 存在,使用其映射
/// 2. 否则如果 `post_only == Some(true)`,使用 "post_only"
/// 3. 否则默认 "gtc"
fn map_time_in_force(time_in_force: Option<TimeInForce>, post_only: Option<bool>) -> String {
    if let Some(tif) = time_in_force {
        match tif {
            TimeInForce::GTC => "gtc".to_string(),
            TimeInForce::IOC => "ioc".to_string(),
            TimeInForce::FOK => "fok".to_string(),
            TimeInForce::PO => "post_only".to_string(),
            TimeInForce::GTD => "gtc".to_string(), // Bitget不支持GTD,降级为GTC
        }
    } else if post_only == Some(true) {
        "post_only".to_string()
    } else {
        "gtc".to_string()
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
    use ccxt_core::types::market::{Market, MarketType};
    use rust_decimal_macros::dec;

    /// 创建测试用的 Market 对象
    fn mock_market(unified_symbol: &str, exchange_id: &str, market_type: MarketType) -> Market {
        let mut market = Market::default();
        market.id = exchange_id.to_string();
        market.symbol = Symbol::new_unchecked(unified_symbol);
        market.market_type = market_type;
        market.base = "BTC".to_string();
        market.quote = "USDT".to_string();
        market
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
            .amount(Amount::new(dec!(100.0))) // 100 USDT
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["symbol"], "BTCUSDT");
        assert_eq!(payload["side"], "buy");
        assert_eq!(payload["orderType"], "market");
        // 现货市价买单: qty 就是 quote 金额 (USDT)
        assert_eq!(payload["qty"], "100.0");
        assert!(!payload.as_object().unwrap().contains_key("quoteOrderQty"));
        assert_eq!(payload["force"], "gtc");
        assert!(!payload.as_object().unwrap().contains_key("price"));
    }

    #[test]
    fn test_build_create_order_market_sell() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.001))) // 0.001 BTC
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["symbol"], "BTCUSDT");
        assert_eq!(payload["side"], "sell");
        assert_eq!(payload["orderType"], "market");
        // 市价卖单使用 qty
        assert_eq!(payload["qty"], "0.001");
        assert!(!payload.as_object().unwrap().contains_key("quoteOrderQty"));
    }

    #[test]
    fn test_build_create_order_limit_sell() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.5)))
            .price(Price::new(dec!(50000)))
            .time_in_force(TimeInForce::GTC)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["symbol"], "BTCUSDT");
        assert_eq!(payload["side"], "sell");
        assert_eq!(payload["orderType"], "limit");
        assert_eq!(payload["qty"], "0.5");
        assert_eq!(payload["price"], "50000");
        assert_eq!(payload["force"], "gtc");
    }

    #[test]
    fn test_build_create_order_limit_maker() {
        let request = OrderRequest::builder()
            .symbol("ETH/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::LimitMaker)
            .amount(Amount::new(dec!(1.0)))
            .price(Price::new(dec!(3000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("ETH/USDT", "ETHUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["orderType"], "limit_maker");
        assert_eq!(payload["force"], "post_only");
        assert_eq!(payload["price"], "3000");
    }

    #[test]
    fn test_build_create_order_with_client_oid() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .client_order_id("my-order-123")
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["clientOid"], "my-order-123");
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

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT", MarketType::Swap);

        // 合约订单需要 contract_params
        let mut contract_params = std::collections::HashMap::new();
        contract_params.insert("category".to_string(), "USDT-FUTURES".to_string());
        contract_params.insert("hold_mode".to_string(), "one_way_mode".to_string());

        let payload = build_create_order_payload(&request, &market, Some(&contract_params))
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["reduceOnly"], "yes");
        // 合约统一使用 qty,即使是市价买单
        assert_eq!(payload["qty"], "0.1");
        assert!(!payload.as_object().unwrap().contains_key("quoteOrderQty"));
    }

    #[test]
    fn test_build_create_order_contract_market_buy() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.01))) // 合约张数
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTCUSDT", MarketType::Swap);

        // 合约订单需要 contract_params
        let mut contract_params = std::collections::HashMap::new();
        contract_params.insert("category".to_string(), "USDT-FUTURES".to_string());
        contract_params.insert("hold_mode".to_string(), "one_way_mode".to_string());

        let payload = build_create_order_payload(&request, &market, Some(&contract_params))
            .expect("build_create_order_payload should succeed");

        // 合约市价买单也使用 qty (合约张数)
        assert_eq!(payload["qty"], "0.01");
        assert!(!payload.as_object().unwrap().contains_key("quoteOrderQty"));
    }

    #[test]
    fn test_build_create_order_with_trigger_price() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLoss)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(45000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["orderType"], "market"); // StopLoss 映射为 market
        assert_eq!(payload["triggerPrice"], "45000");
    }

    #[test]
    fn test_build_create_order_with_take_profit() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::TakeProfit)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(55000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["orderType"], "market"); // TakeProfit 映射为 market
        assert_eq!(payload["triggerPrice"], "55000");
    }

    #[test]
    fn test_build_create_order_with_stop_loss() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::StopLoss)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(Price::new(dec!(48000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["orderType"], "market"); // StopLoss 映射为 market
        assert_eq!(payload["triggerPrice"], "48000");
    }

    #[test]
    fn test_build_create_order_time_in_force_ioc() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .time_in_force(TimeInForce::IOC)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["force"], "ioc");
    }

    #[test]
    fn test_build_create_order_time_in_force_fok() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .time_in_force(TimeInForce::FOK)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["force"], "fok");
    }

    #[test]
    fn test_build_create_order_post_only_via_flag() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000)))
            .post_only(true)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, None)
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["force"], "post_only");
    }

    // ========================================================================
    // build_cancel_order_payload 测试
    // ========================================================================

    #[test]
    fn test_build_cancel_order() {
        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_cancel_order_payload("123456789", &market);

        assert_eq!(payload["symbol"], "BTCUSDT");
        assert_eq!(payload["orderId"], "123456789");
    }

    // ========================================================================
    // build_edit_order_payload 测试
    // ========================================================================

    #[test]
    fn test_build_edit_order() {
        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let payload = build_edit_order_payload("123456789", &market, "51000");

        assert_eq!(payload["symbol"], "BTCUSDT");
        assert_eq!(payload["orderId"], "123456789");
        assert_eq!(payload["newPrice"], "51000");
    }

    // ========================================================================
    // 内部映射函数测试
    // ========================================================================

    #[test]
    fn test_map_side() {
        assert_eq!(map_side(OrderSide::Buy), "buy");
        assert_eq!(map_side(OrderSide::Sell), "sell");
    }

    #[test]
    fn test_map_order_type() {
        assert_eq!(map_order_type(OrderType::Limit), "limit");
        assert_eq!(map_order_type(OrderType::LimitMaker), "limit_maker");
        assert_eq!(map_order_type(OrderType::Market), "market");
        assert_eq!(map_order_type(OrderType::StopLoss), "market");
        assert_eq!(map_order_type(OrderType::TakeProfit), "market");
    }

    #[test]
    fn test_map_time_in_force() {
        // 使用 time_in_force 参数
        assert_eq!(map_time_in_force(Some(TimeInForce::GTC), None), "gtc");
        assert_eq!(map_time_in_force(Some(TimeInForce::IOC), None), "ioc");
        assert_eq!(map_time_in_force(Some(TimeInForce::FOK), None), "fok");
        assert_eq!(map_time_in_force(Some(TimeInForce::PO), None), "post_only");

        // 使用 post_only 标志
        assert_eq!(map_time_in_force(None, Some(true)), "post_only");

        // 默认值
        assert_eq!(map_time_in_force(None, None), "gtc");
    }

    // ========================================================================
    // 验证错误测试 (新增)
    // ========================================================================

    #[test]
    fn test_build_create_order_unsupported_trailing_stop() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::TrailingStop)
            .amount(Amount::new(dec!(0.1)))
            .trailing_callback_rate_percent(dec!(1.0))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let result = build_create_order_payload(&request, &market, None);

        // Bitget 不支持 TrailingStop
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("TrailingStop"));
    }

    #[test]
    fn test_build_create_order_unsupported_stop_limit() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLossLimit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(45000)))
            .stop_price(Price::new(dec!(46000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTCUSDT", MarketType::Spot);
        let result = build_create_order_payload(&request, &market, None);

        // Bitget 不支持 StopLossLimit
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("StopLossLimit"));
    }
}

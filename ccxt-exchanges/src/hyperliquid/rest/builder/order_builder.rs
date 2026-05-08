//! HyperLiquid 订单构建辅助函数
//!
//! 这些函数负责将统一的 OrderRequest 转换为 HyperLiquid 特定的 JSON 格式。
//! 函数设计为纯函数(无外部依赖),便于独立测试。
//!
//! # 设计原则
//!
//! - 纯函数: 不依赖交易所实例,仅使用传入参数
//! - 可测试: 每个函数都可独立单元测试
//! - 清晰命名: 函数名明确表示其用途
//! - 完整注释: 说明 HyperLiquid API 特殊要求
//!
//! # HyperLiquid Exchange API 特殊要求
//!
//! - **嵌套结构**: 订单使用 3-4 层嵌套 JSON,字段名为单字母缩写
//!   - `a`: asset (资产索引)
//!   - `b`: isBuy (是否买入)
//!   - `p`: price (价格)
//!   - `s`: size (数量)
//!   - `r`: reduceOnly (仅减仓)
//!   - `t`: type (订单类型,嵌套对象)
//!   - `c`: cloid (客户端订单 ID,可选)
//!
//! - **TimeInForce 映射**:
//!   - GTC → "Gtc" (Good Til Canceled)
//!   - IOC → "Ioc" (Immediate Or Cancel)
//!   - PostOnly → "Alo" (Add Liquidity Only)
//!
//! - **市价单实现**: HyperLiquid 没有原生市价单，使用 IOC 限价单模拟
//!   - 永续合约: 价格设为 "0" 可以工作
//!   - 现货市场: 价格设为 "0" 可能不被测试网支持，建议使用当前市场价格
//!
//! - **止盈止损**: 使用 `trigger` 字段,只能设置一个(TP 或 SL)
//!   - 同时设置 TP+SL 需要使用 `grouping: "normalTpsl"` 或 `"positionTpsl"`
//!
//! - **Asset Index**:
//!   - 永续合约: universe 索引 (0, 1, 2, ...)
//!   - 现货: 10000 + universe 索引 (10000, 10001, 10002, ...)
//!
//! # 不支持的功能
//!
//! - StopLimit / TakeProfitLimit (HyperLiquid 仅支持市价触发单)
//! - TrailingStop (需客户端实现)

use ccxt_core::Error;
use ccxt_core::types::market::Market;
use ccxt_core::types::{OrderRequest, OrderSide, OrderType, TimeInForce};
use serde_json::{Map, Value};
use std::str::FromStr;
use tracing::warn;

/// 格式化十进制数字字符串，移除尾随零
///
/// HyperLiquid API 要求数值字段不能包含尾随零：
/// - 123.0 → "123"
/// - 0.123450 → "0.12345"
/// - 100.00 → "100"
fn format_decimal(value: String) -> String {
    // 如果不是小数，直接返回
    if !value.contains('.') {
        return value;
    }

    // 移除尾随零
    let trimmed = value.trim_end_matches('0');
    // 如果最后是小数点，也移除
    trimmed.trim_end_matches('.').to_string()
}

/// 对齐价格到 HyperLiquid 要求的精度
///
/// 根据 Python SDK 的逻辑：
/// 1. 保留 5 位有效数字
/// 2. 根据 szDecimals 调整小数位数：
///    - 合约（perp）：6 - szDecimals
///    - 现货（spot）：8 - szDecimals
///
/// # 参数
///
/// - `price`: 原始价格
/// - `market`: 市场信息（包含 szDecimals）
///
/// # 返回
///
/// 对齐后的价格字符串
fn align_price_to_tick_size(price: rust_decimal::Decimal, market: &Market) -> String {
    use rust_decimal::Decimal;

    // 从 market info 获取 szDecimals
    let sz_decimals = market
        .info
        .get("szDecimals")
        .and_then(|v| v.as_u64())
        .unwrap_or(4); // 默认 4

    // 判断是否是现货（合约 asset < 10000，现货 asset >= 10000）
    // 简化处理：根据 settle 字段判断
    let is_spot = market.settle.is_none();

    // 计算价格应该保留的小数位数
    // 合约：6 - szDecimals，现货：8 - szDecimals
    let decimal_places = if is_spot {
        8u32.saturating_sub(sz_decimals as u32)
    } else {
        6u32.saturating_sub(sz_decimals as u32)
    };

    // 先保留 5 位有效数字
    let price_f64: f64 = price.to_string().parse().unwrap_or(0.0);

    // 计算有效数字位数
    let abs_price = price_f64.abs();
    let magnitude = if abs_price > 0.0 {
        abs_price.log10().floor() as i32
    } else {
        0
    };
    let decimal_places_sig = 4 - magnitude; // 5 位有效数字 = 4 位小数（相对于整数位）

    let with_sig_figs = if decimal_places_sig >= 0 {
        format!("{:.*}", decimal_places_sig as usize, price_f64)
    } else {
        // 对于大数，四舍五入到整数位
        format!("{:.0}", price_f64)
    };

    // 再根据 decimal_places 四舍五入
    let final_price: f64 = with_sig_figs.parse().unwrap_or(0.0);
    let multiplier = 10f64.powi(decimal_places as i32);
    let rounded = (final_price * multiplier).round() / multiplier;

    // 转回 Decimal
    let result = Decimal::from_str(&format!("{:.10}", rounded)).unwrap_or(price);

    format_decimal(result.to_string())
}

// ============================================================================
// 字段名常量 (避免硬编码单字母缩写)
// ============================================================================

const FIELD_ASSET: &str = "a";
const FIELD_IS_BUY: &str = "b";
const FIELD_PRICE: &str = "p";
const FIELD_SIZE: &str = "s";
const FIELD_REDUCE_ONLY: &str = "r";
const FIELD_TYPE: &str = "t";
const FIELD_CLOID: &str = "c";
const FIELD_TRIGGER: &str = "trigger";
const FIELD_TRIGGER_IS_MARKET: &str = "isMarket";
const FIELD_TRIGGER_PX: &str = "triggerPx";
const FIELD_TRIGGER_TPSL: &str = "tpsl";
const FIELD_LIMIT: &str = "limit";
const FIELD_TIF: &str = "tif";

// ============================================================================
// 内部辅助结构
// ============================================================================

/// 触发单配置
struct TriggerConfig {
    is_market: bool,
    trigger_px: String,
    tpsl: String, // "tp" or "sl"
}

// ============================================================================
// 创建订单
// ============================================================================

/// 构建创建订单的请求体
///
/// # HyperLiquid API 要求
///
/// - `type`: "order"
/// - `orders`: 订单数组(支持批量,但本实现只处理单个)
/// - `grouping`: "na" | "normalTpsl" | "positionTpsl"
///
/// # 订单对象字段
///
/// - `a`: asset index (从 market.id 解析)
/// - `b`: isBuy (true/false)
/// - `p`: price (市价单为 "0")
/// - `s`: size (订单数量)
/// - `r`: reduceOnly (true/false)
/// - `t`: type 对象 (嵌套: {"limit": {"tif": "Gtc"}} 或 {"trigger": {...}})
/// - `c`: cloid (可选,128 位 hex 字符串)
///
/// # 参数
///
/// - `request`: 统一订单请求
/// - `market`: 市场信息(包含 asset_index)
///
/// # 错误
///
/// - 如果订单类型不被 HyperLiquid 支持,返回 `Error::InvalidOrder`
/// - 如果参数组合冲突,返回 `Error::InvalidOrder`
pub fn build_create_order_payload(request: &OrderRequest, market: &Market) -> Result<Value, Error> {
    // ✅ 验证 1: HyperLiquid 不支持的订单类型
    if !is_hyperliquid_supported_order_type(request.order_type) {
        return Err(Error::InvalidOrder(
            format!(
                "HyperLiquid does not support {:?} orders. \
                 Supported: Market, Limit, LimitMaker, StopLoss, TakeProfit",
                request.order_type
            )
            .into(),
        ));
    }

    // ✅ 验证 2: HyperLiquid 不支持原生追踪止损 (需客户端实现)
    if request.trailing_callback_rate.is_some() {
        warn!(
            "HyperLiquid does not support trailing stop orders via REST API. \
             Consider managing trailing stop client-side."
        );
    }

    // ✅ 验证 3: 限价单必须提供价格
    if matches!(request.order_type, OrderType::Limit | OrderType::LimitMaker)
        && request.price.is_none()
    {
        return Err(Error::InvalidOrder(
            "Limit orders require price parameter".into(),
        ));
    }

    // 解析 asset_index (market.id 已包含正确值)
    let asset_index: u32 = market.id.parse().unwrap_or(0);

    // 确定 isBuy
    let is_buy = matches!(request.side, OrderSide::Buy);

    // 确定价格
    // 注意: HyperLiquid 要求所有订单必须有价格字段（包括市价单）
    // 市价单应该由调用者提供基于市场价的激进价格
    // 价格会自动对齐到 tick size
    let limit_px = request
        .price
        .map(|p| align_price_to_tick_size(p.into(), market))
        .ok_or_else(|| {
            Error::invalid_argument(
                "HyperLiquid requires price for all orders (including market orders). \
                 Please provide a price based on current market price with slippage.",
            )
        })?;

    let size = format_decimal(request.amount.to_string());

    // 确定 reduceOnly
    let reduce_only = request.reduce_only.unwrap_or(false);

    // 构建 trigger 配置 (止盈/止损)
    let trigger = build_trigger_config(request, market)?;

    // 构建订单对象
    let order_map = build_order_wire(
        asset_index,
        is_buy,
        &limit_px,
        &size,
        reduce_only,
        request,
        trigger.as_ref(),
    );

    // 确定 grouping 策略
    // HyperLiquid 支持在创建订单时同时设置 TP+SL
    // 但需要根据订单类型判断
    let grouping = if matches!(
        request.order_type,
        OrderType::TakeProfit | OrderType::StopLoss
    ) {
        // 单独的 TP 或 SL 订单
        "na"
    } else {
        // 普通订单，无 TP/SL
        "na"
    };

    // 构建外层 action 对象
    let mut action_map = Map::new();
    action_map.insert("type".to_string(), Value::String("order".to_string()));
    action_map.insert(
        "orders".to_string(),
        Value::Array(vec![Value::Object(order_map)]),
    );
    action_map.insert("grouping".to_string(), Value::String(grouping.to_string()));

    Ok(Value::Object(action_map))
}

/// 构建订单线 (内部订单对象)
fn build_order_wire(
    asset_index: u32,
    is_buy: bool,
    limit_px: &str,
    size: &str,
    reduce_only: bool,
    request: &OrderRequest,
    trigger: Option<&TriggerConfig>,
) -> Map<String, Value> {
    let mut order_map = Map::new();

    // 基础字段
    order_map.insert(FIELD_ASSET.to_string(), Value::Number(asset_index.into()));
    order_map.insert(FIELD_IS_BUY.to_string(), Value::Bool(is_buy));
    order_map.insert(FIELD_PRICE.to_string(), Value::String(limit_px.to_string()));
    order_map.insert(FIELD_SIZE.to_string(), Value::String(size.to_string()));
    order_map.insert(FIELD_REDUCE_ONLY.to_string(), Value::Bool(reduce_only));

    // 构建 type 字段 (嵌套结构)
    let type_wire = if trigger.is_some() {
        // 触发单
        build_trigger_type_wire(trigger.unwrap())
    } else {
        // 限价单 (包括市价单用 IOC 模拟)
        build_limit_type_wire(request)
    };
    order_map.insert(FIELD_TYPE.to_string(), type_wire);

    // 处理 Client Order ID (Cloid)
    if let Some(cloid) = &request.client_order_id {
        // HyperLiquid 要求 cloid 是 128 位 hex 字符串 (0x 开头,总共 34 字符)
        if cloid.starts_with("0x") && cloid.len() == 34 {
            order_map.insert(FIELD_CLOID.to_string(), Value::String(cloid.clone()));
        } else {
            warn!(
                "Invalid cloid format: {}. Expected 0x-prefixed 128-bit hex string (34 chars). \
                 Cloid will be ignored.",
                cloid
            );
        }
    }

    order_map
}

/// 构建限价单 type 字段
fn build_limit_type_wire(request: &OrderRequest) -> Value {
    let tif = map_time_in_force_to_hyperliquid(request);

    let mut limit_map = Map::new();
    limit_map.insert(FIELD_TIF.to_string(), Value::String(tif));

    let mut type_map = Map::new();
    type_map.insert(FIELD_LIMIT.to_string(), Value::Object(limit_map));

    Value::Object(type_map)
}

/// 构建触发单 type 字段
///
/// 注意：字段顺序必须与 Python SDK 一致：
/// triggerPx → isMarket → tpsl
/// 这对 EIP-712 签名至关重要！
fn build_trigger_type_wire(trigger: &TriggerConfig) -> Value {
    let mut trigger_map = Map::new();
    // 严格按照 Python SDK 的字段顺序
    trigger_map.insert(
        FIELD_TRIGGER_IS_MARKET.to_string(),
        Value::Bool(trigger.is_market),
    );
    trigger_map.insert(
        FIELD_TRIGGER_PX.to_string(),
        Value::String(trigger.trigger_px.clone()),
    );
    trigger_map.insert(
        FIELD_TRIGGER_TPSL.to_string(),
        Value::String(trigger.tpsl.clone()),
    );

    let mut type_map = Map::new();
    type_map.insert(FIELD_TRIGGER.to_string(), Value::Object(trigger_map));

    Value::Object(type_map)
}

/// 构建触发单配置
fn build_trigger_config(
    request: &OrderRequest,
    market: &Market,
) -> Result<Option<TriggerConfig>, Error> {
    // HyperLiquid 的 trigger 字段用于止盈止损
    // 当订单类型是 TakeProfit 或 StopLoss 时，使用 stop_price 作为触发价
    // 或者普通订单设置了 stop_price 也可以附加 trigger
    if request.order_type == OrderType::TakeProfit {
        if let Some(trigger_px) = request.stop_price {
            // 对齐触发价格到精度
            let aligned_trigger_px = align_price_to_tick_size(trigger_px.into(), market);
            Ok(Some(TriggerConfig {
                is_market: true, // HyperLiquid 触发单默认市价
                trigger_px: aligned_trigger_px,
                tpsl: "tp".to_string(),
            }))
        } else {
            return Err(Error::InvalidOrder(
                "TakeProfit orders require stop_price (trigger price)".into(),
            ));
        }
    } else if request.order_type == OrderType::StopLoss {
        if let Some(trigger_px) = request.stop_price {
            // 对齐触发价格到精度
            let aligned_trigger_px = align_price_to_tick_size(trigger_px.into(), market);
            Ok(Some(TriggerConfig {
                is_market: true,
                trigger_px: aligned_trigger_px,
                tpsl: "sl".to_string(),
            }))
        } else {
            return Err(Error::InvalidOrder(
                "StopLoss orders require stop_price (trigger price)".into(),
            ));
        }
    } else if let Some(trigger_px) = request.stop_price {
        // 普通订单也可以设置 stop_price 作为触发价
        // 根据订单方向判断是 TP 还是 SL
        let tpsl = if request.side == OrderSide::Sell {
            "tp" // 卖单设置触发价通常是止盈
        } else {
            "sl" // 买单设置触发价通常是止损
        };
        // 对齐触发价格到精度
        let aligned_trigger_px = align_price_to_tick_size(trigger_px.into(), market);
        Ok(Some(TriggerConfig {
            is_market: true,
            trigger_px: aligned_trigger_px,
            tpsl: tpsl.to_string(),
        }))
    } else {
        Ok(None)
    }
}

// ============================================================================
// 取消订单
// ============================================================================

/// 构建取消订单的请求体
///
/// # HyperLiquid API 要求
///
/// ```json
/// {
///   "type": "cancel",
///   "cancels": [{
///     "a": Number,  // asset index
///     "o": Number   // order id
///   }]
/// }
/// ```
///
/// # 参数
///
/// - `order_id`: 订单 ID (u64)
/// - `asset_index`: 资产索引
pub fn build_cancel_order_payload(order_id: u64, asset_index: u32) -> Value {
    let mut cancel_map = Map::new();
    cancel_map.insert(FIELD_ASSET.to_string(), Value::Number(asset_index.into()));
    cancel_map.insert("o".to_string(), Value::Number(order_id.into()));

    let mut action_map = Map::new();
    action_map.insert("type".to_string(), Value::String("cancel".to_string()));
    action_map.insert(
        "cancels".to_string(),
        Value::Array(vec![Value::Object(cancel_map)]),
    );

    Value::Object(action_map)
}

// ============================================================================
// 修改订单
// ============================================================================

/// 构建修改订单的请求体
///
/// # HyperLiquid API 要求
///
/// ```json
/// {
///   "type": "modify",
///   "oid": Number,
///   "order": { ... }  // 与创建订单相同的结构
/// }
/// ```
///
/// # 参数
///
/// - `request`: 新的订单参数
/// - `market`: 市场信息
/// - `order_id`: 要修改的订单 ID
pub fn build_modify_order_payload(
    request: &OrderRequest,
    market: &Market,
    order_id: u64,
) -> Result<Value, Error> {
    // 复用 build_create_order_payload 构建订单对象
    let order_payload = build_create_order_payload(request, market)?;

    // 提取订单对象 (从 orders 数组中)
    let order = order_payload["orders"][0].clone();

    let mut action_map = Map::new();
    action_map.insert("type".to_string(), Value::String("modify".to_string()));
    action_map.insert("oid".to_string(), Value::Number(order_id.into()));
    action_map.insert("order".to_string(), order);

    Ok(Value::Object(action_map))
}

// ============================================================================
// 内部映射函数 (私有)
// ============================================================================

/// 检查订单类型是否被 HyperLiquid 支持
fn is_hyperliquid_supported_order_type(order_type: OrderType) -> bool {
    matches!(
        order_type,
        OrderType::Market
            | OrderType::Limit
            | OrderType::LimitMaker
            | OrderType::StopLoss
            | OrderType::TakeProfit
    )
}

/// 映射 TimeInForce 到 HyperLiquid 格式
fn map_time_in_force_to_hyperliquid(request: &OrderRequest) -> String {
    // PostOnly 优先级最高
    if request.post_only == Some(true) || request.order_type == OrderType::LimitMaker {
        return "Alo".to_string(); // Add Liquidity Only
    }

    // 市价单使用 IOC
    if request.order_type == OrderType::Market {
        return "Ioc".to_string();
    }

    // 根据 TimeInForce 映射
    match request.time_in_force {
        Some(TimeInForce::IOC) => "Ioc".to_string(),
        Some(TimeInForce::FOK) => {
            // HyperLiquid 不支持 FOK,降级为 IOC
            warn!("HyperLiquid does not support FOK, downgrading to IOC");
            "Ioc".to_string()
        }
        Some(TimeInForce::PO) => "Alo".to_string(),
        _ => "Gtc".to_string(), // 默认 GTC (包括 GTC 和 None)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ccxt_core::types::financial::{Amount, Price};
    use rust_decimal_macros::dec;

    // ============================================================================
    // 辅助函数
    // ============================================================================

    fn create_mock_market(symbol: &str, asset_id: &str) -> Market {
        use ccxt_core::types::Symbol;
        let mut market = Market::default();
        market.id = asset_id.to_string();
        market.symbol = Symbol::new_unchecked(symbol);
        market.base = "BTC".to_string();
        market.quote = "USDC".to_string();
        market
    }

    // ============================================================================
    // 基础订单类型测试
    // ============================================================================

    #[test]
    fn test_build_create_order_market_buy() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(50000))) // HyperLiquid requires price even for market orders
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let action = build_create_order_payload(&request, &market).unwrap();

        // 验证外层结构
        assert_eq!(action["type"], "order");
        assert_eq!(action["grouping"], "na");

        // 验证订单数组
        let orders = action["orders"].as_array().unwrap();
        assert_eq!(orders.len(), 1);

        let order = &orders[0];
        assert_eq!(order["a"], 0); // asset_index
        assert_eq!(order["b"], true); // isBuy
        assert_eq!(order["p"], "50000"); // Market order with slippage price
        assert_eq!(order["s"], "0.1"); // size
        assert_eq!(order["r"], false); // reduceOnly

        // 验证 type 嵌套结构(市价单用 IOC)
        assert_eq!(order["t"]["limit"]["tif"], "Ioc");
    }

    #[test]
    fn test_build_create_order_market_sell() {
        let request = OrderRequest::builder()
            .symbol("ETH/USDC:USDC")
            .side(OrderSide::Sell)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(1.5)))
            .price(Price::new(dec!(3000))) // HyperLiquid requires price
            .build()
            .expect("Valid order request");

        let market = create_mock_market("ETH/USDC:USDC", "1");

        let action = build_create_order_payload(&request, &market).unwrap();
        let order = &action["orders"][0];

        assert_eq!(order["a"], 1);
        assert_eq!(order["b"], false); // Sell
        assert_eq!(order["p"], "3000"); // Market order with slippage price
        assert_eq!(order["s"], "1.5");
    }

    #[test]
    fn test_build_create_order_limit_buy() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.01)))
            .price(ccxt_core::types::financial::Price::new(dec!(50000)))
            .time_in_force(TimeInForce::GTC)
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let action = build_create_order_payload(&request, &market).unwrap();
        let order = &action["orders"][0];

        assert_eq!(order["p"], "50000");
        assert_eq!(order["t"]["limit"]["tif"], "Gtc");
    }

    #[test]
    fn test_build_create_order_limit_sell() {
        let request = OrderRequest::builder()
            .symbol("ETH/USDC:USDC")
            .side(OrderSide::Sell)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(2.0)))
            .price(ccxt_core::types::financial::Price::new(dec!(3000)))
            .build()
            .expect("Valid order request");

        let market = create_mock_market("ETH/USDC:USDC", "1");

        let action = build_create_order_payload(&request, &market).unwrap();
        let order = &action["orders"][0];

        assert_eq!(order["p"], "3000");
        assert_eq!(order["t"]["limit"]["tif"], "Gtc");
    }

    #[test]
    fn test_build_create_order_limit_maker() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::LimitMaker)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(45000)))
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let action = build_create_order_payload(&request, &market).unwrap();
        let order = &action["orders"][0];

        // LimitMaker 应使用 ALO (Add Liquidity Only)
        assert_eq!(order["t"]["limit"]["tif"], "Alo");
    }

    #[test]
    fn test_build_create_order_with_reduce_only() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Sell)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(60000)))
            .reduce_only(true)
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let action = build_create_order_payload(&request, &market).unwrap();
        let order = &action["orders"][0];

        assert_eq!(order["r"], true); // reduceOnly
    }

    // ============================================================================
    // 触发单测试
    // ============================================================================

    // ============================================================================
    // TimeInForce 测试
    // ============================================================================

    #[test]
    fn test_build_create_order_time_in_force_gtc() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(50000)))
            .time_in_force(TimeInForce::GTC)
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let action = build_create_order_payload(&request, &market).unwrap();
        let order = &action["orders"][0];

        assert_eq!(order["t"]["limit"]["tif"], "Gtc");
    }

    #[test]
    fn test_build_create_order_time_in_force_ioc() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(50000)))
            .time_in_force(TimeInForce::IOC)
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let action = build_create_order_payload(&request, &market).unwrap();
        let order = &action["orders"][0];

        assert_eq!(order["t"]["limit"]["tif"], "Ioc");
    }

    #[test]
    fn test_build_create_order_post_only_flag() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(50000)))
            .post_only(true)
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let action = build_create_order_payload(&request, &market).unwrap();
        let order = &action["orders"][0];

        // post_only 标志应转换为 ALO
        assert_eq!(order["t"]["limit"]["tif"], "Alo");
    }

    // ============================================================================
    // 取消和修改订单测试
    // ============================================================================

    #[test]
    fn test_build_cancel_order() {
        let action = build_cancel_order_payload(12345, 0);

        assert_eq!(action["type"], "cancel");
        assert_eq!(action["cancels"][0]["a"], 0);
        assert_eq!(action["cancels"][0]["o"], 12345);
    }

    #[test]
    fn test_build_modify_order() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(52000)))
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let action = build_modify_order_payload(&request, &market, 12345).unwrap();

        assert_eq!(action["type"], "modify");
        assert_eq!(action["oid"], 12345);
        assert_eq!(action["order"]["p"], "52000");
    }

    // ============================================================================
    // 边界和错误处理测试
    // ============================================================================

    #[test]
    fn test_build_create_order_stop_limit_unsupported() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::StopLimit)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(ccxt_core::types::financial::Price::new(dec!(45000)))
            .price(ccxt_core::types::financial::Price::new(dec!(44000)))
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let result = build_create_order_payload(&request, &market);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_create_order_take_profit_limit_unsupported() {
        // HyperLiquid 不支持 TakeProfitLimit (限价止盈单)
        // 只支持市价止盈单
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Sell)
            .order_type(OrderType::TakeProfitLimit)
            .amount(Amount::new(dec!(0.1)))
            .stop_price(ccxt_core::types::financial::Price::new(dec!(55000)))
            .price(ccxt_core::types::financial::Price::new(dec!(56000)))
            .build()
            .expect("Valid order request");

        let market = create_mock_market("BTC/USDC:USDC", "0");

        let result = build_create_order_payload(&request, &market);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_create_order_limit_missing_price() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .build(); // 不使用 .expect()，因为应该失败
        // build() 应该返回错误
        assert!(
            request.is_err(),
            "Limit order without price should fail validation"
        );
    }

    // ============================================================================
    // 内部函数测试
    // ============================================================================

    #[test]
    fn test_map_time_in_force_gtc() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(50000)))
            .time_in_force(TimeInForce::GTC)
            .build()
            .expect("Valid order request");

        let tif = map_time_in_force_to_hyperliquid(&request);
        assert_eq!(tif, "Gtc");
    }

    #[test]
    fn test_map_time_in_force_ioc() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(50000)))
            .time_in_force(TimeInForce::IOC)
            .build()
            .expect("Valid order request");

        let tif = map_time_in_force_to_hyperliquid(&request);
        assert_eq!(tif, "Ioc");
    }

    #[test]
    fn test_map_time_in_force_alo() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDC:USDC")
            .side(OrderSide::Buy)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.1)))
            .price(ccxt_core::types::financial::Price::new(dec!(50000)))
            .time_in_force(TimeInForce::PO)
            .build()
            .expect("Valid order request");

        let tif = map_time_in_force_to_hyperliquid(&request);
        assert_eq!(tif, "Alo");
    }

    #[test]
    fn test_is_hyperliquid_supported_order_type() {
        assert!(is_hyperliquid_supported_order_type(OrderType::Market));
        assert!(is_hyperliquid_supported_order_type(OrderType::Limit));
        assert!(is_hyperliquid_supported_order_type(OrderType::LimitMaker));
        assert!(is_hyperliquid_supported_order_type(OrderType::StopLoss));
        assert!(is_hyperliquid_supported_order_type(OrderType::TakeProfit));

        assert!(!is_hyperliquid_supported_order_type(OrderType::StopLimit));
        assert!(!is_hyperliquid_supported_order_type(
            OrderType::TakeProfitLimit
        ));
        assert!(!is_hyperliquid_supported_order_type(
            OrderType::TrailingStop
        ));
    }
}

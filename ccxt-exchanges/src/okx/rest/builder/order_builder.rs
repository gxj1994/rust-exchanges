//! OKX 订单构建辅助函数
//!
//! 这些函数负责将统一的 OrderRequest 转换为 OKX 特定的 JSON 格式。
//! 函数设计为纯函数(无外部依赖),便于独立测试。
//!
//! # 设计原则
//!
//! - 纯函数: 不依赖交易所实例,仅使用传入参数
//! - 可测试: 每个函数都可独立单元测试
//! - 清晰命名: 函数名明确表示其用途
//! - 完整注释: 说明 OKX API 特殊要求
//!
//! # OKX V5 API 特殊要求
//!
//! - `tdMode`: 交易模式必填 (cash/cross/isolated)
//! - `ordType`: 订单类型映射 (limit/market/post_only/fok/ioc)
//! - `sz`: 订单数量 (现货市价买单用quote金额,其他用base数量)
//! - `px`: 限价单必填
//! - `posSide`: 双向持仓必填 (long/short)
//! - 止盈止损: 支持 tpTriggerPx/slTriggerPx 和 tpOrdPx/slOrdPx
//! - 极简响应: 下单只返回 ordId,需额外 fetch_order 补全

use ccxt_core::Error;
use ccxt_core::types::market::{Market, MarketType};
use ccxt_core::types::{AmountSpec, OrderRequest, OrderSide, OrderType, TimeInForce};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Value};
use std::collections::HashMap;

// ============================================================================
// 创建订单
// ============================================================================

/// 构建创建订单的请求体
///
/// # OKX V5 API 要求
///
/// - `instId`: 产品ID (如 "BTC-USDT")
/// - `tdMode`: 交易模式 (cash/cross/isolated) - 必填
/// - `ordType`: 订单类型 (market/limit/post_only/fok/ioc)
/// - `sz`: 订单数量
/// - `px`: 限价单必填
///
/// # 可选字段
///
/// - `clOrdId`: 客户端订单 ID
/// - `posSide`: 持仓方向 (双向持仓必填)
/// - `reduceOnly`: 仅减仓
/// - `tpTriggerPx`: 止盈触发价格
/// - `slTriggerPx`: 止损触发价格
/// - `tpOrdPx`: 止盈限价
/// - `slOrdPx`: 止损限价
/// - `tpTriggerPxType`: 止盈触发类型 (last/mark/index)
/// - `slTriggerPxType`: 止损触发类型 (last/mark/index)
/// - `tag`: 订单标签 (最多16字符)
///
/// # 注意事项
///
/// - OKX 支持在创建订单时预设止盈止损
/// - 现货市价买单使用 quote 金额 (USDT)
/// - OKX 不支持原生 trailing stop,需客户端实现
///
/// # 参数
///
/// - `request`: 统一订单请求
/// - `market`: 市场信息
/// - `extra_params`: 额外参数 (td_mode, pos_side_mode 等)
///
/// # 错误
///
/// - 如果订单类型不被 OKX 支持,返回 `Error::InvalidOrder`
/// - 如果参数组合冲突,返回 `Error::InvalidOrder`
pub fn build_create_order_payload(
    request: &OrderRequest,
    market: &Market,
    extra_params: &HashMap<String, String>,
) -> Result<Value, Error> {
    // ✅ 验证 1: OKX 不支持的订单类型
    if !is_okx_supported_order_type(request.order_type) {
        return Err(Error::InvalidOrder(
            format!("OKX does not support {:?} orders", request.order_type).into(),
        ));
    }

    // ✅ 验证 2: OKX 不支持原生追踪止损 (需客户端实现)
    if request.trailing_callback_rate.is_some() {
        return Err(Error::InvalidOrder(
            "OKX REST API does not support trailing stop orders natively. \
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

    // 从 extra_params 获取 tdMode (必填)
    let td_mode = extra_params
        .get("td_mode")
        .ok_or_else(|| Error::InvalidOrder("Missing required parameter: td_mode".into()))?;

    let mut map = Map::new();

    // 基础必填字段
    map.insert("instId".to_string(), Value::String(market.id.clone()));
    map.insert("tdMode".to_string(), Value::String(td_mode.to_string()));
    map.insert("side".to_string(), Value::String(map_side(request.side)));
    map.insert(
        "ordType".to_string(),
        Value::String(map_order_type(request.order_type)),
    );

    // ========================================================================
    // 数量参数处理 (关键逻辑)
    // ========================================================================
    // OKX V5 规则:
    // - 现货市价买单: sz = quote 金额 (USDT)
    // - 其他情况: sz = base 数量 (BTC) 或合约张数
    // - 止盈止损单: 也使用sz,通过tpTriggerPx/slTriggerPx实现止盈止损
    let sz_value = match market.market_type {
        MarketType::Spot => {
            if request.order_type == OrderType::Market && request.side == OrderSide::Buy {
                // 现货市价买单: 必须使用 quote 金额
                match &request.amount {
                    AmountSpec::Quote(quote_amt) => quote_amt.as_decimal().to_string(),
                    AmountSpec::Base(_) => {
                        return Err(Error::InvalidOrder(
                            "Spot market buy orders require quote currency amount (USDT), not base currency".into(),
                        ));
                    }
                }
            } else {
                // 现货限价单/市价卖单: 使用 base 数量
                match &request.amount {
                    AmountSpec::Base(base_amt) => base_amt.as_decimal().to_string(),
                    AmountSpec::Quote(_) => {
                        return Err(Error::InvalidOrder(
                            "Spot limit/sell orders require base currency amount, not quote currency"
                                .into(),
                        ));
                    }
                }
            }
        }
        MarketType::Swap | MarketType::Futures | MarketType::Option => {
            // 合约/期权: OKX API要求 sz = 合约张数(不是币数!)
            // 需要将币数转换为张数: 张数 = 币数 / contractSize
            match &request.amount {
                AmountSpec::Base(base_amt) => {
                    // 获取合约面值
                    let contract_size = market.contract_size.unwrap_or(Decimal::ONE);
                    let base_decimal = base_amt.as_decimal();

                    // 币数 → 张数
                    let contracts = base_decimal / contract_size;

                    // 合约数量精度处理 (TRUNCATE)
                    let amount_precision = market.precision.amount.unwrap_or(Decimal::ONE);
                    let rounded_contracts = if amount_precision < Decimal::ONE {
                        // Tick size模式: TRUNCATE
                        let missing = contracts % amount_precision;
                        if contracts >= Decimal::ZERO {
                            contracts - missing
                        } else {
                            contracts + missing
                        }
                    } else {
                        // 小数位数模式
                        let digits = amount_precision.to_u32().unwrap_or(8);
                        let multiplier = Decimal::from(10_i64.pow(digits));
                        let scaled = contracts * multiplier;
                        scaled.floor() / multiplier
                    };

                    // 检查最小下单量
                    let min_contracts = market
                        .limits
                        .amount
                        .as_ref()
                        .and_then(|m| m.min)
                        .unwrap_or(Decimal::ONE);

                    if rounded_contracts < min_contracts {
                        return Err(Error::InvalidOrder(
                            format!(
                                "Order size {} contracts is less than minimum {} contracts for {}",
                                rounded_contracts, min_contracts, market.id
                            )
                            .into(),
                        ));
                    }

                    rounded_contracts.to_string()
                }
                AmountSpec::Quote(_) => {
                    return Err(Error::InvalidOrder(
                        "Futures/options orders require base currency amount, not quote currency"
                            .into(),
                    ));
                }
            }
        }
    };

    // 所有订单类型都使用sz(包括止盈止损)
    map.insert("sz".to_string(), Value::String(sz_value));

    // 价格(限价单必需)
    if let Some(price) = request.price {
        if matches!(request.order_type, OrderType::Limit | OrderType::LimitMaker) {
            map.insert("px".to_string(), Value::String(price.to_string()));
        }
    }

    // TimeInForce 处理
    // OKX 的 ordType 已经包含 post_only/fok/ioc,无需单独设置 timeInForce
    // 但如果 ordType 是 limit,且传入了 timeInForce,需要覆盖 ordType
    if request.order_type == OrderType::Limit {
        if let Some(tif) = request.time_in_force {
            match tif {
                TimeInForce::FOK => {
                    map.insert("ordType".to_string(), Value::String("fok".to_string()));
                }
                TimeInForce::IOC => {
                    map.insert("ordType".to_string(), Value::String("ioc".to_string()));
                }
                TimeInForce::PO => {
                    map.insert(
                        "ordType".to_string(),
                        Value::String("post_only".to_string()),
                    );
                }
                TimeInForce::GTC | TimeInForce::GTD => {
                    // GTC 是默认,无需修改
                }
            }
        } else if request.post_only == Some(true) {
            map.insert(
                "ordType".to_string(),
                Value::String("post_only".to_string()),
            );
        }
    }

    // 客户端订单 ID
    if let Some(client_id) = &request.client_order_id {
        map.insert("clOrdId".to_string(), Value::String(client_id.clone()));
    }

    // 持仓方向(合约双向持仓必填)
    // 注意: 止盈止损单也需要posSide
    if matches!(market.market_type, MarketType::Swap | MarketType::Futures) {
        if let Some(pos_side) = &request.position_side {
            // PositionSide 枚举转字符串: Long/Short/Both → long/short/both
            let pos_side_str = match pos_side {
                ccxt_core::types::order::PositionSide::Long => "long".to_string(),
                ccxt_core::types::order::PositionSide::Short => "short".to_string(),
                ccxt_core::types::order::PositionSide::Both => "both".to_string(),
            };
            map.insert("posSide".to_string(), Value::String(pos_side_str));
        }
    }

    // 仅减仓(合约)
    if let Some(reduce_only) = request.reduce_only {
        map.insert(
            "reduceOnly".to_string(),
            Value::String(reduce_only.to_string()),
        );
    }

    // ========================================================================
    // OKX V5 API: 附加止盈止损 (attachAlgoOrds)
    // ========================================================================
    // OKX 支持在普通订单(/trade/order)中附加止盈止损
    // 通过attachAlgoOrds数组传递止盈止损参数
    // 注意: attachAlgoOrds只适用于开仓订单,不适用于reduceOnly平仓单
    if request.reduce_only != Some(true) {
        let mut algo_ord = Map::new();

        // 处理止盈: tpTriggerPx或tpTriggerRatio(只能传一个)
        // 优先级: tpTriggerPx > extra.tpTriggerRatio
        if let Some(tp_trigger_price) = request.tp_limit_price {
            algo_ord.insert(
                "tpTriggerPx".to_string(),
                Value::String(tp_trigger_price.to_string()),
            );
            // OKX要求: 如果设置了tpTriggerPx,必须设置tpOrdPx
            // -1 表示市价单
            algo_ord.insert("tpOrdPx".to_string(), Value::String("-1".to_string()));
            algo_ord.insert(
                "tpTriggerPxType".to_string(),
                Value::String("last".to_string()),
            );
        } else if let Some(extra) = &request.extra {
            // 从extra中读取tpTriggerRatio(0-1之间的值)
            // order_builder根据订单方向自动添加±
            if let Some(tp_ratio) = extra.get("tpTriggerRatio") {
                // 买入订单: tpTriggerRatio > 0
                // 卖出订单: tpTriggerRatio < 0
                let ratio_str = tp_ratio.as_str().unwrap_or("0");
                let ratio: f64 = ratio_str.parse().unwrap_or(0.0);
                let final_ratio = if request.side == ccxt_core::types::OrderSide::Buy {
                    ratio // 买入: 正值
                } else {
                    -ratio // 卖出: 负值
                };
                algo_ord.insert(
                    "tpTriggerRatio".to_string(),
                    Value::String(format!("{:.2}", final_ratio)),
                );
                algo_ord.insert("tpOrdPx".to_string(), Value::String("-1".to_string()));
            }
        }

        // 处理止损: slTriggerPx或slTriggerRatio(只能传一个)
        // 优先级: slTriggerPx > extra.slTriggerRatio
        if let Some(sl_trigger_price) = request.stop_price {
            algo_ord.insert(
                "slTriggerPx".to_string(),
                Value::String(sl_trigger_price.to_string()),
            );
            // OKX要求: 如果设置了slTriggerPx,必须设置slOrdPx
            // -1 表示市价单
            algo_ord.insert("slOrdPx".to_string(), Value::String("-1".to_string()));
            algo_ord.insert(
                "slTriggerPxType".to_string(),
                Value::String("last".to_string()),
            );
        } else if let Some(extra) = &request.extra {
            // 从extra中读取slTriggerRatio(0-1之间的值)
            // order_builder根据订单方向自动添加±
            if let Some(sl_ratio) = extra.get("slTriggerRatio") {
                // 买入订单: slTriggerRatio < 0
                // 卖出订单: slTriggerRatio > 0
                let ratio_str = sl_ratio.as_str().unwrap_or("0");
                let ratio: f64 = ratio_str.parse().unwrap_or(0.0);
                let final_ratio = if request.side == ccxt_core::types::OrderSide::Buy {
                    -ratio // 买入: 负值
                } else {
                    ratio // 卖出: 正值
                };
                algo_ord.insert(
                    "slTriggerRatio".to_string(),
                    Value::String(format!("{:.2}", final_ratio)),
                );
                algo_ord.insert("slOrdPx".to_string(), Value::String("-1".to_string()));
            }
        }

        // 如果有止盈或止损,添加到请求体
        if !algo_ord.is_empty() {
            let algo_array = vec![Value::Object(algo_ord)];
            map.insert("attachAlgoOrds".to_string(), Value::Array(algo_array));
        }
    }

    // 订单标签(可选) - OKX暂时不支持tag字段,跳过
    // if let Some(tag) = &request.tag {
    //     // OKX 限制 tag 最多 16 字符
    //     let truncated_tag = if tag.len() > 16 {
    //         tag[..16].to_string()
    //     } else {
    //         tag.clone()
    //     };
    //     map.insert("tag".to_string(), Value::String(truncated_tag));
    // }

    Ok(Value::Object(map))
}

// ============================================================================
// 取消订单
// ============================================================================

/// 构建取消订单的请求体
///
/// # OKX API 要求
///
/// - `instId`: 产品ID
/// - `ordId`: 订单 ID (或 clOrdId)
pub fn build_cancel_order_payload(
    order_id: &str,
    market: &Market,
    client_order_id: Option<&str>,
) -> Value {
    let mut map = Map::new();

    map.insert("instId".to_string(), Value::String(market.id.clone()));

    // OKX 支持使用 ordId 或 clOrdId
    if let Some(client_oid) = client_order_id {
        map.insert("clOrdId".to_string(), Value::String(client_oid.to_string()));
    } else {
        map.insert("ordId".to_string(), Value::String(order_id.to_string()));
    }

    Value::Object(map)
}

// ============================================================================
// 修改订单
// ============================================================================

/// 构建修改订单的请求体
///
/// # OKX API 要求
///
/// - `instId`: 产品ID
/// - `ordId`: 订单 ID
/// - `newPx`: 新价格 (可选)
/// - `newSz`: 新数量 (可选)
///
/// # 限制
///
/// - OKX 支持修改价格和数量
/// - 至少提供一个修改字段
pub fn build_edit_order_payload(
    order_id: &str,
    market: &Market,
    price: Option<&str>,
    qty: Option<&str>,
    client_order_id: Option<&str>,
) -> Result<Value, Error> {
    if price.is_none() && qty.is_none() {
        return Err(Error::InvalidOrder(
            "At least one of price or qty must be provided for edit order".into(),
        ));
    }

    let mut map = Map::new();

    map.insert("instId".to_string(), Value::String(market.id.clone()));

    if let Some(client_oid) = client_order_id {
        map.insert("clOrdId".to_string(), Value::String(client_oid.to_string()));
    } else {
        map.insert("ordId".to_string(), Value::String(order_id.to_string()));
    }

    if let Some(p) = price {
        map.insert("newPx".to_string(), Value::String(p.to_string()));
    }

    if let Some(q) = qty {
        map.insert("newSz".to_string(), Value::String(q.to_string()));
    }

    Ok(Value::Object(map))
}

// ============================================================================
// 内部映射函数(私有)
// ============================================================================

/// 检查订单类型是否被 OKX 支持
///
/// # OKX 支持的订单类型
///
/// - `Market`: 市价单
/// - `Limit`: 限价单
/// - `LimitMaker`: 只做 Maker 单 (映射为 post_only)
/// - `StopLoss`: 止损单 (通过 triggerPx 实现)
/// - `StopMarket`: 止损市价单
/// - `TakeProfit`: 止盈单 (通过 triggerPx 实现)
///
/// # 不支持的订单类型
///
/// - `TrailingStop`: 追踪止损 (OKX REST API 不支持)
/// - `StopLossLimit`, `TakeProfitLimit`: 限价止损/止盈 (OKX 不支持)
fn is_okx_supported_order_type(order_type: OrderType) -> bool {
    matches!(
        order_type,
        OrderType::Market
            | OrderType::Limit
            | OrderType::LimitMaker
            | OrderType::StopLoss
            | OrderType::StopMarket
            | OrderType::TakeProfit
    )
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
/// # OKX 订单类型映射
///
/// - `LimitMaker` → "post_only"
/// - `Market` / `StopLoss` / `StopMarket` / `TakeProfit` → "market" (止盈止损通过attachAlgoOrds实现)
/// - 其他(包括 `Limit`) → "limit"
fn map_order_type(order_type: OrderType) -> String {
    match order_type {
        OrderType::LimitMaker => "post_only".to_string(),
        OrderType::Market | OrderType::StopLoss | OrderType::StopMarket | OrderType::TakeProfit => {
            "market".to_string()
        }
        _ => "limit".to_string(),
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

    /// 创建测试用的 extra_params (现货)
    fn create_extra_params_spot() -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("td_mode".to_string(), "cash".to_string());
        params
    }

    /// 创建测试用的 extra_params (合约)
    fn create_extra_params_swap() -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("td_mode".to_string(), "cross".to_string());
        params
    }

    // ========================================================================
    // build_create_order_payload 测试
    // ========================================================================

    #[test]
    fn test_build_create_order_spot_market_buy() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(AmountSpec::quote(Amount::new(dec!(100.0)))) // 100 USDT
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, &create_extra_params_spot())
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["instId"], "BTC-USDT");
        assert_eq!(payload["tdMode"], "cash");
        assert_eq!(payload["side"], "buy");
        assert_eq!(payload["ordType"], "market");
        assert_eq!(payload["sz"], "100.0");
        assert!(!payload.as_object().unwrap().contains_key("px"));
    }

    #[test]
    fn test_build_create_order_spot_market_sell() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.001))) // 0.001 BTC
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, &create_extra_params_spot())
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["instId"], "BTC-USDT");
        assert_eq!(payload["side"], "sell");
        assert_eq!(payload["ordType"], "market");
        assert_eq!(payload["sz"], "0.001");
    }

    #[test]
    fn test_build_create_order_spot_limit() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::Limit)
            .amount(Amount::new(dec!(0.5)))
            .price(Price::new(dec!(50000)))
            .time_in_force(TimeInForce::GTC)
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, &create_extra_params_spot())
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["instId"], "BTC-USDT");
        assert_eq!(payload["ordType"], "limit");
        assert_eq!(payload["px"], "50000");
        assert_eq!(payload["sz"], "0.5");
    }

    #[test]
    fn test_build_create_order_spot_limit_maker() {
        let request = OrderRequest::builder()
            .symbol("ETH/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::LimitMaker)
            .amount(Amount::new(dec!(1.0)))
            .price(Price::new(dec!(3000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("ETH/USDT", "ETH-USDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, &create_extra_params_spot())
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["ordType"], "post_only");
        assert_eq!(payload["px"], "3000");
    }

    #[test]
    fn test_build_create_order_with_client_order_id() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(AmountSpec::quote(Amount::new(dec!(100.0))))
            .client_order_id("my-order-123")
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, &create_extra_params_spot())
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["clOrdId"], "my-order-123");
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

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, &create_extra_params_spot())
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["ordType"], "ioc");
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

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, &create_extra_params_spot())
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["ordType"], "fok");
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

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_create_order_payload(&request, &market, &create_extra_params_spot())
            .expect("build_create_order_payload should succeed");

        assert_eq!(payload["ordType"], "post_only");
    }

    // ========================================================================
    // 验证错误测试
    // ========================================================================

    #[test]
    fn test_build_create_order_spot_market_buy_with_base_amount_error() {
        // 现货市价买单使用 base 数量 (错误)
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(Amount::new(dec!(0.01))) // base 数量,应该用 quote
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let result = build_create_order_payload(&request, &market, &create_extra_params_spot());

        // 现货市价买单应该使用 quote 金额
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Spot market buy orders require quote currency")
        );
    }

    #[test]
    fn test_build_create_order_unsupported_stop_loss_limit() {
        // OKX 不支持 StopLossLimit
        let request = OrderRequest::builder()
            .symbol("BTC/USDT:USDT")
            .side(OrderSide::Sell)
            .order_type(OrderType::StopLossLimit)
            .amount(Amount::new(dec!(0.1)))
            .price(Price::new(dec!(45000)))
            .stop_price(Price::new(dec!(46000)))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT:USDT", "BTC-USDT-SWAP", MarketType::Swap);
        let result = build_create_order_payload(&request, &market, &create_extra_params_swap());

        // OKX 不支持 StopLossLimit
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("StopLossLimit"));
    }

    #[test]
    fn test_build_create_order_missing_td_mode_error() {
        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(AmountSpec::quote(Amount::new(dec!(100.0))))
            .build()
            .expect("OrderRequest should build successfully");

        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let empty_params = HashMap::new();
        let result = build_create_order_payload(&request, &market, &empty_params);

        // 缺少 td_mode 应该报错
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("td_mode"));
    }

    // ========================================================================
    // build_cancel_order_payload 测试
    // ========================================================================

    #[test]
    fn test_build_cancel_order_with_order_id() {
        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_cancel_order_payload("123456789", &market, None);

        assert_eq!(payload["instId"], "BTC-USDT");
        assert_eq!(payload["ordId"], "123456789");
        assert!(!payload.as_object().unwrap().contains_key("clOrdId"));
    }

    #[test]
    fn test_build_cancel_order_with_client_order_id() {
        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_cancel_order_payload("123456789", &market, Some("my-order-123"));

        assert_eq!(payload["clOrdId"], "my-order-123");
        assert!(!payload.as_object().unwrap().contains_key("ordId"));
    }

    // ========================================================================
    // build_edit_order_payload 测试
    // ========================================================================

    #[test]
    fn test_build_edit_order_price_only() {
        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_edit_order_payload("123456789", &market, Some("51000"), None, None)
            .expect("build_edit_order_payload should succeed");

        assert_eq!(payload["ordId"], "123456789");
        assert_eq!(payload["newPx"], "51000");
        assert!(!payload.as_object().unwrap().contains_key("newSz"));
    }

    #[test]
    fn test_build_edit_order_qty_only() {
        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload = build_edit_order_payload("123456789", &market, None, Some("0.2"), None)
            .expect("build_edit_order_payload should succeed");

        assert_eq!(payload["newSz"], "0.2");
        assert!(!payload.as_object().unwrap().contains_key("newPx"));
    }

    #[test]
    fn test_build_edit_order_both_price_and_qty() {
        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let payload =
            build_edit_order_payload("123456789", &market, Some("51000"), Some("0.2"), None)
                .expect("build_edit_order_payload should succeed");

        assert_eq!(payload["newPx"], "51000");
        assert_eq!(payload["newSz"], "0.2");
    }

    #[test]
    fn test_build_edit_order_missing_both() {
        let market = mock_market("BTC/USDT", "BTC-USDT", MarketType::Spot);
        let result = build_edit_order_payload("123456789", &market, None, None, None);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("At least one of price or qty"));
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
        assert_eq!(map_order_type(OrderType::LimitMaker), "post_only");
        assert_eq!(map_order_type(OrderType::Market), "market");
        assert_eq!(map_order_type(OrderType::StopLoss), "market");
        assert_eq!(map_order_type(OrderType::TakeProfit), "market");
    }

    #[test]
    fn test_is_okx_supported_order_type() {
        assert!(is_okx_supported_order_type(OrderType::Market));
        assert!(is_okx_supported_order_type(OrderType::Limit));
        assert!(is_okx_supported_order_type(OrderType::LimitMaker));
        assert!(is_okx_supported_order_type(OrderType::StopLoss));
        assert!(is_okx_supported_order_type(OrderType::StopMarket));
        assert!(is_okx_supported_order_type(OrderType::TakeProfit));

        assert!(!is_okx_supported_order_type(OrderType::StopLossLimit));
        assert!(!is_okx_supported_order_type(OrderType::TakeProfitLimit));
        assert!(!is_okx_supported_order_type(OrderType::TrailingStop));
    }
}

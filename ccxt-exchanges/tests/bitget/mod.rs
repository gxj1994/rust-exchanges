//! Bitget 交易所测试模块
//!
//! 本模块按测试类型分类整合所有 Bitget 测试:
//! - integration: 集成测试
//! - order_query: 订单查询测试
//! - swap_order_query: 合约订单类型测试(市价/限价/计划单等)
//! - spot_order_types: 现货订单类型测试(市价/限价/Post-Only/TP/SL等)
//! - position_query: 仓位管理测试
//! - property: 属性测试
//! - websocket: WebSocket 测试

pub mod integration;
pub mod property;
pub mod spot_swap_mixed;
pub mod websocket;

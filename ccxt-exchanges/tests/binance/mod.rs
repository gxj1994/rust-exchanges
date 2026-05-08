//! Binance 交易所测试模块
//!
//! 本模块按测试类型分类整合所有 Binance 测试：
//! - integration: 集成测试
//! - spot_market_data: 现货市场数据 API 测试
//! - swap_market_data: U本位合约市场数据 API 测试
//! - order_query: 订单查询测试
//! - property: 属性测试
//! - websocket: WebSocket测试

pub mod integration;
pub mod order_query;
pub mod property;
pub mod spot;
pub mod spot_swap_mixed;
pub mod swap;
pub mod websocket;

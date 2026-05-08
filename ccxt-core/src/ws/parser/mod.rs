//! 消息解析模块
//!
//! 提供流解析器 trait 和统一的消息类型
//!
//! # 迁移指南
//!
//! 本模块正在从"多方法"设计迁移到"泛型"设计。
//! 参见 [`StreamParserExt`] 和 [`Parseable`] 了解新设计。

mod orderbook;
mod r#trait;

pub use orderbook::{OrderBookDeltaParser, OrderBookMessageType};
pub use r#trait::{
    ExchangeMessage, ParseResult, Parseable, ParsedMessage, StreamParser, StreamParserExt,
};

// 重新导出宏
pub use crate::define_parseable;

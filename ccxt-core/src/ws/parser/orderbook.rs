//! OrderBook 增量解析器模块
//!
//! 提供统一的 OrderBook 增量更新解析和管理接口

use crate::error::Result;
use crate::types::{OrderBookDelta, Symbol};
use serde_json::Value;

/// OrderBook 增量解析器 trait
///
/// 交易所实现此 trait 来解析 WebSocket 推送的 OrderBook 增量更新消息
pub trait OrderBookDeltaParser: Clone + Send + Sync + 'static {
    /// 解析增量消息
    ///
    /// # Arguments
    /// * `msg` - WebSocket 消息
    /// * `symbol` - 交易对符号
    ///
    /// # Returns
    /// 解析后的 OrderBookDelta
    fn parse_delta(&self, msg: &Value, symbol: &Symbol) -> Result<OrderBookDelta>;

    /// 判断消息是否为增量更新
    ///
    /// # Arguments
    /// * `msg` - WebSocket 消息
    ///
    /// # Returns
    /// true 如果是增量消息，false 如果是快照或其他类型
    fn is_delta(&self, msg: &Value) -> bool;

    /// 判断消息是否为快照（全量）更新
    ///
    /// # Arguments
    /// * `msg` - WebSocket 消息
    ///
    /// # Returns
    /// true 如果是快照消息
    fn is_snapshot(&self, msg: &Value) -> bool;

    /// 从消息中提取 symbol
    ///
    /// 用于在无法从订阅上下文获取 symbol 时，从消息中解析
    ///
    /// # Arguments
    /// * `msg` - WebSocket 消息
    ///
    /// # Returns
    /// 解析出的 symbol 字符串，如果无法解析返回 None
    fn extract_symbol(&self, msg: &Value) -> Option<String> {
        // 默认实现返回 None，交易所可以覆盖
        let _ = msg;
        None
    }

    /// 从消息中提取序列号 (nonce/update_id)
    ///
    /// # Arguments
    /// * `msg` - WebSocket 消息
    ///
    /// # Returns
    /// 序列号，如果消息中没有返回 None
    fn extract_nonce(&self, msg: &Value) -> Option<i64> {
        // 默认实现尝试常见字段
        msg.get("u")
            .and_then(|v| v.as_i64())
            .or_else(|| msg.get("lastUpdateId").and_then(|v| v.as_i64()))
            .or_else(|| msg.get("seq").and_then(|v| v.as_i64()))
    }
}

/// OrderBook 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderBookMessageType {
    /// 快照（全量）更新
    Snapshot,
    /// 增量更新
    Delta,
    /// 未知类型
    Unknown,
}

/// OrderBook 解析辅助函数
#[allow(unused)]
pub mod utils {
    use super::*;

    /// 解析 OrderBook 一侧（bids 或 asks）
    ///
    /// # Arguments
    /// * `data` - JSON 数组，格式为 [[price, amount], ...]
    ///
    /// # Returns
    /// 解析后的 OrderBookEntry 向量
    pub fn parse_orderbook_side(data: &Value) -> Result<Vec<crate::types::OrderBookEntry>> {
        use crate::types::{Amount, OrderBookEntry, Price};
        use rust_decimal::Decimal;
        use std::str::FromStr;

        let array = data.as_array().ok_or_else(|| {
            crate::error::Error::invalid_request("Expected array for orderbook side")
        })?;

        let mut result = Vec::with_capacity(array.len());

        for item in array {
            if let Some(arr) = item.as_array() {
                if arr.len() >= 2 {
                    let price_str = arr[0]
                        .as_str()
                        .map(|s| s.to_string())
                        .or_else(|| arr[0].as_f64().map(|f| f.to_string()))
                        .ok_or_else(|| {
                            crate::error::Error::invalid_request("Invalid price format")
                        })?;

                    let amount_str = arr[1]
                        .as_str()
                        .map(|s| s.to_string())
                        .or_else(|| arr[1].as_f64().map(|f| f.to_string()))
                        .ok_or_else(|| {
                            crate::error::Error::invalid_request("Invalid amount format")
                        })?;

                    let price = Decimal::from_str(&price_str)
                        .map_err(|_| crate::error::Error::invalid_request("Cannot parse price"))?;
                    let amount = Decimal::from_str(&amount_str)
                        .map_err(|_| crate::error::Error::invalid_request("Cannot parse amount"))?;

                    result.push(OrderBookEntry {
                        price: Price::new(price),
                        amount: Amount::new(amount),
                    });
                }
            }
        }

        Ok(result)
    }

    /// 解析标准格式的 OrderBookDelta
    ///
    /// 适用于大多数交易所的标准格式：
    /// ```json
    /// {
    ///   "U": 100,  // first_update_id
    ///   "u": 200,  // final_update_id
    ///   "pu": 99,  // prev_final_update_id (optional)
    ///   "b": [["10000", "1.5"], ...],  // bids
    ///   "a": [["10001", "2.0"], ...]   // asks
    /// }
    /// ```
    pub fn parse_standard_delta(
        msg: &Value,
        symbol: Symbol,
        first_id_key: &str,
        final_id_key: &str,
        prev_id_key: Option<&str>,
        bids_key: &str,
        asks_key: &str,
        timestamp_key: Option<&str>,
    ) -> Result<OrderBookDelta> {
        let first_update_id = msg
            .get(first_id_key)
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                crate::error::Error::invalid_request(format!(
                    "Missing {} (first_update_id)",
                    first_id_key
                ))
            })?;

        let final_update_id = msg
            .get(final_id_key)
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                crate::error::Error::invalid_request(format!(
                    "Missing {} (final_update_id)",
                    final_id_key
                ))
            })?;

        let prev_final_update_id =
            prev_id_key.and_then(|key| msg.get(key).and_then(|v| v.as_i64()));

        let timestamp = timestamp_key
            .and_then(|key| msg.get(key).and_then(|v| v.as_i64()))
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        let bids = parse_orderbook_side(&msg[bids_key])?;
        let asks = parse_orderbook_side(&msg[asks_key])?;

        Ok(OrderBookDelta {
            symbol,
            first_update_id,
            final_update_id,
            prev_final_update_id,
            timestamp,
            bids,
            asks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[derive(Clone)]
    struct TestParser;

    impl OrderBookDeltaParser for TestParser {
        fn parse_delta(&self, msg: &Value, symbol: &Symbol) -> Result<OrderBookDelta> {
            utils::parse_standard_delta(
                msg,
                symbol.clone(),
                "U",
                "u",
                Some("pu"),
                "b",
                "a",
                Some("E"),
            )
        }

        fn is_delta(&self, msg: &Value) -> bool {
            msg.get("e").and_then(|v| v.as_str()) == Some("depthUpdate")
        }

        fn is_snapshot(&self, msg: &Value) -> bool {
            msg.get("e").and_then(|v| v.as_str()) == Some("depthSnapshot")
        }
    }

    #[test]
    fn test_parse_standard_delta() {
        let msg = json!({
            "e": "depthUpdate",
            "E": 1234567890i64,
            "U": 100i64,
            "u": 200i64,
            "pu": 99i64,
            "b": [["50000.00", "1.5"], ["49900.00", "2.0"]],
            "a": [["50100.00", "1.0"], ["50200.00", "0.5"]]
        });

        let parser = TestParser;
        let symbol = Symbol::new_unchecked("BTC/USDT");
        let delta = parser.parse_delta(&msg, &symbol).unwrap();

        assert_eq!(delta.first_update_id, 100);
        assert_eq!(delta.final_update_id, 200);
        assert_eq!(delta.prev_final_update_id, Some(99));
        assert_eq!(delta.timestamp, 1234567890);
        assert_eq!(delta.bids.len(), 2);
        assert_eq!(delta.asks.len(), 2);

        assert_eq!(delta.bids[0].price.as_decimal(), dec!(50000.00));
        assert_eq!(delta.bids[0].amount.as_decimal(), dec!(1.5));
    }

    #[test]
    fn test_is_delta() {
        let parser = TestParser;

        let delta_msg = json!({ "e": "depthUpdate" });
        assert!(parser.is_delta(&delta_msg));

        let snapshot_msg = json!({ "e": "depthSnapshot" });
        assert!(!parser.is_delta(&snapshot_msg));
    }

    #[test]
    fn test_extract_nonce() {
        let parser = TestParser;

        let msg1 = json!({ "u": 100i64 });
        assert_eq!(parser.extract_nonce(&msg1), Some(100));

        let msg2 = json!({ "lastUpdateId": 200i64 });
        assert_eq!(parser.extract_nonce(&msg2), Some(200));

        let msg3 = json!({});
        assert_eq!(parser.extract_nonce(&msg3), None);
    }
}

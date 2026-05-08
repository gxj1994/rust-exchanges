//! Leverage operations for Bitget futures/swap.

use super::super::super::signed_request::HttpMethod;
use super::super::super::{Bitget, core::symbol::BitgetSymbolConverter};
use ccxt_core::{Error, ParseError, Result};

impl Bitget {
    /// Set leverage for a symbol.
    ///
    /// Uses Bitget V3 POST `/api/v3/account/set-leverage` endpoint.
    pub async fn set_leverage_impl(
        &self,
        symbol: &str,
        leverage: u32,
        margin_mode: Option<&str>,
    ) -> Result<()> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let category = Self::product_type_to_category(&self.options().effective_product_type());

        // V3 API 请求体参数
        let mut body = serde_json::Map::new();
        body.insert(
            "category".to_string(),
            serde_json::Value::String(category.to_string()),
        );
        body.insert("symbol".to_string(), serde_json::Value::String(exchange_id));
        body.insert(
            "leverage".to_string(),
            serde_json::Value::String(leverage.to_string()),
        );

        // 逐仓模式需要 posSide 参数
        // 注意：V3 API 文档说明逐仓杠杆暂不支持通过 API 调整（预计2026年Q1上线）
        // 但为了兼容性，我们仍然保留这个逻辑
        if margin_mode == Some("isolated") {
            // 对于逐仓模式，需要分别设置多仓和空仓的杠杆
            // 先设置多仓
            body.insert(
                "posSide".to_string(),
                serde_json::Value::String("long".to_string()),
            );
            let body_value = serde_json::Value::Object(body.clone());

            self.signed_request("/api/v3/account/set-leverage")
                .method(HttpMethod::Post)
                .body(body_value)
                .execute()
                .await?;

            // 再设置空仓
            body.insert(
                "posSide".to_string(),
                serde_json::Value::String("short".to_string()),
            );
            let body_value = serde_json::Value::Object(body);

            self.signed_request("/api/v3/account/set-leverage")
                .method(HttpMethod::Post)
                .body(body_value)
                .execute()
                .await?;
        } else {
            // 全仓模式，不需要 posSide
            let body_value = serde_json::Value::Object(body);

            self.signed_request("/api/v3/account/set-leverage")
                .method(HttpMethod::Post)
                .body(body_value)
                .execute()
                .await?;
        }

        Ok(())
    }

    /// Get current leverage for a symbol.
    ///
    /// Uses Bitget V3 GET `/api/v3/account/settings` endpoint.
    pub async fn get_leverage_impl(&self, symbol: &str) -> Result<u32> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let category = Self::product_type_to_category(&self.options().effective_product_type());

        // V3 API: /api/v3/account/settings 不需要参数
        let response = self
            .signed_request("/api/v3/account/settings")
            .execute()
            .await?;

        // 从 symbolConfigList 中查找对应交易对的 leverage
        let symbol_configs = response["data"]["symbolConfigList"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbolConfigList")))?;

        for config in symbol_configs {
            let config_symbol = config["symbol"].as_str().unwrap_or("");
            let config_category = config["category"].as_str().unwrap_or("");

            if config_symbol == exchange_id && config_category == category {
                let lever_str = config["leverage"]
                    .as_str()
                    .ok_or_else(|| Error::from(ParseError::missing_field("leverage")))?;

                return lever_str.parse::<u32>().map_err(|_| {
                    Error::from(ParseError::invalid_value(
                        "leverage",
                        format!("Cannot parse leverage: {}", lever_str),
                    ))
                });
            }
        }

        Err(Error::from(ParseError::invalid_value(
            "symbol",
            format!("Symbol {} not found in account settings", symbol),
        )))
    }

    /// Set position mode (one-way or hedge mode).
    ///
    /// Uses Bitget V3 POST `/api/v3/account/set-hold-mode` endpoint.
    ///
    /// # Arguments
    /// * `hold_mode` - Position mode: "one_way_mode" or "hedge_mode"
    pub async fn set_hold_mode(&self, hold_mode: &str) -> Result<()> {
        let hold_mode = match hold_mode.to_lowercase().as_str() {
            "one_way_mode" | "oneway" | "one-way" => "one_way_mode",
            "hedge_mode" | "hedge" => "hedge_mode",
            _ => {
                return Err(Error::invalid_request(format!(
                    "Invalid hold mode: {}. Must be 'one_way_mode' or 'hedge_mode'",
                    hold_mode
                )));
            }
        };

        let mut body = serde_json::Map::new();
        body.insert(
            "holdMode".to_string(),
            serde_json::Value::String(hold_mode.to_string()),
        );
        let body_value = serde_json::Value::Object(body);

        self.signed_request("/api/v3/account/set-hold-mode")
            .method(HttpMethod::Post)
            .body(body_value)
            .execute()
            .await?;

        Ok(())
    }
}

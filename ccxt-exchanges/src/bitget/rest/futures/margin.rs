//! Margin mode operations for Bitget futures/swap.

use super::super::super::{Bitget, core::symbol::BitgetSymbolConverter};
use ccxt_core::{Error, ParseError, Result};

impl Bitget {
    /// Get current margin mode for a symbol.
    ///
    /// Uses Bitget V3 GET `/api/v3/account/settings` endpoint.
    ///
    /// # Returns
    /// - `"crossed"` for cross margin mode
    /// - `"isolated"` for isolated margin mode
    pub async fn get_margin_mode_impl(&self, symbol: &str) -> Result<String> {
        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let category = Self::product_type_to_category(&self.options().effective_product_type());

        // V3 API: /api/v3/account/settings 不需要参数
        let response = self
            .signed_request("/api/v3/account/settings")
            .execute()
            .await?;

        // 从 symbolConfigList 中查找对应交易对的 marginMode
        let symbol_configs = response["data"]["symbolConfigList"]
            .as_array()
            .ok_or_else(|| Error::from(ParseError::missing_field("symbolConfigList")))?;

        for config in symbol_configs {
            let config_symbol = config["symbol"].as_str().unwrap_or("");
            let config_category = config["category"].as_str().unwrap_or("");

            if config_symbol == exchange_id && config_category == category {
                let margin_mode = config["marginMode"]
                    .as_str()
                    .ok_or_else(|| Error::from(ParseError::missing_field("marginMode")))?
                    .to_string();
                return Ok(margin_mode);
            }
        }

        Err(Error::from(ParseError::invalid_value(
            "symbol",
            format!("Symbol {} not found in account settings", symbol),
        )))
    }

    /// Set margin mode (cross or isolated) for a symbol.
    ///
    /// Note: V3 API does not support setting margin mode via API yet.
    /// This is documented in the official API docs.
    /// Users need to set margin mode through the web interface.
    pub async fn set_margin_mode_impl(&self, _symbol: &str, _mode: &str) -> Result<()> {
        // V3 API 暂不支持通过 API 设置逐仓保证金模式
        // 官方文档说明：预计将于 2026 年第二季度上线
        Err(Error::invalid_request(
            "Setting margin mode via API is not yet supported in V3 API. \
             Please set margin mode through the web interface. \
             Isolated margin mode API support is expected in Q2 2026."
                .to_string(),
        ))
    }
}

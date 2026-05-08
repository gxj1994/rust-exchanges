//! Position management methods for Bitget futures/swap.

use super::super::super::{Bitget, core::symbol::BitgetSymbolConverter, parser};
use ccxt_core::{Error, ParseError, Result, types::Position};
use std::collections::HashMap;
use tracing::warn;

impl Bitget {
    /// Fetch a single position for a symbol.
    ///
    /// Uses Bitget V3 GET `/api/v3/position/current-position` endpoint.
    ///
    /// # Note
    /// Only works for contract symbols (contains ':'). Spot symbols will fail.
    pub async fn fetch_position_impl(&self, symbol: &str) -> Result<Position> {
        // Validate that this is a contract symbol
        if !BitgetSymbolConverter::is_contract(symbol) {
            return Err(Error::from(ParseError::missing_field_owned(format!(
                "Position API only supports contract symbols (e.g., BTC/USDT:USDT), got: {}",
                symbol
            ))));
        }

        let exchange_id = BitgetSymbolConverter::unified_to_exchange(symbol);
        let category = Self::product_type_to_category(
            &BitgetSymbolConverter::product_type_from_symbol(symbol),
        );

        let response = self
            .signed_request("/api/v3/position/current-position")
            .param("category", category)
            .param("symbol", &exchange_id)
            .execute()
            .await?;

        // V3 API 返回结构: { "data": { "list": [...] } }
        let list = response["data"]["list"].as_array().ok_or_else(|| {
            Error::from(ParseError::invalid_format(
                "data.list",
                "Expected data.list array",
            ))
        })?;

        if list.is_empty() {
            return Err(Error::from(ParseError::missing_field_owned(format!(
                "No position found for symbol: {}",
                symbol
            ))));
        }

        parser::parse_position(&list[0], symbol)
    }

    /// Fetch positions for specific symbols, or all positions if empty.
    ///
    /// Uses Bitget V3 GET `/api/v3/position/current-position` endpoint.
    ///
    /// # Strategy
    /// - If symbols is empty: Fetch all positions by product type (one API call per type)
    /// - If symbols provided: Group by product type, fetch each type once, then filter
    pub async fn fetch_positions_impl(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        if symbols.is_empty() {
            // 获取所有仓位：按产品类型调用
            self.fetch_all_positions_by_product_type().await
        } else {
            // 按产品类型分组，减少 API 调用次数
            self.fetch_positions_by_symbols(symbols).await
        }
    }

    /// Fetch all positions grouped by product type.
    async fn fetch_all_positions_by_product_type(&self) -> Result<Vec<Position>> {
        let product_types = ["umcbl", "dmcbl", "sumcbl"]; // USDT, COIN, USDC
        let mut all_positions = Vec::new();

        for product_type in product_types {
            let category = Self::product_type_to_category(product_type);

            let response = self
                .signed_request("/api/v3/position/current-position")
                .param("category", category)
                .execute()
                .await?;

            let list = response["data"]["list"].as_array();
            if let Some(positions_array) = list {
                for position_data in positions_array {
                    let bitget_symbol = position_data["symbol"].as_str().unwrap_or_default();
                    let symbol_hint = BitgetSymbolConverter::exchange_to_unified_hint(
                        bitget_symbol,
                        product_type,
                    );

                    match parser::parse_position(position_data, &symbol_hint) {
                        Ok(position) => all_positions.push(position),
                        Err(e) => {
                            warn!(error = %e, symbol = %bitget_symbol, "Failed to parse Bitget position");
                        }
                    }
                }
            }
        }

        Ok(all_positions)
    }

    /// Fetch positions for specific symbols, grouped by product type.
    async fn fetch_positions_by_symbols(&self, symbols: &[&str]) -> Result<Vec<Position>> {
        // 按产品类型分组
        let mut grouped_by_type: HashMap<&str, Vec<&str>> = HashMap::new();

        for &symbol in symbols {
            let product_type = BitgetSymbolConverter::product_type_from_symbol(symbol);
            grouped_by_type
                .entry(product_type)
                .or_default()
                .push(symbol);
        }

        // 每个产品类型调用一次 API
        let mut all_positions = Vec::new();

        for (product_type, symbols_in_group) in &grouped_by_type {
            let category = Self::product_type_to_category(product_type);

            // 不带 symbol 参数获取该产品类型的所有仓位
            let response = self
                .signed_request("/api/v3/position/current-position")
                .param("category", category)
                .execute()
                .await?;

            let list = response["data"]["list"].as_array();
            if let Some(positions_array) = list {
                for position_data in positions_array {
                    let bitget_symbol = position_data["symbol"].as_str().unwrap_or_default();
                    let symbol_hint = BitgetSymbolConverter::exchange_to_unified_hint(
                        bitget_symbol,
                        product_type,
                    );

                    match parser::parse_position(position_data, &symbol_hint) {
                        Ok(position) => {
                            // 过滤：只保留请求的 symbols
                            if symbols_in_group.iter().any(|&s| position.symbol == s) {
                                all_positions.push(position);
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, symbol = %bitget_symbol, "Failed to parse Bitget position");
                        }
                    }
                }
            }
        }

        Ok(all_positions)
    }
}

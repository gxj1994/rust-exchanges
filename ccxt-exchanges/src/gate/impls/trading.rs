//! Gate.io Trading trait implementation.
//!
//! This module implements the `Trading` trait from `ccxt-core` for Gate.io,
//! providing order management capabilities for both spot and contract markets.
//!
//! # Routing
//!
//! All methods route to spot or contract implementations based on
//! the market's `market_type` (or `default_type` when symbol is None).

use async_trait::async_trait;
use ccxt_core::{
    Result,
    traits::{MarketData, Trading},
    types::market::MarketType,
    types::{Order, OrderRequest, Trade},
};

use crate::gate::Gate;

/// Determine if contract methods should be used, based on default_type.
fn is_contract(default_type: &ccxt_core::types::common::default_type::DefaultType) -> bool {
    matches!(
        default_type,
        ccxt_core::types::common::default_type::DefaultType::Swap
            | ccxt_core::types::common::default_type::DefaultType::Futures
    )
}

/// Resolve market_type for a symbol by looking up markets.
async fn resolve_market_type(gate: &Gate, symbol: &str) -> Result<MarketType> {
    let markets = gate.load_markets_with_reload(false).await?;
    let market = markets
        .get(symbol)
        .ok_or_else(|| ccxt_core::Error::bad_symbol(format!("Market {} not found", symbol)))?;
    Ok(market.market_type)
}

#[async_trait]
impl Trading for Gate {
    async fn create_order(&self, request: OrderRequest) -> Result<Order> {
        let market_type = resolve_market_type(self, &request.symbol).await?;
        match market_type {
            MarketType::Spot => Gate::create_spot_order(self, request).await,
            MarketType::Swap | MarketType::Futures => {
                Gate::create_contract_order(self, request).await
            }
            _ => Err(ccxt_core::Error::invalid_request(format!(
                "Unsupported market type {:?} for create_order on Gate",
                market_type
            ))),
        }
    }

    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market_type = resolve_market_type(self, symbol).await?;
        match market_type {
            MarketType::Spot => Gate::cancel_spot_order(self, id, symbol).await,
            MarketType::Swap | MarketType::Futures => {
                Gate::cancel_contract_order(self, id, symbol).await
            }
            _ => Err(ccxt_core::Error::invalid_request(format!(
                "Unsupported market type {:?} for cancel_order on Gate",
                market_type
            ))),
        }
    }

    async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        let market_type = resolve_market_type(self, symbol).await?;
        match market_type {
            MarketType::Spot => Gate::cancel_all_spot_orders(self, symbol).await,
            MarketType::Swap | MarketType::Futures => {
                Gate::cancel_all_contract_orders(self, symbol).await
            }
            _ => Err(ccxt_core::Error::invalid_request(format!(
                "Unsupported market type {:?} for cancel_all_orders on Gate",
                market_type
            ))),
        }
    }

    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        let market_type = resolve_market_type(self, symbol).await?;
        match market_type {
            MarketType::Spot => Gate::fetch_spot_order(self, id, symbol).await,
            MarketType::Swap | MarketType::Futures => {
                Gate::fetch_contract_order(self, id, symbol).await
            }
            _ => Err(ccxt_core::Error::invalid_request(format!(
                "Unsupported market type {:?} for fetch_order on Gate",
                market_type
            ))),
        }
    }

    async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>> {
        match symbol {
            Some(sym) => {
                let market_type = resolve_market_type(self, sym).await?;
                match market_type {
                    MarketType::Spot => Gate::fetch_spot_open_orders(self, Some(sym)).await,
                    MarketType::Swap | MarketType::Futures => {
                        Gate::fetch_contract_open_orders(self, Some(sym)).await
                    }
                    _ => Err(ccxt_core::Error::invalid_request(format!(
                        "Unsupported market type {:?} for fetch_open_orders on Gate",
                        market_type
                    ))),
                }
            }
            None => {
                if is_contract(&self.options.default_type) {
                    Gate::fetch_contract_open_orders(self, None).await
                } else {
                    Gate::fetch_spot_open_orders(self, None).await
                }
            }
        }
    }

    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>> {
        match symbol {
            Some(sym) => {
                let market_type = resolve_market_type(self, sym).await?;
                match market_type {
                    MarketType::Spot => {
                        Gate::fetch_spot_history_orders(self, Some(sym), since, limit).await
                    }
                    MarketType::Swap | MarketType::Futures => {
                        Gate::fetch_contract_history_orders(self, Some(sym), since, limit).await
                    }
                    _ => Err(ccxt_core::Error::invalid_request(format!(
                        "Unsupported market type {:?} for fetch_history_orders on Gate",
                        market_type
                    ))),
                }
            }
            None => {
                if is_contract(&self.options.default_type) {
                    Gate::fetch_contract_history_orders(self, None, since, limit).await
                } else {
                    Gate::fetch_spot_history_orders(self, None, since, limit).await
                }
            }
        }
    }

    async fn fetch_trades_with_limit(
        &self,
        symbol: &str,
        _since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        let market_type = resolve_market_type(self, symbol).await?;
        match market_type {
            MarketType::Spot => Gate::fetch_spot_market_trades(self, symbol, limit).await,
            MarketType::Swap | MarketType::Futures => {
                Gate::fetch_contract_market_trades(self, symbol, limit).await
            }
            _ => Err(ccxt_core::Error::invalid_request(format!(
                "Unsupported market type {:?} for fetch_trades on Gate",
                market_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::GateBuilder;

    #[test]
    fn test_gate_trading_trait_object_safety() {
        let gate = GateBuilder::default().build().unwrap();
        let trading: Box<dyn Trading> = Box::new(gate);

        assert_eq!(trading.id(), "gate");
        // Trading capabilities will be enabled when methods are implemented
    }
}

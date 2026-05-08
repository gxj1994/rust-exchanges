//! WsExchange trait implementation for HyperLiquid
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for HyperLiquid.
//!
//! Uses the new ws_v2 architecture with:
//! - Message broadcasting (single message loop + multiple subscribers)
//! - Reference counting for same channel subscriptions
//! - Automatic reconnection with subscription recovery
//!
//! Connection state is queried directly from the underlying WsClient.

use async_trait::async_trait;
use ccxt_core::{
    error::{Error, Result},
    types::{Balance, Ohlcv, Order, OrderBook, Ticker, Timeframe, Trade},
    ws::SubscriptionChannel,
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};

use crate::common::build_subscription_channel;
use crate::hyperliquid::HyperLiquid;
use crate::hyperliquid::parser::ws::{parse_all_mids_map, parse_user_fills};

#[async_trait]
impl WsExchange for HyperLiquid {
    // ==================== Connection Management ====================

    async fn ws_connect(&self) -> Result<()> {
        let context = ccxt_core::ws::WsContext::new();
        self.ws_client().connect(&context).await
    }

    async fn ws_disconnect(&self) -> Result<()> {
        self.ws_client().disconnect().await
    }

    fn ws_is_connected(&self) -> bool {
        self.ws_client().is_connected_sync()
    }

    fn ws_state(&self) -> WsConnectionState {
        self.ws_client().state_sync()
    }

    // ==================== Public Data Streams ====================

    /// Watch ticker for a specific symbol.
    ///
    /// Hyperliquid uses `allMids` channel which returns mid prices for ALL coins.
    /// This method filters the allMids response to only return the requested symbol's ticker.
    async fn watch_ticker(&self, symbol: &str) -> Result<MessageStream<Ticker>> {
        let channel = SubscriptionChannel::ticker(symbol);
        let mut rx = self.ws_client().subscribe_and_receive(&channel).await?;
        let symbol = symbol.to_string();

        Ok(Box::pin(async_stream::stream! {
            while let Some(msg) = rx.recv().await {
                // Extract data field from allMids message
                let data = match msg.get("data") {
                    Some(d) => d,
                    None => continue,
                };
                // Parse all mids from the data
                match parse_all_mids_map(data) {
                    Ok(tickers) => {
                        // Filter by the requested symbol
                        for ticker in tickers {
                            if ticker.symbol.as_str() == &symbol {
                                yield Ok(ticker);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, symbol = %symbol, "Failed to parse allMids message");
                    }
                }
            }
        }))
    }

    /// Watch tickers for multiple symbols.
    ///
    /// Subscribes once to `allMids` and filters results for the requested symbols.
    async fn watch_tickers(&self, symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        if symbols.is_empty() {
            return Err(Error::invalid_request("symbols cannot be empty"));
        }

        // Subscribe once to allMids (all Ticker subscriptions share the same channel)
        let channel = SubscriptionChannel::ticker(&symbols[0]);
        let mut rx = self.ws_client().subscribe_and_receive(&channel).await?;
        let symbols: Vec<String> = symbols.to_vec();

        Ok(Box::pin(async_stream::stream! {
            while let Some(msg) = rx.recv().await {
                // Extract data field from allMids message
                let data = match msg.get("data") {
                    Some(d) => d,
                    None => continue,
                };
                match parse_all_mids_map(data) {
                    Ok(all_tickers) => {
                        // Filter to only requested symbols
                        let filtered: Vec<Ticker> = all_tickers
                            .into_iter()
                            .filter(|t| symbols.iter().any(|s| t.symbol.as_str() == s.as_str()))
                            .collect();
                        if !filtered.is_empty() {
                            yield Ok(filtered);
                        }
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "Failed to parse allMids message");
                    }
                }
            }
        }))
    }

    async fn watch_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<MessageStream<OrderBook>> {
        let params = limit.map(|l| vec![("limit".to_string(), l.into())]);
        self.ws_client().watch::<OrderBook>(symbol, params).await
    }

    async fn watch_market_trades(&self, symbol: &str) -> Result<MessageStream<Vec<Trade>>> {
        self.ws_client().watch::<Vec<Trade>>(symbol, None).await
    }

    async fn watch_ohlcv(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> Result<MessageStream<Vec<Ohlcv>>> {
        // Convert timeframe to Hyperliquid format (same as standard format)
        let interval = timeframe.to_string();
        let params = vec![("interval".to_string(), interval.into())];
        self.ws_client()
            .watch::<Vec<Ohlcv>>(symbol, Some(params))
            .await
    }

    // ==================== Private Data Streams ====================

    async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        // Check if credentials are available
        let _address = self.wallet_address().ok_or_else(|| {
            Error::authentication("watch_balance requires authentication (wallet address)")
        })?;

        self.ws_client().watch::<Balance>("", None).await
    }

    async fn watch_orders(&self, symbol: Option<&str>) -> Result<MessageStream<Order>> {
        // Check if credentials are available
        let _address = self.wallet_address().ok_or_else(|| {
            Error::authentication("watch_orders requires authentication (wallet address)")
        })?;

        let params = symbol.map(|s| vec![("symbol".to_string(), s.into())]);
        self.ws_client()
            .watch::<Order>(symbol.unwrap_or(""), params)
            .await
    }

    async fn watch_account_trades(
        &self,
        symbol_filter: Option<&str>,
    ) -> Result<MessageStream<Trade>> {
        // Check if credentials are available
        let _address = self.wallet_address().ok_or_else(|| {
            Error::authentication("watch_account_trades requires authentication (wallet address)")
        })?;

        // Subscribe to userFills channel
        let channel = SubscriptionChannel::account_trades(None);
        let client = self.ws_client();
        let mut rx = client.subscribe_and_receive(&channel).await?;
        let symbol_filter = symbol_filter.map(ToString::to_string);

        // Create a stream that parses userFills messages
        Ok(Box::pin(async_stream::stream! {
            while let Some(msg) = rx.recv().await {
                // Parse userFills
                match parse_user_fills(&msg) {
                    Ok(trades) => {
                        for trade in trades {
                            // Apply symbol filter if specified
                            if let Some(ref filter) = symbol_filter {
                                if trade.symbol.as_str() != filter {
                                    continue;
                                }
                            }
                            yield Ok(trade);
                        }
                    }
                    Err(e) => {
                        tracing::debug!(error = %e, "Failed to parse Hyperliquid userFills message");
                    }
                }
            }
        }))
    }

    // ==================== Subscription Management ====================

    async fn subscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()> {
        let sub_channel = build_subscription_channel(channel, symbol)?;
        self.ws_client().subscribe(&[sub_channel]).await
    }

    async fn unsubscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()> {
        let sub_channel = build_subscription_channel(channel, symbol)?;
        self.ws_client().unsubscribe(&[sub_channel]).await
    }

    fn subscriptions(&self) -> Vec<String> {
        self.ws_client().subscriptions_sync()
    }
}

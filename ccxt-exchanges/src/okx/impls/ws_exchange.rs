//! WsExchange trait implementation for OKX
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for OKX,
//! providing real-time WebSocket data streaming capabilities.
//!
//! Uses the new ws_v2 architecture with:
//! - Message broadcasting (single message loop + multiple subscribers)
//! - Reference counting for same channel subscriptions
//! - Automatic reconnection with subscription recovery
//!
//! The WebSocket client is lazily initialized and shared across all method calls.
//! Connection state is directly queried from the underlying `WsClient`, no duplicate state tracking.

use async_trait::async_trait;
use ccxt_core::{
    error::{Error, Result},
    types::{Balance, Ohlcv, Order, OrderBook, Ticker, Timeframe, Trade},
    ws::SubscriptionChannel,
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};
use futures::StreamExt;

use crate::common::{build_subscription_channel, timeframe_to_uppercase_interval};
use crate::okx::Okx;
use crate::okx::parser::ws::parse_ws_account_trade;

#[async_trait]
impl WsExchange for Okx {
    // ==================== Connection Management ====================

    async fn ws_connect(&self) -> Result<()> {
        // Create context with market_type based on exchange options
        use ccxt_core::types::common::default_type::DefaultType;
        use ccxt_core::ws::subscription::MarketType as WsMarketType;

        let market_type = match self.options().default_type {
            DefaultType::Swap | DefaultType::Futures => {
                // Use Swap for both Swap and Futures types
                WsMarketType::Swap
            }
            DefaultType::Option => WsMarketType::Option,
            _ => WsMarketType::Spot, // Spot, Margin, etc.
        };

        let context = ccxt_core::ws::WsContext::new().with_market_type(market_type);
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

    async fn watch_ticker(&self, symbol: &str) -> Result<MessageStream<Ticker>> {
        self.ws_client().watch::<Ticker>(symbol, None).await
    }

    async fn watch_tickers(&self, symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        if symbols.is_empty() {
            return Err(Error::invalid_request("symbols cannot be empty"));
        }

        // Subscribe to all ticker channels
        let channels: Vec<SubscriptionChannel> = symbols
            .iter()
            .map(|s| SubscriptionChannel::ticker(s))
            .collect();

        let client = self.ws_client();
        client.subscribe(&channels).await?;

        // Create individual streams and merge them
        // Reuses parse_ws_ticker from ws.rs for consistent parsing
        let mut streams = Vec::new();
        for symbol in symbols {
            let stream = client.watch::<Ticker>(symbol, None).await?;
            streams.push(stream.map(|r| r.map(|t| vec![t])));
        }

        // Merge all streams using select_all
        let merged = futures::stream::select_all(streams);
        Ok(Box::pin(merged))
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
        // Convert timeframe to OKX format (uppercase H, D, W, M)
        let interval = timeframe_to_uppercase_interval(timeframe);
        let params = vec![("interval".to_string(), interval.into())];
        self.ws_client()
            .watch::<Vec<Ohlcv>>(symbol, Some(params))
            .await
    }

    // ==================== Private Data Streams ====================

    async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        // Check if credentials are available
        let _auth = self
            .get_auth()
            .map_err(|_| Error::authentication("API credentials required for watch_balance"))?;

        self.ws_client().watch::<Balance>("", None).await
    }

    async fn watch_orders(&self, symbol: Option<&str>) -> Result<MessageStream<Order>> {
        // Check if credentials are available
        let _auth = self
            .get_auth()
            .map_err(|_| Error::authentication("API credentials required for watch_orders"))?;

        let params = symbol.map(|s| vec![("symbol".to_string(), s.into())]);
        self.ws_client()
            .watch::<Order>(symbol.unwrap_or(""), params)
            .await
    }

    async fn watch_account_trades(&self, symbol: Option<&str>) -> Result<MessageStream<Trade>> {
        // Check if credentials are available
        let _auth = self.get_auth().map_err(|_| {
            Error::authentication("API credentials required for watch_account_trades")
        })?;

        // Subscribe to orders channel - OKX reports fills through the orders channel
        // Reuse the logic pattern from ws.rs::OkxWs::watch_account_trades
        let channel = SubscriptionChannel::orders(symbol.map(ToString::to_string));
        let client = self.ws_client();
        let mut rx = client.subscribe_and_receive(&channel).await?;
        let symbol_filter = symbol.map(ToString::to_string);

        // Create a stream that extracts trade information from order updates
        // Reuses parse_ws_account_trade from ws.rs for consistent parsing
        Ok(Box::pin(async_stream::stream! {
            while let Some(msg) = rx.recv().await {
                // Check if this is an orders message
                if let Some(arg) = msg.get("arg") {
                    let is_orders = arg.get("channel").and_then(|c| c.as_str()) == Some("orders");
                    if is_orders {
                        if let Some(data_array) = msg.get("data").and_then(|d| d.as_array()) {
                            for data in data_array {
                                // Only emit trades for filled/partially filled orders
                                let fill_sz = data
                                    .get("fillSz")
                                    .and_then(|v| v.as_str())
                                    .and_then(|s| s.parse::<f64>().ok())
                                    .unwrap_or(0.0);

                                if fill_sz <= 0.0 {
                                    continue;
                                }

                                // Apply symbol filter if specified
                                if let Some(ref filter) = symbol_filter {
                                    let inst_id = data.get("instId").and_then(|i| i.as_str()).unwrap_or("");
                                    let unified = inst_id.replace('-', "/");
                                    if unified != *filter && inst_id != filter.as_str() {
                                        continue;
                                    }
                                }

                                // Reuse parse_ws_account_trade from ws_v2
                                match parse_ws_account_trade(data) {
                                    Ok(trade) => yield Ok(trade),
                                    Err(e) => {
                                        tracing::debug!(error = %e, "Failed to parse OKX trade update");
                                    }
                                }
                            }
                        }
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

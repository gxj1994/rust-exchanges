//! WsExchange trait implementation for Bitget
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for Bitget,
//! providing real-time WebSocket data streaming capabilities.
//!
//! Uses the new ws_v2 architecture with:
//! - Message broadcasting (single message loop + multiple subscribers)
//! - Reference counting for same channel subscriptions
//! - Automatic reconnection with subscription recovery
//!
//! # Market Type Support
//!
//! The implementation automatically detects market type from the symbol format:
//! - `BTC/USDT` -> SPOT
//! - `BTC/USDT:USDT` -> USDT-FUTURES (linear perpetual)
//! - `BTC/USD:BTC` -> COIN-FUTURES (inverse perpetual)
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
use futures::StreamExt;

use crate::bitget::Bitget;
use crate::bitget::parser::ws::parse_ws_account_trade;
use crate::common::{build_subscription_channel, timeframe_to_uppercase_interval};

#[async_trait]
impl WsExchange for Bitget {
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

    async fn watch_ticker(&self, symbol: &str) -> Result<MessageStream<Ticker>> {
        self.ws_client().watch::<Ticker>(symbol, None).await
    }

    async fn watch_tickers(&self, symbols: &[String]) -> Result<MessageStream<Vec<Ticker>>> {
        if symbols.is_empty() {
            return Err(Error::invalid_request("symbols cannot be empty"));
        }

        let client = self.ws_client();

        // Subscribe to all ticker channels
        let channels: Vec<SubscriptionChannel> = symbols
            .iter()
            .map(|s| SubscriptionChannel::ticker(s))
            .collect();

        client.subscribe(&channels).await?;

        // Create individual streams and merge them
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
        // Convert timeframe to Bitget format
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

    async fn watch_account_trades(
        &self,
        symbol_filter: Option<&str>,
    ) -> Result<MessageStream<Trade>> {
        // Check if credentials are available
        let _auth = self.get_auth().map_err(|_| {
            Error::authentication("API credentials required for watch_account_trades")
        })?;

        // Subscribe to orders channel - Bitget reports fills through the orders channel
        let channel = SubscriptionChannel::orders(symbol_filter.map(ToString::to_string));
        let client = self.ws_client();
        let mut rx = client.subscribe_and_receive(&channel).await?;
        let symbol_filter = symbol_filter.map(ToString::to_string);

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
                                    .or_else(|| data.get("baseVolume"))
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
                                        tracing::debug!(error = %e, "Failed to parse Bitget trade update");
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

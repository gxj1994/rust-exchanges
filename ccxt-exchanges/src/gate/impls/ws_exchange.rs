//! WsExchange trait implementation for Gate
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for Gate,
//! providing real-time WebSocket data streaming capabilities.
//!
//! Uses the unified WebSocket architecture with:
//! - GateWsClient (GenericWsClient with Gate-specific components)
//! - GateSubscriptionBuilder for building subscription messages
//! - GateStreamParser for parsing incoming messages
//!
//! The WebSocket client is lazily initialized and shared across all method calls.

use async_trait::async_trait;
use ccxt_core::{
    error::{Error, Result},
    types::{Balance, Ohlcv, Order, OrderBook, Ticker, Timeframe, Trade},
    ws::subscription::MarketType as WsMarketType,
    ws::{SubscriptionChannel, WsContext},
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};
use futures::StreamExt;

use crate::gate::Gate;

/// Determine market type from Gate options
fn get_ws_market_type(gate: &Gate) -> WsMarketType {
    use ccxt_core::types::common::default_type::DefaultType;

    if matches!(
        gate.options.default_type,
        DefaultType::Swap | DefaultType::Futures
    ) {
        WsMarketType::Swap
    } else {
        WsMarketType::Spot
    }
}

/// Create ticker channel with market type from Gate options
#[allow(unused)]
fn ticker_channel(gate: &Gate, symbol: &str) -> SubscriptionChannel {
    let market_type = get_ws_market_type(gate);
    SubscriptionChannel::ticker(symbol).with_market_type(market_type)
}

/// Create orderbook channel with market type from Gate options
#[allow(unused)]
fn orderbook_channel(gate: &Gate, symbol: &str) -> SubscriptionChannel {
    let market_type = get_ws_market_type(gate);
    SubscriptionChannel::orderbook(symbol).with_market_type(market_type)
}

/// Create trades channel with market type from Gate options
#[allow(unused)]
fn trades_channel(gate: &Gate, symbol: &str) -> SubscriptionChannel {
    let market_type = get_ws_market_type(gate);
    SubscriptionChannel::trades(symbol).with_market_type(market_type)
}

/// Create kline channel with market type from Gate options
#[allow(unused)]
fn kline_channel(gate: &Gate, symbol: &str, interval: &str) -> SubscriptionChannel {
    let market_type = get_ws_market_type(gate);
    SubscriptionChannel::kline(symbol, interval).with_market_type(market_type)
}

#[async_trait]
impl WsExchange for Gate {
    // ==================== Connection Management ====================

    async fn ws_connect(&self) -> Result<()> {
        // Create context with market_type from Gate options
        // This ensures the correct WebSocket endpoint is selected
        let market_type = get_ws_market_type(self);
        let context = WsContext::new().with_market_type(market_type);
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
        // Convert timeframe to Gate format (e.g., "1m", "5m", "1h")
        let interval = timeframe.to_string();
        let params = vec![("interval".to_string(), interval.into())];
        self.ws_client()
            .watch::<Vec<Ohlcv>>(symbol, Some(params))
            .await
    }

    // ==================== Private Data Streams ====================
    //
    // Gate private channels use SubscribeAuthenticator pattern (HMAC-SHA512 signature).
    // Available private channels:
    // - spot.orders: Order updates
    // - spot.usertrades: User trade updates
    // - spot.balances: Balance updates

    async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        // Check credentials
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_balance"))?;

        // Get authenticated client
        let client = self.ws_client_auth()?;

        // Connect and subscribe
        let context = ccxt_core::ws::WsContext::new();
        client.connect(&context).await?;

        // Subscribe to balance channel (empty symbol for all balances)
        client.watch::<Balance>("", None).await
    }

    async fn watch_orders(&self, symbol: Option<&str>) -> Result<MessageStream<Order>> {
        // Check credentials
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_orders"))?;

        // Get authenticated client
        let client = self.ws_client_auth()?;

        // Connect and subscribe
        let context = ccxt_core::ws::WsContext::new();
        client.connect(&context).await?;

        // Subscribe to orders channel
        let sym = symbol.unwrap_or("");
        client.watch::<Order>(sym, None).await
    }

    async fn watch_account_trades(&self, symbol: Option<&str>) -> Result<MessageStream<Trade>> {
        // Check credentials
        self.base.check_required_credentials().map_err(|_| {
            Error::authentication("API credentials required for watch_account_trades")
        })?;

        // Get authenticated client
        let client = self.ws_client_auth()?;

        // Connect and subscribe
        let context = ccxt_core::ws::WsContext::new();
        client.connect(&context).await?;

        // Subscribe to user trades channel
        let sym = symbol.unwrap_or("");
        client.watch::<Trade>(sym, None).await
    }

    // ==================== Subscription Management ====================

    async fn subscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()> {
        let sym =
            symbol.ok_or_else(|| Error::invalid_request("symbol is required for subscription"))?;

        let sub_channel = match channel {
            "ticker" => SubscriptionChannel::ticker(sym),
            "orderbook" | "orderBook" => SubscriptionChannel::orderbook(sym),
            "trades" => SubscriptionChannel::trades(sym),
            "ohlcv" | "kline" => SubscriptionChannel::kline(sym, "1m"),
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unknown channel: {}",
                    channel
                )));
            }
        };

        self.ws_client().subscribe(&[sub_channel]).await
    }

    async fn unsubscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()> {
        let sym = symbol
            .ok_or_else(|| Error::invalid_request("symbol is required for unsubscription"))?;

        let sub_channel = match channel {
            "ticker" => SubscriptionChannel::ticker(sym),
            "orderbook" | "orderBook" => SubscriptionChannel::orderbook(sym),
            "trades" => SubscriptionChannel::trades(sym),
            "ohlcv" | "kline" => SubscriptionChannel::kline(sym, "1m"),
            _ => {
                return Err(Error::invalid_request(format!(
                    "Unknown channel: {}",
                    channel
                )));
            }
        };

        self.ws_client().unsubscribe(&[sub_channel]).await
    }

    fn subscriptions(&self) -> Vec<String> {
        self.ws_client().subscriptions_sync()
    }
}

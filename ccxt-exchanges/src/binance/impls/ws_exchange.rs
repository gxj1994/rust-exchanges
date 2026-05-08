//! WsExchange trait implementation for Binance
//!
//! This module implements the unified `WsExchange` trait from `ccxt-core` for Binance,
//! providing real-time WebSocket data streaming capabilities.
//!
//! Uses the new ws architecture with:
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
    ws::{StreamParser, SubscriptionChannel, WsContext},
    ws_client::WsConnectionState,
    ws_exchange::{MessageStream, WsExchange},
};
use futures::StreamExt;

use crate::binance::Binance;

#[async_trait]
impl WsExchange for Binance {
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
        // Binance builder expects "depth" parameter, not "limit"
        let params = limit.map(|l| vec![("depth".to_string(), (l as u64).into())]);
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
        // Convert timeframe to Binance format
        let interval = timeframe.to_string();
        let params = vec![("interval".to_string(), interval.into())];
        self.ws_client()
            .watch::<Vec<Ohlcv>>(symbol, Some(params))
            .await
    }

    // ==================== Private Data Streams ====================
    //
    // Private channels use the new GenericWsClient architecture with TokenProvider.
    // Requires `init_for_private_channels()` to be called first.

    async fn watch_balance(&self) -> Result<MessageStream<Balance>> {
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_balance"))?;

        // Check if private channels are initialized
        if !self.is_private_channels_initialized() {
            return Err(Error::invalid_request(
                "Private channels not initialized. Use Binance::new_arc() or call init_for_private_channels() first.",
            ));
        }

        // Get authenticated client and connect if needed
        let client = self.ws_client_auth()?;

        // Connect with token (automatically gets listenKey)
        let context = WsContext::new().with_private();
        client.connect_with_token(&context).await?;

        // Subscribe and return stream
        client.watch::<Balance>("", None).await
    }

    async fn watch_orders(&self, symbol: Option<&str>) -> Result<MessageStream<Order>> {
        self.base
            .check_required_credentials()
            .map_err(|_| Error::authentication("API credentials required for watch_orders"))?;

        // Check if private channels are initialized
        if !self.is_private_channels_initialized() {
            return Err(Error::invalid_request(
                "Private channels not initialized. Use Binance::new_arc() or call init_for_private_channels() first.",
            ));
        }

        // Get authenticated client and connect if needed
        let client = self.ws_client_auth()?;

        // Connect with token (automatically gets listenKey)
        let context = WsContext::new().with_private();
        client.connect_with_token(&context).await?;

        // Subscribe and return stream
        let params = symbol.map(|s| vec![("symbol".to_string(), s.into())]);
        client.watch::<Order>(symbol.unwrap_or(""), params).await
    }

    async fn watch_account_trades(&self, symbol: Option<&str>) -> Result<MessageStream<Trade>> {
        self.base.check_required_credentials().map_err(|_| {
            Error::authentication("API credentials required for watch_account_trades")
        })?;

        // Check if private channels are initialized
        if !self.is_private_channels_initialized() {
            return Err(Error::invalid_request(
                "Private channels not initialized. Use Binance::new_arc() or call init_for_private_channels() first.",
            ));
        }

        // Get authenticated client and connect if needed
        let client = self.ws_client_auth()?;

        // Connect with token (automatically gets listenKey)
        let context = WsContext::new().with_private();
        client.connect_with_token(&context).await?;

        // For account_trades, we need to subscribe to user data stream
        // Binance sends account_trades via executionReport events with filled quantity
        let channel = SubscriptionChannel::account_trades(symbol.map(ToString::to_string));

        // Subscribe and receive
        let mut rx = client.subscribe_and_receive(&channel).await?;
        let parser = super::super::ws::BinanceStreamParser;
        let symbol_filter = symbol.map(ToString::to_string);

        Ok(Box::pin(async_stream::stream! {
            while let Some(msg) = rx.recv().await {
                // Parse as account_trade (handles both spot and futures)
                match parser.parse(&msg) {
                    Ok(ccxt_core::ws::ParsedMessage::Trades(trades)) => {
                        // Filter by symbol if needed
                        for trade in trades {
                            if let Some(s) = &symbol_filter {
                                if trade.symbol.as_ref() != s {
                                    continue;
                                }
                            }
                            yield Ok(trade);
                        }
                    }
                    Ok(ccxt_core::ws::ParsedMessage::Error { message, .. }) => {
                        yield Err(Error::invalid_request(message));
                    }
                    Err(e) => {
                        yield Err(e);
                    }
                    _ => {
                        // Ignore other message types
                    }
                }
            }
        }))
    }

    // ==================== Subscription Management ====================

    async fn subscribe(&self, channel: &str, symbol: Option<&str>) -> Result<()> {
        let sub_channel =
            match channel {
                "ticker" => SubscriptionChannel::ticker(symbol.ok_or_else(|| {
                    Error::invalid_request("symbol is required for ticker channel")
                })?),
                "orderbook" | "orderBook" => {
                    SubscriptionChannel::orderbook(symbol.ok_or_else(|| {
                        Error::invalid_request("symbol is required for orderbook channel")
                    })?)
                }
                "trades" => SubscriptionChannel::trades(symbol.ok_or_else(|| {
                    Error::invalid_request("symbol is required for trades channel")
                })?),
                "ohlcv" | "kline" => SubscriptionChannel::kline(
                    symbol.ok_or_else(|| {
                        Error::invalid_request("symbol is required for ohlcv channel")
                    })?,
                    "1m",
                ),
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
        let sub_channel =
            match channel {
                "ticker" => SubscriptionChannel::ticker(symbol.ok_or_else(|| {
                    Error::invalid_request("symbol is required for ticker channel")
                })?),
                "orderbook" | "orderBook" => {
                    SubscriptionChannel::orderbook(symbol.ok_or_else(|| {
                        Error::invalid_request("symbol is required for orderbook channel")
                    })?)
                }
                "trades" => SubscriptionChannel::trades(symbol.ok_or_else(|| {
                    Error::invalid_request("symbol is required for trades channel")
                })?),
                "ohlcv" | "kline" => SubscriptionChannel::kline(
                    symbol.ok_or_else(|| {
                        Error::invalid_request("symbol is required for ohlcv channel")
                    })?,
                    "1m",
                ),
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

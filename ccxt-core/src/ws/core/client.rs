//! 泛型 WebSocket 客户端
//!
//! 组合 SubscriptionBuilder、StreamParser、WsEndpointProvider 和 WsAuthCore

use crate::error::Result;
use crate::network::ws_client::reconnect::AutoReconnectCoordinator;
use crate::network::ws_client::{WsClient, WsConfig, WsConnectionState};
use crate::types::market::orderbook_manager::OrderBookManager;
use crate::ws::WsEndpointProvider;
use crate::ws::auth::{MessageAuthenticator, NoAuth, TokenProvider, WsAuthCore};
use crate::ws::parser::{OrderBookDeltaParser, Parseable, StreamParser, StreamParserExt};
use crate::ws::subscription::{MarketType, SubscriptionBuilder, SubscriptionChannel};
use serde_json::Value;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};

/// 连接条目：存储 WebSocket 客户端和其市场类型上下文
#[derive(Debug)]
struct ConnectionEntry {
    /// WebSocket 客户端
    client: Arc<WsClient>,
    /// 此连接对应的市场类型（根据 URL 路由确定）
    #[allow(dead_code)]
    market_type: Option<crate::ws::subscription::MarketType>,
    /// 自动重连协调器
    coordinator: Option<AutoReconnectCoordinator>,
}

/// 泛型 WebSocket 客户端
///
/// 组合四个 trait 实现统一的 WebSocket 框架：
/// - `B`: SubscriptionBuilder - 构建订阅消息
/// - `P`: StreamParser - 解析消息
/// - `E`: WsEndpointProvider - 提供 WebSocket URL
/// - `A`: WsAuthCore - 认证策略
///
/// # 架构
///
/// ```text
/// ┌───────────────────────────────────────────────────────────────┐
/// │                GenericWsClient<B, P, E, A>                     │
/// ├───────────────────────────────────────────────────────────────┤
/// │  connections: HashMap<URL, Arc<WsClient>>    (多连接管理)     │
/// │  subscription_builder: B                     (订阅构建)       │
/// │  parser: P                                   (消息解析)       │
/// │  endpoint_provider: E                        (URL 提供)       │
/// │  auth_strategy: A                            (认证策略) ⭐    │
/// └───────────────────────────────────────────────────────────────┘
/// ```
///
/// # 核心特性
///
/// 1. **消息广播**: 单一消息循环 + 多订阅者广播
/// 2. **引用计数**: 支持同一频道多次订阅
/// 3. **多 URL 支持**: 根据市场类型自动选择正确的 URL
/// 4. **自动重连恢复**: 断线后自动恢复所有订阅
/// 5. **统一认证**: 支持多种认证模式（Token 预认证、消息认证等）
///
/// # 认证模式
///
/// - `NoAuth`: 无认证（默认）
/// - `TokenProvider`: Token 预认证（Binance, Kraken, KuCoin）
/// - `MessageAuthenticator`: 消息认证（OKX, Bybit, Bitget）
///
/// # 示例
///
/// ```rust,ignore
/// // 定义类型别名（带认证）
/// pub type OkxWsClient = GenericWsClient<
///     OkxSubscriptionBuilder,
///     OkxStreamParser,
///     OkxWsEndpointProvider,
///     OkxWsAuth,  // 实现 MessageAuthenticator
/// >;
///
/// // 使用
/// let auth = OkxWsAuth::new(api_key, passphrase, secret_key);
/// let client = OkxWsClient::new(
///     OkxSubscriptionBuilder,
///     OkxStreamParser,
///     OkxWsEndpointProvider::new(false),
///     auth,
/// );
///
/// // 连接（自动发送登录消息）
/// client.connect_with_login().await?;
///
/// // 订阅
/// let mut ticker_stream = client.watch_ticker("BTC/USDT").await?;
/// while let Some(ticker) = ticker_stream.recv().await {
///     println!("Ticker: {:?}", ticker);
/// }
/// ```
#[derive(Debug)]
pub struct GenericWsClient<B, P, E, A = NoAuth>
where
    B: SubscriptionBuilder,
    P: StreamParser,
    E: WsEndpointProvider,
    A: WsAuthCore,
{
    /// 多连接管理器（按 URL 分组，每个连接携带其市场类型上下文）
    connections: Arc<RwLock<HashMap<String, ConnectionEntry>>>,
    /// 订阅构建器
    subscription_builder: B,
    /// 消息解析器
    parser: P,
    /// 端点提供者
    endpoint_provider: E,
    /// 认证策略
    auth_strategy: A,
    /// 默认 WebSocket 配置
    default_ws_config: WsConfig,
}

impl<B, P, E, A> GenericWsClient<B, P, E, A>
where
    B: SubscriptionBuilder,
    P: StreamParser,
    E: WsEndpointProvider,
    A: WsAuthCore,
{
    /// 创建新的泛型 WebSocket 客户端
    pub fn new(builder: B, parser: P, endpoint_provider: E, auth_strategy: A) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            subscription_builder: builder,
            parser,
            endpoint_provider,
            auth_strategy,
            default_ws_config: WsConfig::default(),
        }
    }

    /// 使用自定义 WebSocket 配置创建
    pub fn with_config(
        builder: B,
        parser: P,
        endpoint_provider: E,
        auth_strategy: A,
        config: WsConfig,
    ) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            subscription_builder: builder,
            parser,
            endpoint_provider,
            auth_strategy,
            default_ws_config: config,
        }
    }

    /// 订阅频道
    ///
    /// # 参数
    ///
    /// - `channels`: 要订阅的频道列表
    ///
    /// # 返回
    ///
    /// 返回订阅结果（不返回接收器，用于简单订阅场景）
    pub async fn subscribe(&self, channels: &[SubscriptionChannel]) -> Result<()> {
        for ch in channels {
            self.subscribe_single(ch).await?;
        }
        Ok(())
    }

    /// 订阅并获取消息接收器
    ///
    /// # 参数
    ///
    /// - `channel`: 要订阅的频道
    ///
    /// # 返回
    ///
    /// 返回消息接收器
    pub async fn subscribe_and_receive(
        &self,
        channel: &SubscriptionChannel,
    ) -> Result<mpsc::Receiver<Value>> {
        self.subscribe_single(channel).await
    }

    /// 订阅单个频道
    ///
    /// 返回消息接收器，可用于接收该频道的消息
    async fn subscribe_single(
        &self,
        channel: &SubscriptionChannel,
    ) -> Result<mpsc::Receiver<Value>> {
        // 1. 确定 WebSocket 上下文
        let context = crate::ws::core::endpoint::WsContext::from_channel(channel);

        // 2. 获取正确的 URL
        let url = self.endpoint_provider.ws_url(&context);

        debug!(
            url = %url,
            channel = %channel.to_key(),
            "Subscribing to channel"
        );

        // 3. 获取或创建连接
        let client = self.get_or_create_connection(&url, &context).await?;

        // 4. 构建订阅消息并确定预期的 channel key
        let msg = self
            .subscription_builder
            .build_subscribe(&[channel.clone()])?;

        // 4.1 从订阅消息中提取预期的频道 key（使用交易所特定的 extract_channel 逻辑）
        // 这确保订阅注册和消息路由使用相同的 key
        let expected_channel_key = self.subscription_builder.extract_channel_from_subscription(
            &channel.channel_type,
            &channel.symbol,
            &channel.params,
        );

        // 🔍 调试日志
        debug!(
            channel_key = %expected_channel_key,
            "Generated expected channel key for subscription"
        );

        // 5. 发送订阅消息
        debug!(url = %url, message = %msg, "Sending subscription message");
        client.send_json(&msg).await?;

        // 6. 注册到底层订阅管理器并获取接收器
        // 使用预期的 channel key 而不是 channel_type.to_string()
        let (channel_name, symbol) = Self::parse_channel_key(&expected_channel_key);
        let receiver = client
            .subscription_manager()
            .add_subscriber(channel_name, symbol, Some(channel.params.clone()))
            .await?;

        info!(
            url = %url,
            channel = %expected_channel_key,
            "Subscription successful"
        );

        Ok(receiver)
    }

    /// 解析 channel key 为 (channel_name, symbol)
    fn parse_channel_key(key: &str) -> (String, Option<String>) {
        if let Some(colon_pos) = key.find(':') {
            let channel = &key[..colon_pos];
            let symbol = &key[colon_pos + 1..];
            (channel.to_string(), Some(symbol.to_string()))
        } else {
            (key.to_string(), None)
        }
    }

    /// 取消订阅
    pub async fn unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<()> {
        for ch in channels {
            self.unsubscribe_single(ch).await?;
        }
        Ok(())
    }

    /// 取消订阅单个频道
    async fn unsubscribe_single(&self, channel: &SubscriptionChannel) -> Result<()> {
        let context = crate::ws::core::endpoint::WsContext::from_channel(channel);
        let url = self.endpoint_provider.ws_url(&context);

        // 从底层订阅管理器移除
        let connections = self.connections.read().await;
        if let Some(entry) = connections.get(&url) {
            // 使用与 subscribe_single 相同的 channel key 提取逻辑，确保匹配
            let expected_channel_key = self.subscription_builder.extract_channel_from_subscription(
                &channel.channel_type,
                &channel.symbol,
                &channel.params,
            );
            let (channel_name, symbol) = Self::parse_channel_key(&expected_channel_key);

            // 🔍 调试日志
            debug!(
                url = %url,
                channel_key = %expected_channel_key,
                channel_name = %channel_name,
                symbol = ?symbol,
                "Unsubscribing with channel key"
            );

            let is_last = entry
                .client
                .subscription_manager()
                .remove_subscriber(&channel_name, symbol.as_ref())
                .await?;

            // 如果是最后一个订阅者，发送取消订阅消息
            if is_last {
                let msg = self
                    .subscription_builder
                    .build_unsubscribe(&[channel.clone()])?;
                entry.client.send_json(&msg).await?;
            }

            info!(
                url = %url,
                channel = %channel.to_key(),
                is_last = is_last,
                "Unsubscription successful"
            );
        }

        Ok(())
    }

    /// 获取或创建指定 URL 的连接
    async fn get_or_create_connection(
        &self,
        url: &str,
        context: &crate::ws::core::endpoint::WsContext,
    ) -> Result<Arc<WsClient>> {
        let market_type = context.market_type;

        // 先尝试读取
        {
            let connections = self.connections.read().await;
            if let Some(entry) = connections.get(url) {
                return Ok(entry.client.clone());
            }
        }

        // 需要创建新连接
        let mut connections = self.connections.write().await;

        // 再次检查（可能有其他任务已创建）
        if let Some(entry) = connections.get(url) {
            return Ok(entry.client.clone());
        }

        // 创建新连接
        let mut config = self.default_ws_config.clone();
        config.url = url.to_string();

        let client = Arc::new(WsClient::new(config));

        // ⭐ 关键：注入交易所特定的订阅/取消订阅构建器
        // 这样重连时可以使用正确的消息格式
        self.inject_subscribe_functions(&client).await;

        client.connect().await?;

        // 启动自动重连协调器 ⭐ 关键：确保连接断开后自动重连并恢复订阅
        // 注意：只在首次创建连接时启动，避免重复启动多个 coordinator
        let reconnect_coordinator = client.clone().create_auto_reconnect_coordinator();
        reconnect_coordinator.start().await;
        debug!(url = %url, "Auto-reconnect coordinator started");

        // 启动消息循环（传递市场类型上下文）
        self.start_message_loop(client.clone(), url.to_string(), market_type)
            .await;

        connections.insert(
            url.to_string(),
            ConnectionEntry {
                client: client.clone(),
                market_type,
                coordinator: Some(reconnect_coordinator),
            },
        );

        info!(url = %url, "WebSocket connection established");

        Ok(client)
    }

    /// 启动消息循环
    ///
    /// 核心逻辑：单一消息循环 + 广播
    /// 优化：支持自动重连后继续接收消息
    ///
    /// 注意：心跳管理已由底层 WsClient 的消息循环处理，
    /// 本方法只负责从 message channel 读取消息并广播到订阅者。
    async fn start_message_loop(
        &self,
        client: Arc<WsClient>,
        url: String,
        market_type: Option<crate::ws::subscription::MarketType>,
    ) {
        let sub_builder = self.subscription_builder.clone();
        let _parser = self.parser.clone();
        let client_clone = client.clone();

        tokio::spawn(async move {
            info!(url = %url, market_type = ?market_type, "Message loop started");

            // 获取状态监听器（事件驱动）
            let mut state_rx = client_clone.state_watch_rx();

            loop {
                // 事件驱动：等待连接状态变为 Connected
                while *state_rx.borrow() != WsConnectionState::Connected {
                    debug!(url = %url, state = ?*state_rx.borrow(), "Waiting for connection...");
                    // 等待状态变化（事件驱动，无忙等）
                    if state_rx.changed().await.is_err() {
                        info!(url = %url, "State watch channel closed, exiting message loop");
                        return;
                    }
                }

                // 接收消息（从 WsClient 的 message channel 读取）
                // 注意：PING/PONG 消息已被 WsClient 过滤，不会到达这里
                match client_clone.receive().await {
                    Some(msg) => {
                        // 提取频道标识（带市场类型上下文，支持现货/合约区分路由）
                        if let Some(channel_key) =
                            sub_builder.extract_channel_with_context(&msg, market_type)
                        {
                            // 🔍 调试日志 - 消息路由
                            debug!(
                                channel_key = %channel_key,
                                "Message received, will broadcast to channel"
                            );

                            // 注入市场类型上下文，帮助下游解析器区分 spot/swap
                            let tagged_msg = Self::tag_message_with_market_type(msg, market_type);

                            // 广播给所有订阅者（使用底层订阅管理器）
                            client_clone
                                .subscription_manager()
                                .broadcast(&channel_key, tagged_msg)
                                .await;
                        } else if sub_builder.is_error(&msg) {
                            // 处理错误
                            if let Some(err_msg) = sub_builder.extract_error(&msg) {
                                error!(
                                    url = %url,
                                    error = %err_msg,
                                    "WebSocket error received"
                                );
                            }
                        }
                        // 注意：PING/PONG 消息已被 WsClient 处理，不会到达这里
                    }
                    None => {
                        warn!(url = %url, "Message channel closed, waiting for reconnection...");
                        // 不 break，等待自动重连后继续
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    }
                }
            }
        });
    }

    /// 连接（初始化连接但不订阅）
    pub async fn connect(&self, context: &crate::ws::core::endpoint::WsContext) -> Result<()> {
        let url = self.endpoint_provider.ws_url(context);
        self.get_or_create_connection(&url, context).await?;
        Ok(())
    }

    /// 断开所有连接（保留订阅状态，暂停自动重连）
    pub async fn disconnect(&self) -> Result<()> {
        let connections = self.connections.read().await;
        for (url, entry) in connections.iter() {
            // 暂停自动重连协调器
            if let Some(coordinator) = &entry.coordinator {
                coordinator.pause().await;
            }

            if let Err(e) = entry.client.disconnect().await {
                warn!(url = %url, error = %e, "Failed to disconnect");
            }
        }
        info!("Manual disconnect completed, subscriptions preserved");
        Ok(())
    }

    /// 检查是否已连接
    pub async fn is_connected(&self, url: &str) -> bool {
        let connections = self.connections.read().await;
        connections
            .get(url)
            .map(|c| c.client.is_connected())
            .unwrap_or(false)
    }

    /// 获取所有连接状态
    pub async fn connection_states(&self) -> HashMap<String, WsConnectionState> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .map(|(url, entry)| (url.clone(), entry.client.state()))
            .collect()
    }

    /// 注入市场类型标记到消息中，帮助下游解析器区分 spot/swap
    ///
    /// 在 JSON 对象中插入 `_ccxt_mt` 字段（值为 "spot"/"swap"/"future"/"option"）。
    /// 非 Object 类型的消息原样返回。
    fn tag_message_with_market_type(msg: Value, market_type: Option<MarketType>) -> Value {
        let mt = match market_type {
            Some(mt) => mt,
            None => return msg,
        };
        match msg {
            Value::Object(mut map) => {
                map.insert("_ccxt_mt".to_string(), Value::String(mt.to_string()));
                Value::Object(map)
            }
            other => other,
        }
    }

    /// 同步获取第一个连接的状态
    ///
    /// 用于实现同步的 `WsExchange::ws_state()` 方法。
    /// 如果没有连接或无法获取锁，返回 `Disconnected`。
    pub fn state_sync(&self) -> WsConnectionState {
        self.connections
            .try_read()
            .ok()
            .and_then(|connections| {
                connections
                    .iter()
                    .next()
                    .map(|(_, entry)| entry.client.state())
            })
            .unwrap_or(WsConnectionState::Disconnected)
    }

    /// 同步检查是否已连接
    ///
    /// 用于实现同步的 `WsExchange::ws_is_connected()` 方法。
    /// 如果没有连接或无法获取锁，返回 `false`。
    pub fn is_connected_sync(&self) -> bool {
        self.state_sync() == WsConnectionState::Connected
    }

    /// 同步获取所有订阅列表
    ///
    /// 用于实现同步的 `WsExchange::subscriptions()` 方法。
    /// 收集所有连接的所有订阅。
    /// 如果无法获取锁，返回空 Vec。
    pub fn subscriptions_sync(&self) -> Vec<String> {
        self.connections
            .try_read()
            .ok()
            .map(|connections| {
                connections
                    .iter()
                    .flat_map(|(_, entry)| entry.client.subscriptions())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 异步获取所有订阅列表
    ///
    /// 收集所有连接的所有订阅。
    pub async fn subscriptions(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .flat_map(|(_, entry)| entry.client.subscriptions())
            .collect()
    }

    /// 获取订阅统计
    pub async fn subscription_stats(
        &self,
    ) -> HashMap<String, crate::network::ws_client::SubscriptionStats> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .map(|(url, entry)| (url.clone(), entry.client.subscription_manager().stats()))
            .collect()
    }

    // ========================================================================
    // Watch Methods (高级订阅 API)
    // ========================================================================
    //
    // 注意：旧的 watch_ticker, watch_orderbook 等方法已删除
    // 使用新的泛型方法 watch<T>() 替代
    // 例如：
    //   client.watch::<Ticker>("BTC/USDT", None).await?;
    //   client.watch::<OrderBook>("BTC/USDT", params).await?;

    // ========================================================================
    // 新设计：泛型 Watch 方法（推荐）
    // ========================================================================

    /// 泛型订阅方法 - 使用新设计
    ///
    /// 这是新的推荐接口，使用泛型方法替代多个具体的 watch_* 方法。
    ///
    /// # Type Parameters
    /// * `T` - 要订阅的数据类型，必须实现 `Parseable`
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号（可选，某些类型如 Balance 不需要）
    /// * `params` - 额外参数（如 OHLCV 的时间间隔）
    ///
    /// # Returns
    /// 返回类型化的消息流 `MessageStream<T>`
    ///
    /// # Example
    /// ```rust,ignore
    /// // 订阅 Ticker
    /// let stream = client.watch::<Ticker>("BTC/USDT", None).await?;
    ///
    /// // 订阅 OrderBook
    /// let stream = client.watch::<OrderBook>("BTC/USDT", Some(vec![
    ///     ("depth".to_string(), json!(10))
    /// ])).await?;
    ///
    /// // 订阅 OHLCV
    /// let stream = client.watch::<Vec<Ohlcv>>("BTC/USDT", Some(vec![
    ///     ("interval".to_string(), json!("1m"))
    /// ])).await?;
    /// ```
    ///
    /// # 迁移指南
    ///
    /// 旧方法                    新方法
    /// --------------------     ---------------------------
    /// watch_ticker(symbol)     watch::<Ticker>(symbol, None)
    /// watch_orderbook(s, l)    watch::<OrderBook>(s, params)
    /// watch_trades(symbol)     watch::<Vec<Trade>>(symbol, None)
    /// watch_ohlcv(s, i)        watch::<Vec<Ohlcv>>(s, params)
    /// watch_balance()          watch::<Balance>("", None)
    /// watch_orders(symbol)     watch::<Order>(symbol, None)
    pub async fn watch<T: Parseable>(
        &self,
        symbol: &str,
        params: Option<Vec<(String, Value)>>,
    ) -> Result<crate::ws_exchange::MessageStream<T>>
    where
        P: StreamParserExt,
    {
        // 根据类型确定频道类型
        let channel = Self::channel_for_type::<T>(symbol, params)?;
        let mut rx = self.subscribe_and_receive(&channel).await?;
        let parser = self.parser.clone();

        Ok(Box::pin(async_stream::stream! {
            while let Some(msg) = rx.recv().await {
                match parser.parse_as::<T>(&msg) {
                    Ok(data) => yield Ok(data),
                    Err(e) => yield Err(e),
                }
            }
        }))
    }

    /// 根据类型创建对应的 SubscriptionChannel
    fn channel_for_type<T: Parseable>(
        symbol: &str,
        params: Option<Vec<(String, Value)>>,
    ) -> Result<SubscriptionChannel> {
        let mut channel = match T::TYPE_NAME {
            "ticker" => SubscriptionChannel::ticker(symbol),
            "orderbook" => SubscriptionChannel::orderbook(symbol),
            "trade" => SubscriptionChannel::trades(symbol),
            "ohlcv" => SubscriptionChannel::kline(symbol, "1m"), // 默认1分钟，可通过params覆盖
            "balance" => SubscriptionChannel::balance(),
            "order" => SubscriptionChannel::orders(Some(symbol.to_string())),
            "mark_price" => SubscriptionChannel::mark_price(symbol),
            "bid_ask" => SubscriptionChannel::bids_asks(symbol),
            "position" => SubscriptionChannel::positions(Some(symbol.to_string())),
            _ => {
                return Err(crate::error::Error::invalid_request(format!(
                    "Unsupported type for watch: {}",
                    T::TYPE_NAME
                )));
            }
        };

        // market_type is already auto-detected by SubscriptionChannel constructors
        // (see detect_market_type in ws::subscription::types)

        // 应用额外参数
        if let Some(params) = params {
            for (key, value) in params {
                channel = channel.with_param(&key, value);
            }
        }

        Ok(channel)
    }

    // ========================================================================
    // 认证相关方法
    // ========================================================================

    /// 获取认证策略引用
    pub fn auth_strategy(&self) -> &A {
        &self.auth_strategy
    }

    /// 检查是否需要认证
    pub fn needs_auth(&self) -> bool {
        self.auth_strategy.needs_auth()
    }

    // ========================================================================
    // 订阅函数注入
    // ========================================================================

    /// 注入交易所特定的订阅/取消订阅构建器
    ///
    /// 这些函数在重连时用于生成正确的交易所特定消息格式
    async fn inject_subscribe_functions(&self, client: &Arc<WsClient>) {
        use crate::network::ws_client::{SubscribeFn, SubscriptionInfo, UnsubscribeFn};

        // 注入订阅函数
        let builder = self.subscription_builder.clone();
        let subscribe_fn: SubscribeFn = Arc::new(move |info: SubscriptionInfo| {
            let builder = builder.clone();
            Box::pin(async move {
                // 使用交易所特定的 rebuild_subscription_channel 重建频道
                let channel = builder.rebuild_subscription_channel(&info)?;

                // 使用交易所特定的 SubscriptionBuilder 构建消息
                builder.build_subscribe(&[channel])
            })
        });

        client.set_subscribe_fn(subscribe_fn).await;

        // 注入取消订阅函数
        let builder = self.subscription_builder.clone();
        let unsubscribe_fn: UnsubscribeFn = Arc::new(move |info: SubscriptionInfo| {
            let builder = builder.clone();
            Box::pin(async move {
                let channel = builder.rebuild_subscription_channel(&info)?;
                builder.build_unsubscribe(&[channel])
            })
        });

        client.set_unsubscribe_fn(unsubscribe_fn).await;

        debug!("Exchange-specific subscribe/unsubscribe functions injected");
    }
}

// ============================================================================
// Token 预认证专用方法
// ============================================================================

impl<B, P, E, A> GenericWsClient<B, P, E, A>
where
    B: SubscriptionBuilder,
    P: StreamParser,
    E: WsEndpointProvider,
    A: TokenProvider,
{
    /// 连接并自动获取 Token（Token 预认证模式）
    ///
    /// # 认证流程
    ///
    /// 1. 通过 REST API 获取 Token
    /// 2. 修改 WebSocket URL 以包含 Token
    /// 3. 建立 WebSocket 连接
    /// 4. 启动 Token 自动续期任务（如果需要）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // Binance 私有频道连接
    /// let auth = BinanceWsAuth::new(binance_client, BinanceWsMarket::Spot);
    /// let client = BinanceWsClientAuth::new(
    ///     BinanceSubscriptionBuilder,
    ///     BinanceStreamParser,
    ///     BinanceWsEndpointProvider::new(false),
    ///     auth,
    /// );
    ///
    /// // 连接（自动获取 listenKey 并续期）
    /// let context = WsContext::new().with_private();
    /// client.connect_with_token(&context).await?;
    ///
    /// // 订阅私有频道
    /// let balance_stream = client.watch_balance().await?;
    /// ```
    pub async fn connect_with_token(
        &self,
        context: &crate::ws::core::endpoint::WsContext,
    ) -> Result<()> {
        // 1. 获取 Token
        let token = self.auth_strategy.get_token().await?;

        // 2. 修改 URL
        let base_url = self.endpoint_provider.ws_url(context);
        let url = self.auth_strategy.modify_url(&base_url, &token);

        debug!(
            original_url = %base_url,
            modified_url = %url,
            "Token pre-auth: URL modified"
        );

        // 3. 创建连接
        self.get_or_create_connection(&url, context).await?;

        // 4. 启动续期任务（如果需要）
        if self.auth_strategy.needs_renewal() {
            self.start_token_renewal_task();
        }

        info!(
            url = %url,
            mode = ?self.auth_strategy.mode(),
            "WebSocket connected with token authentication"
        );

        Ok(())
    }

    /// 启动 Token 自动续期任务
    ///
    /// 在后台定期调用 `renew()` 方法续期 Token
    fn start_token_renewal_task(&self) {
        let auth = self.auth_strategy.clone();
        let interval_secs = auth.renewal_interval_secs();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

            info!(interval_secs = interval_secs, "Token renewal task started");

            loop {
                interval.tick().await;

                match auth.renew().await {
                    Ok(()) => {
                        debug!("Token renewed successfully");
                    }
                    Err(e) => {
                        error!(error = %e, "Token renewal failed");
                    }
                }
            }
        });
    }
}

// ============================================================================
// 消息认证专用方法
// ============================================================================

impl<B, P, E, A> GenericWsClient<B, P, E, A>
where
    B: SubscriptionBuilder,
    P: StreamParser,
    E: WsEndpointProvider,
    A: MessageAuthenticator,
{
    /// 连接并发送登录消息（消息认证模式）
    ///
    /// # 认证流程
    ///
    /// 1. 建立普通 WebSocket 连接
    /// 2. 发送登录消息
    /// 3. 等待登录确认
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // OKX 私有频道连接
    /// let auth = OkxWsAuth::new(api_key, passphrase, secret_key);
    /// let client = OkxWsClientAuth::new(
    ///     OkxSubscriptionBuilder,
    ///     OkxStreamParser,
    ///     OkxWsEndpointProvider::new(false),
    ///     auth,
    /// );
    ///
    /// // 连接（自动发送登录消息）
    /// let context = WsContext::new().with_private();
    /// client.connect_with_login(&context).await?;
    ///
    /// // 订阅私有频道
    /// let balance_stream = client.watch_balance().await?;
    /// ```
    pub async fn connect_with_login(
        &self,
        context: &crate::ws::core::endpoint::WsContext,
    ) -> Result<()> {
        // 1. 创建普通连接
        let url = self.endpoint_provider.ws_url(context);
        let client = self.get_or_create_connection(&url, context).await?;

        // 2. 发送登录消息
        if let Some(login_msg) = self.auth_strategy.login_message() {
            debug!(
                url = %url,
                login_msg = ?login_msg,
                "Sending login message"
            );

            client.send_json(&login_msg).await?;

            // 3. 等待登录确认
            self.wait_login_confirm(&client).await?;
        }

        info!(
            url = %url,
            mode = ?self.auth_strategy.mode(),
            "WebSocket connected with message authentication"
        );

        Ok(())
    }

    /// 等待登录确认消息
    ///
    /// 轮询接收消息，检查是否为登录成功/失败响应
    async fn wait_login_confirm(&self, client: &Arc<WsClient>) -> Result<()> {
        let timeout_ms = self.auth_strategy.login_timeout_ms();
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        debug!(timeout_ms = timeout_ms, "Waiting for login confirmation");

        while start.elapsed() < timeout {
            // 尝试接收消息（带超时）
            match tokio::time::timeout(std::time::Duration::from_millis(100), client.receive())
                .await
            {
                Ok(Some(msg)) => {
                    // 检查是否为登录成功
                    if self.auth_strategy.is_success(&msg) {
                        info!("Login successful");
                        return Ok(());
                    }

                    // 检查是否为登录失败
                    if self.auth_strategy.is_failure(&msg) {
                        let error_msg = msg
                            .get("msg")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Login failed")
                            .to_string();
                        error!(error = %error_msg, "Login failed");
                        return Err(crate::error::Error::authentication(error_msg));
                    }

                    // 其他消息，继续等待
                    debug!("Received non-login message, continuing to wait");
                }
                Ok(None) => {
                    // 连接已关闭
                    return Err(crate::error::Error::network(
                        "Connection closed during login",
                    ));
                }
                Err(_) => {
                    // 超时，继续循环检查总超时
                }
            }
        }

        // 总超时
        error!(timeout_ms = timeout_ms, "Login confirmation timeout");
        Err(crate::error::Error::timeout("Login confirmation timeout"))
    }
}

// ============================================================================
// OrderBook 增量管理支持
// ============================================================================

impl<B, P, E, A> GenericWsClient<B, P, E, A>
where
    B: SubscriptionBuilder,
    P: StreamParser + OrderBookDeltaParser,
    E: WsEndpointProvider,
    A: WsAuthCore,
{
    /// 创建 OrderBookManager 用于管理增量更新
    ///
    /// 此方法返回 OrderBookManager，可以与 watch_order_book 配合使用：
    /// - 先调用此方法创建 manager
    /// - 然后调用 watch_order_book 订阅流
    /// - 在消息循环中使用 manager.process_message 处理消息
    ///
    /// # Arguments
    /// * `max_depth` - 可选的最大深度限制
    ///
    /// # Returns
    /// 新创建的 OrderBookManager
    ///
    /// # Example
    /// ```rust,ignore
    /// // 创建 manager
    /// let manager = client.create_orderbook_manager(Some(1000));
    ///
    /// // 订阅 OrderBook 流（增量流）
    /// let mut stream = client.watch_order_book("BTC/USDT").await?;
    ///
    /// // 处理消息
    /// while let Some(msg) = stream.next().await {
    ///     let orderbook = manager.process_message(&symbol, &msg, &parser, false).await?;
    ///     println!("Best bid: {:?}", orderbook.best_bid());
    /// }
    /// ```
    pub fn create_orderbook_manager(&self, max_depth: Option<usize>) -> OrderBookManager {
        match max_depth {
            Some(depth) => OrderBookManager::with_max_depth(depth),
            None => OrderBookManager::new(),
        }
    }

    /// 获取用于增量解析的解析器引用
    ///
    /// 此方法返回 &P，可以用于 OrderBookManager.process_message
    pub fn delta_parser(&self) -> &P {
        &self.parser
    }

    /// 手动重新连接（自动恢复订阅）
    ///
    /// 使用场景：
    /// - 在 `disconnect()` 后重新连接
    /// - 网络恢复后重新建立连接
    ///
    /// 特点：
    /// - 自动恢复所有订阅（使用注入的 subscribe_fn）
    /// - 恢复自动重连协调器
    pub async fn reconnect(&self) -> Result<()> {
        let connections = self.connections.read().await;

        for (_url, entry) in connections.iter() {
            // 1. 重新建立连接（内部会调用 resubscribe_all）
            entry.client.connect().await?;

            // 2. 恢复自动重连协调器
            if let Some(coordinator) = &entry.coordinator {
                coordinator.resume().await;
            }
        }

        info!("Manual reconnect completed, subscriptions restored");
        Ok(())
    }
}

impl<B, P, E, A> Clone for GenericWsClient<B, P, E, A>
where
    B: SubscriptionBuilder + Clone,
    P: StreamParser + Clone,
    E: WsEndpointProvider + Clone,
    A: WsAuthCore + Clone,
{
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            subscription_builder: self.subscription_builder.clone(),
            parser: self.parser.clone(),
            endpoint_provider: self.endpoint_provider.clone(),
            auth_strategy: self.auth_strategy.clone(),
            default_ws_config: self.default_ws_config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::types::Ticker;
    use crate::ws::parser::ParsedMessage;
    use serde_json::Value;

    #[derive(Clone)]
    struct TestBuilder;

    impl SubscriptionBuilder for TestBuilder {
        fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
            Ok(serde_json::json!({
                "op": "subscribe",
                "channels": channels.iter().map(|c| c.to_key()).collect::<Vec<_>>()
            }))
        }

        fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
            Ok(serde_json::json!({
                "op": "unsubscribe",
                "channels": channels.iter().map(|c| c.to_key()).collect::<Vec<_>>()
            }))
        }

        fn extract_channel(&self, msg: &Value) -> Option<String> {
            msg.get("channel")?.as_str().map(|s| s.to_string())
        }

        fn extract_channel_from_subscription(
            &self,
            channel_type: &crate::ws::subscription::ChannelType,
            symbol: &str,
            _params: &std::collections::HashMap<String, Value>,
        ) -> String {
            format!("{}:{}", channel_type, symbol)
        }

        fn rebuild_subscription_channel(
            &self,
            info: &crate::network::ws_client::subscription::SubscriptionInfo,
        ) -> Result<SubscriptionChannel> {
            // 简单实现
            Ok(SubscriptionChannel {
                channel_type: crate::ws::subscription::ChannelType::Ticker,
                symbol: info.symbol.clone().unwrap_or_default(),
                params: info.params.clone(),
                market_type: None,
                is_private: false,
            })
        }
    }

    #[derive(Clone)]
    struct TestParser;

    impl StreamParser for TestParser {
        fn parse(&self, _msg: &Value) -> Result<ParsedMessage> {
            Ok(ParsedMessage::Heartbeat)
        }
    }

    impl StreamParserExt for TestParser {
        fn parse_as<T: Parseable>(&self, _msg: &Value) -> Result<T> {
            use std::any::TypeId;

            // 根据类型返回测试数据
            if TypeId::of::<T>() == TypeId::of::<Ticker>() {
                let ticker = Ticker::default();
                Ok(unsafe { std::mem::transmute_copy(&ticker) })
            } else if TypeId::of::<T>() == TypeId::of::<crate::types::OrderBook>() {
                use crate::types::{Symbol, Timestamp};
                let orderbook = crate::types::OrderBook::new(
                    Symbol::new_unchecked("BTC/USDT"),
                    Timestamp::default(),
                );
                Ok(unsafe { std::mem::transmute_copy(&orderbook) })
            } else if TypeId::of::<T>() == TypeId::of::<Vec<crate::types::Trade>>() {
                let trades: Vec<crate::types::Trade> = vec![];
                Ok(unsafe { std::mem::transmute_copy(&trades) })
            } else if TypeId::of::<T>() == TypeId::of::<Vec<crate::types::Ohlcv>>() {
                use crate::types::{Amount, Price, Timestamp};
                use rust_decimal::Decimal;
                let ohlcv = vec![crate::types::Ohlcv::new(
                    Timestamp::default(),
                    Price::new(Decimal::ZERO),
                    Price::new(Decimal::ZERO),
                    Price::new(Decimal::ZERO),
                    Price::new(Decimal::ZERO),
                    Amount::new(Decimal::ZERO),
                )];
                Ok(unsafe { std::mem::transmute_copy(&ohlcv) })
            } else if TypeId::of::<T>() == TypeId::of::<crate::types::Balance>() {
                let balance = crate::types::Balance::default();
                Ok(unsafe { std::mem::transmute_copy(&balance) })
            } else if TypeId::of::<T>() == TypeId::of::<crate::types::Order>() {
                Err(Error::not_implemented("parse_as::<Order>"))
            } else {
                Err(Error::invalid_request(format!(
                    "TestParser does not support type: {}",
                    T::TYPE_NAME
                )))
            }
        }
    }

    #[derive(Clone)]
    struct TestEndpoint {
        is_sandbox: bool,
    }

    impl WsEndpointProvider for TestEndpoint {
        fn ws_public_url(&self, _context: &crate::ws::core::endpoint::WsContext) -> String {
            if self.is_sandbox {
                "wss://test.example.com/ws".to_string()
            } else {
                "wss://api.example.com/ws".to_string()
            }
        }

        fn ws_private_url(&self, _context: &crate::ws::core::endpoint::WsContext) -> String {
            if self.is_sandbox {
                "wss://test.example.com/private".to_string()
            } else {
                "wss://api.example.com/private".to_string()
            }
        }
    }

    #[tokio::test]
    async fn test_generic_ws_client_creation() {
        let client = GenericWsClient::new(
            TestBuilder,
            TestParser,
            TestEndpoint { is_sandbox: false },
            NoAuth,
        );
        assert!(client.connections.read().await.is_empty());
    }
}

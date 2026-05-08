//! Trading trait definition.
//!
//! The `Trading` trait provides methods for order management operations
//! including creating, canceling, and fetching orders. These operations
//! require authentication.
//!
//! # Object Safety
//!
//! This trait is designed to be object-safe, allowing for dynamic dispatch via
//! trait objects (`dyn Trading`).
//!
//! # Example
//!
//! ```rust,ignore
//! use ccxt_core::traits::Trading;
//! use ccxt_core::types::order::order_request::OrderRequestBuilder;
//! use rust_decimal_macros::dec;
//!
//! async fn place_order(exchange: &dyn Trading) -> Result<(), ccxt_core::Error> {
//!     // Market buy
//!     let order = exchange.market_buy("BTC/USDT", dec!(0.01)).await?;
//!     
//!     // Limit sell with options
//!     let request = OrderRequestBuilder::limit_sell("BTC/USDT", dec!(0.01), dec!(50000))
//!         .time_in_force(ccxt_core::types::TimeInForce::IOC)
//!         .client_order_id("my-order-123".to_string())
//!         .build()?;
//!     let order = exchange.create_order(request).await?;
//!     
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;

use crate::Trade;
use crate::error::Result;
use crate::traits::PublicExchange;
use crate::types::{Order, OrderRequest};

/// Trait for order management operations.
///
/// This trait provides methods for creating, canceling, and fetching orders.
/// All methods require authentication and are async.
///
/// # Supertrait
///
/// Requires `PublicExchange` as a supertrait to access exchange metadata
/// and capabilities.
///
/// # Thread Safety
///
/// This trait requires `Send + Sync` bounds (inherited from `PublicExchange`)
/// to ensure safe usage across thread boundaries in async contexts.
#[async_trait]
pub trait Trading: PublicExchange {
    // ========================================================================
    // Order Creation
    // ========================================================================

    /// Create a new order using `OrderRequest`.
    ///
    /// This is the primary method for creating orders. Use the `OrderRequestBuilder`
    /// for ergonomic order construction with compile-time type safety.
    ///
    /// # Arguments
    ///
    /// * `request` - Order request built by `OrderRequestBuilder`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ccxt_core::types::order::order_request::OrderRequestBuilder;
    /// use rust_decimal_macros::dec;
    ///
    /// // Market buy
    /// let request = OrderRequestBuilder::market_buy("BTC/USDT", dec!(0.01))
    ///     .build()?;
    /// let order = exchange.create_order(request).await?;
    ///
    /// // Limit sell with custom options
    /// let request = OrderRequestBuilder::limit_sell("BTC/USDT", dec!(0.01), dec!(50000))
    ///     .time_in_force(TimeInForce::IOC)
    ///     .client_order_id("my-order-123".to_string())
    ///     .build()?;
    /// let order = exchange.create_order(request).await?;
    /// ```
    async fn create_order(&self, request: OrderRequest) -> Result<Order>;

    /// Convenience method for market buy order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USDT")
    /// * `amount` - Order amount (base currency)
    async fn market_buy(&self, symbol: &str, amount: rust_decimal::Decimal) -> Result<Order> {
        let request = OrderRequest::builder()
            .symbol(symbol)
            .side(crate::types::OrderSide::Buy)
            .order_type(crate::types::OrderType::Market)
            .amount(amount)
            .build()
            .map_err(|e| crate::error::Error::InvalidOrder(e.to_string().into()))?;
        self.create_order(request).await
    }

    /// Convenience method for market sell order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `amount` - Order amount (base currency)
    async fn market_sell(&self, symbol: &str, amount: rust_decimal::Decimal) -> Result<Order> {
        let request = OrderRequest::builder()
            .symbol(symbol)
            .side(crate::types::OrderSide::Sell)
            .order_type(crate::types::OrderType::Market)
            .amount(amount)
            .build()
            .map_err(|e| crate::error::Error::InvalidOrder(e.to_string().into()))?;
        self.create_order(request).await
    }

    /// Convenience method for limit buy order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `amount` - Order amount (base currency)
    /// * `price` - Limit price
    async fn limit_buy(
        &self,
        symbol: &str,
        amount: rust_decimal::Decimal,
        price: rust_decimal::Decimal,
    ) -> Result<Order> {
        let request = OrderRequest::builder()
            .symbol(symbol)
            .side(crate::types::OrderSide::Buy)
            .order_type(crate::types::OrderType::Limit)
            .amount(amount)
            .price(crate::types::Price::new(price))
            .time_in_force(crate::types::TimeInForce::GTC)
            .build()
            .map_err(|e| crate::error::Error::InvalidOrder(e.to_string().into()))?;
        self.create_order(request).await
    }

    /// Convenience method for limit sell order.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `amount` - Order amount (base currency)
    /// * `price` - Limit price
    async fn limit_sell(
        &self,
        symbol: &str,
        amount: rust_decimal::Decimal,
        price: rust_decimal::Decimal,
    ) -> Result<Order> {
        let request = OrderRequest::builder()
            .symbol(symbol)
            .side(crate::types::OrderSide::Sell)
            .order_type(crate::types::OrderType::Limit)
            .amount(amount)
            .price(crate::types::Price::new(price))
            .time_in_force(crate::types::TimeInForce::GTC)
            .build()
            .map_err(|e| crate::error::Error::InvalidOrder(e.to_string().into()))?;
        self.create_order(request).await
    }

    // ========================================================================
    // Order Cancellation
    // ========================================================================

    /// Cancel an existing order.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID to cancel
    /// * `symbol` - Trading pair symbol (required for most exchanges)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cancelled = exchange.cancel_order("12345", "BTC/USDT").await?;
    /// println!("Cancelled order: {}", cancelled.id);
    /// ```
    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order>;

    /// Cancel all orders for a symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    ///
    /// # Returns
    ///
    /// Returns a vector of cancelled orders.
    async fn cancel_all_orders(&self, symbol: &str) -> Result<Vec<Order>>;

    // ========================================================================
    // Order Queries
    // ========================================================================

    /// Fetch a specific order by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Order ID
    /// * `symbol` - Trading pair symbol (required for most exchanges)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let order = exchange.fetch_order("12345", "BTC/USDT").await?;
    /// println!("Order status: {:?}", order.status);
    /// ```
    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order>;

    /// Fetch open orders.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol. If `None`, returns all open orders.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // All open orders
    /// let orders = exchange.fetch_open_orders(None).await?;
    ///
    /// // Open orders for specific symbol
    /// let orders = exchange.fetch_open_orders(Some("BTC/USDT")).await?;
    /// ```
    async fn fetch_open_orders(&self, symbol: Option<&str>) -> Result<Vec<Order>>;

    /// Fetch closed orders with pagination.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Optional trading pair symbol
    /// * `since` - Optional start timestamp in milliseconds (i64) since Unix epoch
    /// * `limit` - Optional maximum number of orders to return
    ///
    /// # Timestamp Format
    ///
    /// The `since` parameter uses `i64` milliseconds since Unix epoch:
    /// - `1609459200000` = January 1, 2021, 00:00:00 UTC
    /// - `chrono::Utc::now().timestamp_millis()` = Current time
    /// - `chrono::Utc::now().timestamp_millis() - 86400000` = 24 hours ago
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Recent closed orders
    /// let orders = exchange.fetch_history_orders(Some("BTC/USDT"), None, Some(100)).await?;
    ///
    /// // Closed orders since timestamp
    /// let orders = exchange.fetch_history_orders(
    ///     Some("BTC/USDT"),
    ///     Some(1609459200000i64),
    ///     Some(50)
    /// ).await?;
    /// ```
    async fn fetch_history_orders(
        &self,
        symbol: Option<&str>,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Order>>;

    // ========================================================================
    // Trades
    // ========================================================================

    /// Fetch recent public trades.
    ///
    /// Returns recent trades for the specified symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let trades = exchange.fetch_trades("BTC/USDT").await?;
    /// for trade in trades {
    ///     println!("{}: {} @ {}", trade.side, trade.amount, trade.price);
    /// }
    /// ```
    async fn fetch_trades(&self, symbol: &str) -> Result<Vec<Trade>> {
        self.fetch_trades_with_limit(symbol, None, None).await
    }

    /// Fetch recent trades with limit only.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol
    /// * `limit` - Maximum number of trades to return
    async fn fetch_trades_with_limit_only(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>> {
        self.fetch_trades_with_limit(symbol, None, limit).await
    }

    /// Fetch recent trades with limit and optional timestamp filtering.
    ///
    /// Returns recent trades for the specified symbol, optionally filtered by timestamp.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The trading pair symbol (e.g., "BTC/USDT")
    /// * `since` - Optional start timestamp in milliseconds (i64) since Unix epoch
    /// * `limit` - Maximum number of trades to return
    ///
    /// # Timestamp Format
    ///
    /// The `since` parameter uses `i64` milliseconds since Unix epoch:
    /// - `1609459200000` = January 1, 2021, 00:00:00 UTC
    /// - `chrono::Utc::now().timestamp_millis()` = Current time
    /// - `chrono::Utc::now().timestamp_millis() - 3600000` = 1 hour ago
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Fetch recent trades without timestamp filter
    /// let trades = exchange.fetch_trades_with_limit("BTC/USDT", None, Some(100)).await?;
    ///
    /// // Fetch trades from the last hour
    /// let since = chrono::Utc::now().timestamp_millis() - 3600000;
    /// let trades = exchange.fetch_trades_with_limit("BTC/USDT", Some(since), Some(50)).await?;
    /// ```
    async fn fetch_trades_with_limit(
        &self,
        symbol: &str,
        since: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<Trade>>;
    // ========================================================================
    // Deprecated u64 Wrapper Methods (Backward Compatibility)
    // ========================================================================
}

/// Type alias for boxed Trading trait object.
pub type BoxedTrading = Box<dyn Trading>;

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;
    use crate::capability::ExchangeCapabilities;
    use crate::types::{OrderSide, OrderStatus, OrderType, Symbol, Timeframe};

    // Mock implementation for testing trait object safety
    struct MockExchange;

    impl PublicExchange for MockExchange {
        fn id(&self) -> &str {
            "mock"
        }
        fn name(&self) -> &str {
            "Mock Exchange"
        }
        fn capabilities(&self) -> ExchangeCapabilities {
            ExchangeCapabilities::all()
        }
        fn timeframes(&self) -> &'static [Timeframe] {
            &[Timeframe::H1]
        }
    }

    #[async_trait]
    impl Trading for MockExchange {
        async fn create_order(&self, request: OrderRequest) -> Result<Order> {
            Ok(Order::new(
                "test-order-123".to_string(),
                Symbol::new_unchecked(request.symbol),
                request.order_type,
                request.side,
                request.amount.as_decimal(),
                request.price.map(|p| p.as_decimal()),
                OrderStatus::Open,
            ))
        }

        async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
            Ok(Order::new(
                id.to_string(),
                Symbol::new_unchecked(symbol),
                OrderType::Limit,
                OrderSide::Buy,
                Decimal::ZERO,
                None,
                OrderStatus::Cancelled,
            ))
        }

        async fn cancel_all_orders(&self, _symbol: &str) -> Result<Vec<Order>> {
            Ok(vec![])
        }

        async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
            Ok(Order::new(
                id.to_string(),
                Symbol::new_unchecked(symbol),
                OrderType::Limit,
                OrderSide::Buy,
                Decimal::ZERO,
                None,
                OrderStatus::Open,
            ))
        }

        async fn fetch_open_orders(&self, _symbol: Option<&str>) -> Result<Vec<Order>> {
            Ok(vec![])
        }

        async fn fetch_history_orders(
            &self,
            _symbol: Option<&str>,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Order>> {
            Ok(vec![])
        }

        async fn fetch_trades_with_limit(
            &self,
            _symbol: &str,
            _since: Option<i64>,
            _limit: Option<u32>,
        ) -> Result<Vec<Trade>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_trait_object_safety() {
        // Verify trait is object-safe by creating a trait object
        let _exchange: BoxedTrading = Box::new(MockExchange);
    }

    #[tokio::test]
    async fn test_create_order() {
        use rust_decimal_macros::dec;

        let exchange = MockExchange;

        let request = OrderRequest::builder()
            .symbol("BTC/USDT")
            .side(OrderSide::Buy)
            .order_type(OrderType::Market)
            .amount(dec!(0.01))
            .build()
            .expect("Should build successfully");

        let order = exchange.create_order(request).await.unwrap();

        assert_eq!(order.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Market);
    }

    #[tokio::test]
    async fn test_convenience_methods() {
        use rust_decimal_macros::dec;

        let exchange = MockExchange;

        // Test market_buy
        let order = exchange.market_buy("BTC/USDT", dec!(0.01)).await.unwrap();
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Market);

        // Test market_sell
        let order = exchange.market_sell("BTC/USDT", dec!(0.01)).await.unwrap();
        assert_eq!(order.side, OrderSide::Sell);
        assert_eq!(order.order_type, OrderType::Market);

        // Test limit_buy
        let order = exchange
            .limit_buy("BTC/USDT", dec!(0.01), dec!(50000))
            .await
            .unwrap();
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);

        // Test limit_sell
        let order = exchange
            .limit_sell("BTC/USDT", dec!(0.01), dec!(50000))
            .await
            .unwrap();
        // Test fetch_trades default
        let trades = exchange.fetch_trades("BTC/USDT").await.unwrap();
        assert!(trades.is_empty());

        assert_eq!(order.side, OrderSide::Sell);
        assert_eq!(order.order_type, OrderType::Limit);
    }

    #[tokio::test]
    async fn test_cancel_order() {
        let exchange = MockExchange;

        let order = exchange.cancel_order("12345", "BTC/USDT").await.unwrap();
        assert_eq!(order.id, "12345");
        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_fetch_orders() {
        let exchange = MockExchange;

        // Test fetch_order
        let order = exchange.fetch_order("12345", "BTC/USDT").await.unwrap();
        assert_eq!(order.id, "12345");

        // Test fetch_open_orders
        let orders = exchange.fetch_open_orders(Some("BTC/USDT")).await.unwrap();
        assert!(orders.is_empty());

        // Test FETCH_HISTORY_ORDERS
        let orders = exchange
            .fetch_history_orders(Some("BTC/USDT"), None, Some(100))
            .await
            .unwrap();
        assert!(orders.is_empty());
    }
}

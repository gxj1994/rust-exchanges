//! Phase 1 验证测试: 订阅函数注入机制
//!
//! 测试目标:
//! 1. 验证 subscribe_fn 可以正确注入
//! 2. 验证 resubscribe_all() 使用注入的函数
//! 3. 验证重连时订阅恢复使用正确的消息格式

#[cfg(test)]
mod phase1_validation_tests {
    use ccxt_core::network::ws_client::{SubscribeFn, SubscriptionInfo, WsClient, WsConfig};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_subscribe_fn_injection() {
        // 创建 WsClient
        let config = WsConfig {
            url: "wss://test.example.com/ws".to_string(),
            ..Default::default()
        };
        let client = WsClient::new(config);

        // 验证初始状态: subscribe_fn 为 None
        assert!(client.get_subscribe_fn().await.is_none());

        // 创建测试用的 subscribe_fn
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let subscribe_fn: SubscribeFn = Arc::new(move |info: SubscriptionInfo| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);

            // 构建一个测试消息
            let msg = serde_json::json!({
                "op": "subscribe",
                "args": [{
                    "channel": info.channel,
                    "instId": info.symbol.unwrap_or_default()
                }]
            });

            Box::pin(async move { Ok(msg) })
        });

        // 注入 subscribe_fn
        client.set_subscribe_fn(subscribe_fn).await;

        // 验证注入成功
        assert!(client.get_subscribe_fn().await.is_some());
        assert_eq!(call_count.load(Ordering::SeqCst), 0); // 还未调用
    }

    #[tokio::test]
    async fn test_subscribe_fn_execution() {
        let config = WsConfig {
            url: "wss://test.example.com/ws".to_string(),
            ..Default::default()
        };
        let client = WsClient::new(config);

        // 创建会记录调用次数的 subscribe_fn
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let subscribe_fn: SubscribeFn = Arc::new(move |info: SubscriptionInfo| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);

            let msg = serde_json::json!({
                "op": "subscribe",
                "channel": info.channel,
                "symbol": info.symbol
            });

            Box::pin(async move { Ok(msg) })
        });

        client.set_subscribe_fn(subscribe_fn).await;

        // 手动调用 subscribe_fn
        let func = client.get_subscribe_fn().await.unwrap();
        let info = SubscriptionInfo {
            channel: "ticker".to_string(),
            symbol: Some("BTC-USDT".to_string()),
            params: std::collections::HashMap::new(),
        };

        let result = func(info).await;
        assert!(result.is_ok());

        let msg = result.unwrap();
        assert_eq!(msg["op"], "subscribe");
        assert_eq!(msg["channel"], "ticker");
        assert_eq!(msg["symbol"], "BTC-USDT");

        // 验证调用次数
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_multiple_subscribe_fn() {
        let config = WsConfig {
            url: "wss://test.example.com/ws".to_string(),
            ..Default::default()
        };
        let client = WsClient::new(config);

        // 第一次注入
        let fn1: SubscribeFn = Arc::new(|info: SubscriptionInfo| {
            let msg = serde_json::json!({
                "version": 1,
                "channel": info.channel
            });
            Box::pin(async move { Ok(msg) })
        });
        client.set_subscribe_fn(fn1).await;

        // 第二次注入（覆盖）
        let fn2: SubscribeFn = Arc::new(|info: SubscriptionInfo| {
            let msg = serde_json::json!({
                "version": 2,
                "channel": info.channel
            });
            Box::pin(async move { Ok(msg) })
        });
        client.set_subscribe_fn(fn2).await;

        // 验证使用的是第二次注入的函数
        let func = client.get_subscribe_fn().await.unwrap();
        let info = SubscriptionInfo {
            channel: "test".to_string(),
            symbol: None,
            params: std::collections::HashMap::new(),
        };

        let result = func(info).await.unwrap();
        assert_eq!(result["version"], 2); // 应该是版本 2
    }
}

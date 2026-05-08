/// 验证取消订阅后消息流正确终止的辅助函数
/// 
/// # 参数
/// 
/// - `stream`: 消息流
/// - `stream_name`: 流名称(用于错误提示)
/// - `buffer_duration`: 缓冲时间(允许取消订阅后短时间内收到缓冲区消息)
/// 
/// # 行为
/// 
/// 1. 记录当前时间为取消订阅时间
/// 2. 清空缓冲区中的残留消息(100ms超时)
/// 3. 如果在缓冲时间后还收到新消息,则 panic
/// 4. 如果通道关闭或超时,则验证通过
#[allow(dead_code)]
async fn verify_stream_terminated_after_unsubscribe<T: std::fmt::Debug>(
    stream: &mut tokio_stream::wrappers::ReceiverStream<Result<T, ccxt_core::error::Error>>,
    stream_name: &str,
    buffer_duration: std::time::Duration,
) {
    use tokio_stream::StreamExt;
    
    println!("[TEST] Verifying {} stream terminates after unsubscribe...", stream_name);
    
    // 记录取消订阅的时间
    let unsubscribe_time = std::time::Instant::now();
    
    // 清空缓冲区中的残留消息,并检查时间戳
    let mut drained_count = 0;
    
    loop {
        match tokio::time::timeout(Duration::from_millis(100), stream.next()).await {
            Ok(Some(Ok(data))) => {
                drained_count += 1;
                // 检查消息是否在取消订阅之后
                let elapsed = unsubscribe_time.elapsed();
                if elapsed > buffer_duration {
                    // 超过缓冲时间还收到消息,说明取消订阅失败
                    panic!(
                        "❌ FAIL: {} received NEW message {}ms after unsubscribe (buffer: {}ms), data={:?}",
                        stream_name,
                        elapsed.as_millis(),
                        buffer_duration.as_millis(),
                        data
                    );
                }
                // 继续清空缓冲区
            }
            Ok(Some(Err(e))) => {
                panic!("❌ FAIL: {} returned error after unsubscribe: {}", stream_name, e);
            }
            Ok(None) | Err(_) => {
                // 通道关闭或超时,缓冲区已清空
                break;
            }
        }
    }
    
    if drained_count > 0 {
        println!("[TEST] ⚠ Drained {} buffered messages from {} after unsubscribe", drained_count, stream_name);
    }
    
    println!("[TEST] ✓ {} stream correctly terminated within buffer period", stream_name);
}

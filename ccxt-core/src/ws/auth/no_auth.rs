//! 无认证策略
//!
//! 用于不需要认证的公共 WebSocket 连接。

use super::{AuthMode, WsAuthCore};

/// 无认证标记
///
/// 用于不需要认证的公共 WebSocket 连接。
/// 作为 `GenericWsClient` 的默认认证策略。
///
/// # 示例
///
/// ```rust,ignore
/// // 使用默认无认证
/// let client = GenericWsClient::new(
///     OkxSubscriptionBuilder,
///     OkxStreamParser,
///     OkxWsEndpointProvider,
///     NoAuth,  // 或使用默认值
/// );
///
/// // 等价于
/// let client: GenericWsClient<_, _, _, NoAuth> = GenericWsClient::new(
///     OkxSubscriptionBuilder,
///     OkxStreamParser,
///     OkxWsEndpointProvider,
///     NoAuth,
/// );
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NoAuth;

impl WsAuthCore for NoAuth {
    fn mode(&self) -> AuthMode {
        AuthMode::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_auth_mode() {
        let auth = NoAuth;
        assert_eq!(auth.mode(), AuthMode::None);
        assert!(!auth.needs_auth());
    }

    #[test]
    fn test_no_auth_clone() {
        let auth = NoAuth;
        let auth2 = auth.clone();
        assert_eq!(auth2.mode(), AuthMode::None);
    }
}

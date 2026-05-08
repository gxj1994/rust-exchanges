//! HyperLiquid error handling module.
//!
//! Provides error types and parsing for HyperLiquid API responses.

use ccxt_core::Error;
use serde_json::Value;

/// HyperLiquid-specific error codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperLiquidErrorCode {
    /// Invalid signature
    InvalidSignature,
    /// Insufficient margin/balance
    InsufficientMargin,
    /// Order not found
    OrderNotFound,
    /// Invalid parameter
    InvalidParameter,
    /// Rate limited
    RateLimited,
    /// Server error
    ServerError,
    /// User not found
    UserNotFound,
    /// Invalid asset
    InvalidAsset,
    /// Position not found
    PositionNotFound,
    /// Order would cross
    OrderWouldCross,
    /// Reduce only violation
    ReduceOnlyViolation,
    /// Unknown error
    Unknown(String),
}

impl std::fmt::Display for HyperLiquidErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InsufficientMargin => write!(f, "Insufficient margin"),
            Self::OrderNotFound => write!(f, "Order not found"),
            Self::InvalidParameter => write!(f, "Invalid parameter"),
            Self::RateLimited => write!(f, "Rate limited"),
            Self::ServerError => write!(f, "Server error"),
            Self::UserNotFound => write!(f, "User not found"),
            Self::InvalidAsset => write!(f, "Invalid asset"),
            Self::PositionNotFound => write!(f, "Position not found"),
            Self::OrderWouldCross => write!(f, "Order would cross"),
            Self::ReduceOnlyViolation => write!(f, "Reduce only violation"),
            Self::Unknown(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<HyperLiquidErrorCode> for Error {
    fn from(code: HyperLiquidErrorCode) -> Self {
        match code {
            HyperLiquidErrorCode::InvalidSignature => Error::authentication("Invalid signature"),
            HyperLiquidErrorCode::InsufficientMargin => {
                Error::insufficient_balance("Insufficient margin")
            }
            HyperLiquidErrorCode::OrderNotFound => Error::invalid_request("Order not found"),
            HyperLiquidErrorCode::InvalidParameter => Error::invalid_request("Invalid parameter"),
            HyperLiquidErrorCode::RateLimited => Error::rate_limit("Rate limited", None),
            HyperLiquidErrorCode::ServerError => Error::exchange("-1", "Server error"),
            HyperLiquidErrorCode::UserNotFound => Error::authentication("User not found"),
            HyperLiquidErrorCode::InvalidAsset => Error::bad_symbol("Invalid asset"),
            HyperLiquidErrorCode::PositionNotFound => Error::invalid_request("Position not found"),
            HyperLiquidErrorCode::OrderWouldCross => Error::invalid_request("Order would cross"),
            HyperLiquidErrorCode::ReduceOnlyViolation => {
                Error::invalid_request("Reduce only violation")
            }
            HyperLiquidErrorCode::Unknown(msg) => Error::exchange("-1", &msg),
        }
    }
}

/// Checks if a response is an error response.
///
/// HyperLiquid returns errors in various formats:
/// - `{"error": "message"}` - Top-level error
/// - `{"status": "err", "response": "message"}` - Status-based error
/// - `{"response": {"data": {"statuses": [{"error": "message"}]}}}` - Order status error
///
/// # Arguments
///
/// * `response` - The JSON response to check.
///
/// # Returns
///
/// `true` if the response indicates an error.
pub fn is_error_response(response: &Value) -> bool {
    // Check for explicit error field
    if response.get("error").is_some() {
        return true;
    }

    // Check for status: err
    if let Some(status) = response.get("status") {
        if status.as_str() == Some("err") {
            return true;
        }
    }

    // Check for order status errors: {"response": {"data": {"statuses": [{"error": "..."}]}}}
    if let Some(statuses) = response["response"]["data"]["statuses"].as_array() {
        for status in statuses {
            if status.get("error").is_some() {
                return true;
            }
        }
    }

    // Check for response containing error message
    if let Some(resp) = response.get("response") {
        if let Some(s) = resp.as_str() {
            if s.contains("error") || s.contains("Error") || s.contains("failed") {
                return true;
            }
        }
    }

    false
}

/// Parses an error response into a ccxt_core::Error.
///
/// # Arguments
///
/// * `response` - The JSON error response.
///
/// # Returns
///
/// A ccxt_core::Error with appropriate type and message.
pub fn parse_error(response: &Value) -> Error {
    // Try to extract error message
    let message = extract_error_message(response);

    // Map to error code
    let code = map_error_message(&message);

    code.into()
}

/// Extracts the error message from a response.
fn extract_error_message(response: &Value) -> String {
    // Try "message" field
    if let Some(msg) = response.get("message") {
        if let Some(s) = msg.as_str() {
            return s.to_string();
        }
    }
    // Try "error" field at top level
    if let Some(error) = response.get("error") {
        if let Some(s) = error.as_str() {
            return s.to_string();
        }
    }

    // Try "response" field as string
    if let Some(resp) = response.get("response") {
        if let Some(s) = resp.as_str() {
            return s.to_string();
        }
    }

    // Try order status errors: {"response": {"data": {"statuses": [{"error": "..."}]}}}
    if let Some(statuses) = response["response"]["data"]["statuses"].as_array() {
        for status in statuses {
            if let Some(error) = status.get("error") {
                if let Some(s) = error.as_str() {
                    return s.to_string();
                }
            }
        }
    }

    "Unknown error".to_string()
}

/// Maps an error message to an error code.
fn map_error_message(message: &str) -> HyperLiquidErrorCode {
    let lower = message.to_lowercase();

    // 签名和认证错误
    if lower.contains("signature") || lower.contains("auth") || lower.contains("unauthorized") {
        HyperLiquidErrorCode::InvalidSignature
    }
    // 用户和钱包错误
    else if lower.contains("user")
        && (lower.contains("not found") || lower.contains("does not exist"))
    {
        HyperLiquidErrorCode::UserNotFound
    }
    // 余额和保证金错误
    else if lower.contains("insufficient")
        || lower.contains("margin")
        || lower.contains("balance")
    {
        HyperLiquidErrorCode::InsufficientMargin
    }
    // 订单相关错误
    else if lower.contains("order") && lower.contains("not found") {
        HyperLiquidErrorCode::OrderNotFound
    } else if lower.contains("invalid price")
        || lower.contains("price") && lower.contains("invalid")
    {
        HyperLiquidErrorCode::InvalidParameter
    } else if lower.contains("invalid size") || lower.contains("size") && lower.contains("invalid")
    {
        HyperLiquidErrorCode::InvalidParameter
    } else if lower.contains("invalid") && lower.contains("order") {
        HyperLiquidErrorCode::InvalidParameter
    }
    // 跨仓和减仓错误
    else if lower.contains("cross") || lower.contains("would cross") {
        HyperLiquidErrorCode::OrderWouldCross
    } else if lower.contains("reduce only") || lower.contains("reduce-only") {
        HyperLiquidErrorCode::ReduceOnlyViolation
    }
    // 资产和交易对错误
    else if lower.contains("asset") && (lower.contains("invalid") || lower.contains("not found"))
    {
        HyperLiquidErrorCode::InvalidAsset
    } else if lower.contains("position") && lower.contains("not found") {
        HyperLiquidErrorCode::PositionNotFound
    }
    // 频率限制错误
    else if lower.contains("rate") || lower.contains("limit") || lower.contains("throttle") {
        HyperLiquidErrorCode::RateLimited
    }
    // 测试网特定错误
    else if lower.contains("testnet") && lower.contains("not supported") {
        HyperLiquidErrorCode::InvalidParameter
    }
    // 服务器错误
    else if lower.contains("server") || lower.contains("internal") {
        HyperLiquidErrorCode::ServerError
    }
    // 默认：未知错误
    else {
        HyperLiquidErrorCode::Unknown(message.to_string())
    }
}

/// Parses order status errors from the response.
///
/// HyperLiquid returns order errors in the statuses array:
/// ```json
/// {
///   "response": {
///     "data": {
///       "statuses": [{"error": "Order has invalid price."}]
///     }
///   }
/// }
/// ```
///
/// # Arguments
///
/// * `response` - The JSON response.
///
/// # Returns
///
/// `Some(Error)` if there's an order status error, `None` otherwise.
pub fn parse_order_status_error(response: &Value) -> Option<Error> {
    if let Some(statuses) = response["response"]["data"]["statuses"].as_array() {
        for status in statuses {
            if let Some(error) = status.get("error") {
                if let Some(error_msg) = error.as_str() {
                    return Some(parse_error(&serde_json::json!({
                        "error": error_msg
                    })));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_error_response_with_error_field() {
        let response = json!({"error": "Invalid signature"});
        assert!(is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_with_status_err() {
        let response = json!({"status": "err", "response": "Something went wrong"});
        assert!(is_error_response(&response));
    }

    #[test]
    fn test_is_error_response_success() {
        let response = json!({"status": "ok", "response": {"data": []}});
        assert!(!is_error_response(&response));
    }

    #[test]
    fn test_parse_error_insufficient_margin() {
        let response = json!({"error": "Insufficient margin for order"});
        let error = parse_error(&response);
        assert!(error.to_string().contains("Insufficient"));
    }

    #[test]
    fn test_parse_error_invalid_signature() {
        let response = json!({"error": "Invalid signature"});
        let error = parse_error(&response);
        assert!(error.to_string().contains("signature") || error.to_string().contains("Signature"));
    }

    #[test]
    fn test_parse_error_rate_limited() {
        let response = json!({"error": "Rate limit exceeded"});
        let error = parse_error(&response);
        assert!(error.to_string().contains("Rate") || error.to_string().contains("rate"));
    }

    #[test]
    fn test_parse_error_unknown() {
        let response = json!({"error": "Some unknown error occurred"});
        let error = parse_error(&response);
        assert!(error.to_string().contains("unknown") || error.to_string().contains("Unknown"));
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(
            HyperLiquidErrorCode::InvalidSignature.to_string(),
            "Invalid signature"
        );
        assert_eq!(
            HyperLiquidErrorCode::InsufficientMargin.to_string(),
            "Insufficient margin"
        );
    }

    #[test]
    fn test_error_code_into_ccxt_error() {
        let code = HyperLiquidErrorCode::InsufficientMargin;
        let error: Error = code.into();
        // Just verify it converts without panic
        let _ = error.to_string();
    }

    #[test]
    fn test_is_error_response_with_order_status_error() {
        let response = json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{"error": "Order has invalid price."}]
                }
            }
        });
        assert!(is_error_response(&response));
    }

    #[test]
    fn test_parse_order_status_error() {
        let response = json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{"error": "Order has invalid price."}]
                }
            }
        });
        let error = parse_order_status_error(&response);
        assert!(error.is_some());
        let error = error.unwrap();
        assert!(
            error.to_string().contains("invalid price") || error.to_string().contains("Invalid")
        );
    }

    #[test]
    fn test_parse_order_status_error_no_error() {
        let response = json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{"resting": {"oid": 12345}}]
                }
            }
        });
        let error = parse_order_status_error(&response);
        assert!(error.is_none());
    }

    #[test]
    fn test_extract_error_message_from_order_status() {
        let response = json!({
            "status": "ok",
            "response": {
                "type": "order",
                "data": {
                    "statuses": [{"error": "Order has invalid price."}]
                }
            }
        });
        let message = extract_error_message(&response);
        assert_eq!(message, "Order has invalid price.");
    }

    #[test]
    fn test_map_error_invalid_price() {
        let code = map_error_message("Order has invalid price.");
        assert_eq!(code, HyperLiquidErrorCode::InvalidParameter);
    }

    #[test]
    fn test_map_error_user_does_not_exist() {
        let code = map_error_message("User or API Wallet 0x1234 does not exist.");
        assert_eq!(code, HyperLiquidErrorCode::UserNotFound);
    }

    #[test]
    fn test_map_error_insufficient_balance() {
        let code = map_error_message("Insufficient balance for order");
        assert_eq!(code, HyperLiquidErrorCode::InsufficientMargin);
    }
}

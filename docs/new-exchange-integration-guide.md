# 新交易所接入指南

> 基于 rust-exchanges 项目实际开发经验，提供从 0 到 1 接入新交易所的完整流程。

**适用对象**：有 Rust 基础，了解 async/await 的开发者  
**预期时间**：有经验的开发者 8-12 小时，初次开发者 14-22 小时

---

## 目录

1. [接入前准备](#一接入前准备)
2. [模块实现优先级](#二模块实现优先级)
3. [分步实施指南](#三分步实施指南)
   - [阶段 1：基础结构搭建（30 分钟）](#阶段-1基础结构搭建30-分钟)
   - [阶段 2：核心配置模块（1 小时）](#阶段-2核心配置模块1-小时)
   - [阶段 3：认证模块（1-2 小时）](#阶段-3认证模块1-2-小时)
   - [阶段 4：数据解析器（1-2 小时）](#阶段-4数据解析器1-2-小时)
   - [阶段 5：REST API 实现（2-3 小时）](#阶段-5rest-api-实现2-3-小时)
   - [阶段 6：WebSocket 实现（2-3 小时）](#阶段-6websocket-实现2-3-小时)
   - [阶段 7：测试与验证（1-2 小时）](#阶段-7测试与验证1-2-小时)
4. [常见陷阱与解决方案](#四常见陷阱与解决方案)
5. [测试规范](#五测试规范)
6. [验证清单](#六验证清单)
7. [参考实现](#七参考实现)

---

## 一、接入前准备

### 1.1 收集 API 文档

在开始编码前，必须收集以下信息：

- [ ] **REST API 文档** - 完整的端点列表、请求格式、响应格式
- [ ] **WebSocket API 文档** - 订阅格式、消息格式、心跳机制
- [ ] **认证方式** - HMAC-SHA256、HMAC-SHA512、RSA、Ed25519
- [ ] **端点 URL** - 生产环境、测试网环境
- [ ] **支持的市场类型** - Spot（现货）、Swap（永续合约）、Futures（交割合约）
- [ ] **支持的时间周期** - 1m, 5m, 15m, 1h, 4h, 1d 等
- [ ] **速率限制** - 请求频率限制、权重计算规则

### 1.2 设计决策

回答以下问题，明确实现范围：

| 问题 | 选项 | 说明 |
|------|------|------|
| 支持哪些能力？ | MarketData, Trading, Account, Margin, Funding, WebSocket | 按需实现 |
| 市场类型？ | Spot, Swap, Futures | 至少实现 Spot |
| 认证方式？ | HMAC, RSA, Ed25519 | 查看 API 文档 |
| 是否需要时间同步？ | 是/否 | 如果 API 对时间戳敏感则需要 |
| 是否有测试网？ | 是/否 | 有测试网便于调试 |

### 1.3 环境要求

```bash
# Rust 1.70+
rustc --version

# 确保项目可以编译
cargo build --workspace
```

---

## 二、模块实现优先级

### 2.1 模块分类

根据依赖关系，将模块分为三类：

#### 🔴 基础模块（必须首先实现）

这些模块是整个交易所的基础，其他模块都依赖它们：

| 模块 | 文件位置 | 职责 | 依赖 |
|------|---------|------|------|
| **mod.rs** | `src/<exchange>/mod.rs` | 结构体定义、Options、Urls、Builder | 无 |
| **核心配置** | `src/<exchange>/core/` | Symbol 转换、常量定义 | 无 |
| **端点路由** | `src/<exchange>/network/endpoint_router.rs` | REST/WS 端点选择 | Options |

#### 🟡 核心模块（第二步实现）

这些模块实现交易所的核心功能：

| 模块 | 文件位置 | 职责 | 依赖 |
|------|---------|------|------|
| **认证** | `src/<exchange>/auth/` | REST 签名、WS 认证 | 基础模块 |
| **解析器** | `src/<exchange>/parser/` | REST 数据解析 | 无 |
| **REST API** | `src/<exchange>/rest/` | 公开/私有 API 调用 | 认证、解析器 |

#### 🟢 扩展模块（最后实现）

这些模块提供高级功能，可以在基础功能完成后逐步添加：

| 模块 | 文件位置 | 职责 | 依赖 |
|------|---------|------|------|
| **WebSocket** | `src/<exchange>/ws/` | 实时数据订阅 | 基础模块、解析器 |
| **Trait 实现** | `src/<exchange>/impls/` | MarketData、Trading 等 | REST API |
| **合约交易** | `src/<exchange>/swap/` | 合约下单逻辑 | REST API |

### 2.2 实现顺序

```
基础模块 → 核心模块 → 扩展模块
   ↓          ↓          ↓
mod.rs    认证模块    WebSocket
核心配置  解析器      Trait 实现
端点路由  REST API    合约交易
```

**关键原则**：
1. **先实现基础模块**，确保可以创建交易所实例
2. **再实现 REST API**，测试公开接口（不需要 API Key）
3. **然后实现认证**，测试私有接口（需要 API Key）
4. **最后实现 WebSocket**，测试实时数据

---

## 三、分步实施指南

### 阶段 1：基础结构搭建（30 分钟）

#### 1.1 创建目录结构

```bash
cd ccxt-exchanges/src

# 创建交易所目录（以 gate 为例）
mkdir -p gate/{auth,core,network,parser,rest,ws,impls}

# 创建必要文件
touch gate/mod.rs
touch gate/auth/mod.rs
touch gate/auth/core.rs
touch gate/auth/signed_request.rs
touch gate/core/mod.rs
touch gate/core/builder.rs
touch gate/core/symbol.rs
touch gate/network/mod.rs
touch gate/network/endpoint_router.rs
touch gate/parser/mod.rs
touch gate/rest/mod.rs
touch gate/ws/mod.rs
touch gate/ws/builder.rs
touch gate/ws/parser.rs
touch gate/impls/mod.rs
touch gate/impls/market_data.rs
touch gate/impls/trading.rs
```

#### 1.2 配置 Feature Flag

编辑 `ccxt-exchanges/Cargo.toml`：

```toml
[features]
default = ["binance", "okx", "bybit", "bitget", "gate", "hyperliquid", "rustls-tls", "websocket"]

# 添加新交易所 feature
gate = []
```

编辑 `ccxt-exchanges/src/lib.rs`：

```rust
/// Gate.io exchange implementation.
#[cfg(feature = "gate")]
pub mod gate;
```

#### 1.3 创建模块入口（`mod.rs`）

这是交易所的核心文件，定义结构体和配置：

```rust
//! Gate.io exchange implementation.

use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};
use ccxt_core::{BaseExchange, ExchangeConfig, Result};
use serde::{Deserialize, Serialize};

pub mod auth;
pub mod core;
pub mod impls;
pub mod network;
pub mod parser;
pub mod rest;
pub mod ws;

// 公开导出
pub use auth::GateAuth;
pub use core::GateBuilder;
pub use network::GateEndpointRouter;

/// Gate.io exchange structure.
#[derive(Debug)]
pub struct Gate {
    /// Base exchange instance.
    base: BaseExchange,
    /// Exchange-specific options.
    options: GateOptions,
    /// WebSocket client (lazily initialized).
    ws_client: std::sync::OnceLock<ws::GateWsClient>,
    /// WebSocket client with authentication.
    ws_client_auth: std::sync::OnceLock<ws::GateWsClientAuth>,
}

/// Gate.io specific options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateOptions {
    /// Default market type.
    #[serde(default)]
    pub default_type: DefaultType,
    /// Default sub-type for contract settlement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sub_type: Option<DefaultSubType>,
    /// Enables testnet/demo environment.
    #[serde(default)]
    pub testnet: bool,
}

impl Default for GateOptions {
    fn default() -> Self {
        Self {
            default_type: DefaultType::Spot,
            default_sub_type: None,
            testnet: false,
        }
    }
}

impl Gate {
    /// Creates a new Gate instance using the builder pattern.
    pub fn builder() -> GateBuilder {
        GateBuilder::new()
    }

    /// Creates a new Gate instance.
    pub fn new(config: ExchangeConfig) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        let options = GateOptions::default();
        
        Ok(Self {
            base,
            options,
            ws_client: std::sync::OnceLock::new(),
            ws_client_auth: std::sync::OnceLock::new(),
        })
    }

    /// Creates a new Gate instance with custom options.
    pub fn new_with_options(config: ExchangeConfig, options: GateOptions) -> Result<Self> {
        let base = BaseExchange::new(config)?;
        Ok(Self {
            base,
            options,
            ws_client: std::sync::OnceLock::new(),
            ws_client_auth: std::sync::OnceLock::new(),
        })
    }
    
    /// Returns a reference to the base exchange.
    pub fn base(&self) -> &BaseExchange {
        &self.base
    }

    /// Returns the WebSocket client (public channels).
    pub fn ws_client(&self) -> &ws::GateWsClient {
        self.ws_client.get_or_init(|| {
            ws::create_gate_ws_client(self.is_testnet())
        })
    }

    /// Returns the WebSocket client with authentication (private channels).
    pub fn ws_client_auth(&self) -> &ws::GateWsClientAuth {
        self.ws_client_auth.get_or_init(|| {
            let api_key = self.base.config.api_key
                .as_ref()
                .map(|k| k.expose_secret().clone())
                .unwrap_or_default();
            let secret = self.base.config.secret
                .as_ref()
                .map(|s| s.expose_secret().clone())
                .unwrap_or_default();
            let auth = ws::GateWsAuth::new(api_key, secret);
            ws::create_gate_ws_client_auth(self.is_testnet(), auth)
        })
    }

    /// Returns `true` if testnet mode is enabled.
    pub fn is_testnet(&self) -> bool {
        self.base().config.sandbox || self.options.testnet
    }
}
```

**关键点**：
- ✅ 使用 `OnceLock` 懒加载 WebSocket 客户端
- ✅ 分离公共和私有 WebSocket 客户端
- ✅ 提供 Builder 模式创建实例
- ✅ `is_testnet()` 同时检查 `sandbox` 和 `testnet` 标志

---

### 阶段 2：核心配置模块（1 小时）

#### 2.1 Builder 模式（`core/builder.rs`）

提供流畅的 API 来创建交易所实例：

```rust
//! Builder pattern for Gate.

use ccxt_core::{ExchangeConfig, Result, credentials::SecretString};
use crate::{Gate, GateOptions};

/// Builder for creating Gate instances.
pub struct GateBuilder {
    config: ExchangeConfig,
    options: GateOptions,
}

impl GateBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self {
            config: ExchangeConfig::default(),
            options: GateOptions::default(),
        }
    }

    /// Sets the API key.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config.api_key = Some(SecretString::new(api_key.into()));
        self
    }

    /// Sets the secret.
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.config.secret = Some(SecretString::new(secret.into()));
        self
    }

    /// Sets sandbox/testnet mode.
    pub fn sandbox(mut self, sandbox: bool) -> Self {
        self.config.sandbox = sandbox;
        self
    }

    /// Sets testnet mode via options.
    pub fn testnet(mut self, testnet: bool) -> Self {
        self.options.testnet = testnet;
        self
    }

    /// Sets the default market type.
    pub fn default_type(mut self, default_type: DefaultType) -> Self {
        self.options.default_type = default_type;
        self
    }

    /// Sets the default sub-type.
    pub fn default_sub_type(mut self, sub_type: DefaultSubType) -> Self {
        self.options.default_sub_type = Some(sub_type);
        self
    }

    /// Builds the Gate instance.
    pub fn build(self) -> Result<Gate> {
        Gate::new_with_options(self.config, self.options)
    }
}

impl Default for GateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
```

**使用示例**：

```rust
// 现货实例
let gate_spot = Gate::builder()
    .default_type(DefaultType::Spot)
    .build()?;

// 合约实例（USDT 结算）
let gate_swap = Gate::builder()
    .default_type(DefaultType::Swap)
    .default_sub_type(DefaultSubType::Linear)
    .build()?;

// 测试网实例
let gate_testnet = Gate::builder()
    .default_type(DefaultType::Swap)
    .testnet(true)
    .api_key("your_api_key")
    .secret("your_secret")
    .build()?;
```

#### 2.2 Symbol 转换器（`core/symbol.rs`）

**⚠️ 重要**：不同交易所的 Symbol 格式不同，必须正确实现转换逻辑。

```rust
//! Symbol converter for Gate.

use ccxt_core::symbol::{SymbolConverter, SymbolContext, ParsedSymbol};
use ccxt_core::types::market::SymbolMarketType;

/// Gate.io symbol converter.
/// 
/// Gate uses underscore format: BTC_USDT
/// CCXT uses slash format: BTC/USDT
#[derive(Debug, Clone, Copy)]
pub struct GateSymbolConverter;

impl SymbolConverter for GateSymbolConverter {
    /// Convert CCXT symbol to exchange format.
    /// 
    /// Examples:
    /// - BTC/USDT → BTC_USDT (spot)
    /// - BTC/USDT:USDT → BTC_USDT (swap)
    fn to_exchange_id(&self, symbol: &ParsedSymbol) -> String {
        match symbol.market_type {
            SymbolMarketType::Spot => {
                format!("{}_{}", symbol.base, symbol.quote)
            }
            SymbolMarketType::LinearSwap | SymbolMarketType::InverseSwap => {
                // Swap: 去掉 :USDT 后缀
                format!("{}_{}", symbol.base, symbol.quote)
            }
            SymbolMarketType::Future => {
                // Future: BTC/USDT:BTC-20241231 → BTC_USDT-20241231
                if let Some(expiry) = &symbol.expiry_date {
                    format!("{}_{:?}", symbol.base, expiry)
                } else {
                    format!("{}_{}", symbol.base, symbol.quote)
                }
            }
        }
    }

    /// Convert exchange format to CCXT symbol.
    fn from_exchange_id(&self, id: &str, market_type: SymbolMarketType) -> ParsedSymbol {
        let parts: Vec<&str> = id.split('_').collect();
        
        match market_type {
            SymbolMarketType::Spot => {
                if parts.len() >= 2 {
                    ParsedSymbol::new(parts[0], parts[1], SymbolMarketType::Spot)
                } else {
                    ParsedSymbol::new_unchecked(id)
                }
            }
            SymbolMarketType::LinearSwap | SymbolMarketType::InverseSwap => {
                if parts.len() >= 2 {
                    ParsedSymbol::new(parts[0], parts[1], SymbolMarketType::LinearSwap)
                        .with_settle(parts[1])
                } else {
                    ParsedSymbol::new_unchecked(id)
                }
            }
            _ => ParsedSymbol::new_unchecked(id),
        }
    }
}
```

**⚠️ 常见陷阱**：
- ❌ 假设所有交易所都使用相同格式（Binance 用 `BTCUSDT`，Gate 用 `BTC_USDT`）
- ❌ 合约 Symbol 忘记添加结算币种（应该是 `BTC/USDT:USDT`）
- ✅ **正确做法**：打印实际 API 返回的 symbol 格式，据此编写转换逻辑

#### 2.3 端点路由（`network/endpoint_router.rs`）

管理 REST 和 WebSocket 端点选择：

```rust
//! Endpoint router for Gate.

use ccxt_core::ws::{WsEndpointProvider, WsContext};
use crate::GateOptions;
use ccxt_core::types::market::MarketType;

/// Gate.io endpoint router.
pub struct GateEndpointRouter;

impl GateEndpointRouter {
    /// Returns REST API endpoint.
    pub fn rest_endpoint(options: &GateOptions, market_type: MarketType) -> String {
        if options.is_testnet() {
            // Gate 测试网：现货和合约共用同一个域名
            "https://api-testnet.gateapi.io".to_string()
        } else {
            // 生产环境：现货和合约使用相同端点
            "https://api.gateio.ws/api/v4".to_string()
        }
    }

    /// Returns WebSocket endpoint.
    pub fn ws_endpoint(options: &GateOptions, market_type: MarketType) -> String {
        if options.is_testnet() {
            "wss://api-testnet.gateapi.io/v4/ws".to_string()
        } else {
            match market_type {
                MarketType::Spot => "wss://api.gateio.ws/v4/ws".to_string(),
                MarketType::Swap | MarketType::Futures => {
                    // ⚠️ 合约 WebSocket 使用不同的域名
                    "wss://fx-ws.gateio.ws/v4/ws/usdt".to_string()
                }
            }
        }
    }
}

/// WebSocket endpoint provider for GenericWsClient.
pub struct GateWsEndpointProvider {
    is_testnet: bool,
    market_type: MarketType,
}

impl GateWsEndpointProvider {
    pub fn new(is_testnet: bool, market_type: MarketType) -> Self {
        Self { is_testnet, market_type }
    }
}

impl WsEndpointProvider for GateWsEndpointProvider {
    fn get_url(&self, _context: &WsContext) -> String {
        let options = GateOptions {
            testnet: self.is_testnet,
            ..GateOptions::default()
        };
        GateEndpointRouter::ws_endpoint(&options, self.market_type)
    }
}
```

**⚠️ 关键陷阱**（来自 Gate 实战经验）：
- ❌ 旧文档中的 `fx-api-testnet.gateapi.io` 已废弃
- ✅ 测试网域名已统一为 `api-testnet.gateapi.io`
- ⚠️ WebSocket 端点必须在连接时根据 `market_type` 选择，不能依赖订阅列表

---

### 阶段 3：认证模块（1-2 小时）

#### 3.1 REST 认证（`auth/core.rs`）

根据交易所 API 文档实现签名逻辑：

```rust
//! Gate.io authentication.

use ccxt_core::{Error, Result};
use hmac::{Hmac, Mac};
use sha2::Sha512;

/// Gate.io authenticator.
/// 
/// Gate uses HMAC-SHA512 signature.
pub struct GateAuth {
    api_key: String,
    secret: String,
}

impl GateAuth {
    /// Creates a new authenticator.
    pub fn new(api_key: String, secret: String) -> Self {
        Self { api_key, secret }
    }

    /// Signs a message with HMAC-SHA512.
    pub fn sign(&self, message: &str) -> Result<String> {
        let mut mac = Hmac::<Sha512>::new_from_slice(self.secret.as_bytes())
            .map_err(|e| Error::auth_error(&format!("HMAC init failed: {}", e)))?;
        mac.update(message.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        Ok(hex::encode(code_bytes))
    }

    /// Creates signature for HTTP request.
    /// 
    /// Sign string format:
    /// ```
    /// {METHOD}\n{REQUEST_URL}\n\n{BODY_HASH}\nsha512\n{TIMESTAMP}
    /// ```
    pub fn create_signature(
        &self,
        method: &str,
        url: &str,
        body: &str,
        timestamp: i64,
    ) -> Result<String> {
        // Calculate body hash
        use sha2::Digest;
        let mut hasher = Sha512::new();
        hasher.update(body.as_bytes());
        let body_hash = hex::encode(hasher.finalize().as_slice());

        // Build sign string
        let sign_string = format!(
            "{}\n{}\n\n{}\nsha512\n{}",
            method.to_uppercase(),
            url,
            timestamp,
            body_hash
        );

        self.sign(&sign_string)
    }

    /// Returns the API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}
```

**⚠️ 关键点**：
1. **查看官方文档**，确认签名算法（HMAC-SHA256 vs HMAC-SHA512）
2. **确认签名字符串格式**，不同交易所格式不同
3. **测试签名逻辑**，使用官方提供的测试用例验证

#### 3.2 签名请求构建器（`auth/signed_request.rs`）

```rust
//! Signed request builder for Gate.

use crate::gate::Gate;
use ccxt_core::Result;
use reqwest::header::HeaderMap;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// HTTP method.
#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }
}

/// Signed request builder.
pub struct GateSignedRequestBuilder<'a> {
    exchange: &'a Gate,
    endpoint: String,
    method: HttpMethod,
    params: Vec<(String, String)>,
}

impl<'a> GateSignedRequestBuilder<'a> {
    /// Creates a new signed request builder.
    pub fn new(exchange: &'a Gate, endpoint: impl Into<String>) -> Self {
        Self {
            exchange,
            endpoint: endpoint.into(),
            method: HttpMethod::Get,
            params: Vec::new(),
        }
    }

    /// Sets the HTTP method.
    pub fn method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    /// Adds a parameter.
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((key.into(), value.into()));
        self
    }

    /// Executes the request.
    pub async fn execute(self) -> Result<Value> {
        // 1. Build URL
        let base_url = GateEndpointRouter::rest_endpoint(
            &self.exchange.options,
            MarketType::Spot,
        );
        let mut full_url = format!("{}{}", base_url, self.endpoint);

        // 2. Add query parameters
        let body = if !self.params.is_empty() {
            let query = self
                .params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            
            if self.method == HttpMethod::Get {
                full_url = format!("{}?{}", full_url, query);
                String::new()
            } else {
                query
            }
        } else {
            String::new()
        };

        // 3. Generate timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 4. Sign request
        let auth = GateAuth::new(
            self.exchange.base.config.api_key
                .as_ref()
                .map(|k| k.expose_secret().clone())
                .unwrap_or_default(),
            self.exchange.base.config.secret
                .as_ref()
                .map(|s| s.expose_secret().clone())
                .unwrap_or_default(),
        );

        let signature = auth.create_signature(
            self.method.as_str(),
            &self.endpoint,
            &body,
            timestamp,
        )?;

        // 5. Build headers
        let mut headers = HeaderMap::new();
        headers.insert("KEY", auth.api_key().parse()?);
        headers.insert("Timestamp", timestamp.to_string().parse()?);
        headers.insert("SIGN", signature.parse()?);

        // 6. Send request
        match self.method {
            HttpMethod::Get => {
                self.exchange.base().http_client.get(&full_url, Some(headers)).await
            }
            HttpMethod::Post => {
                self.exchange.base().http_client.post(&full_url, Some(headers), Some(body)).await
            }
            _ => todo!("Implement other HTTP methods"),
        }
    }
}
```

---

### 阶段 4：数据解析器（1-2 小时）

#### 4.1 解析器设计原则

**⚠️ 关键原则**：REST API 和 WebSocket 返回的数据格式通常不同，必须使用独立的解析函数。

```
解析器目录结构：
parser/
├── market.rs        # Market 数据解析
├── ticker.rs        # Ticker 数据解析
├── orderbook.rs     # OrderBook 数据解析
├── ohlcv.rs         # OHLCV 数据解析
└── trade.rs         # Trade 数据解析
```

#### 4.2 Ticker 解析器示例（`parser/ticker.rs`）

```rust
//! Ticker parser for Gate.

use ccxt_core::types::{Ticker, Price, Amount};
use ccxt_core::{Error, Result};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse ticker from REST API response.
/// 
/// REST format:
/// ```json
/// {
///   "currency_pair": "BTC_USDT",
///   "last": "50000",
///   "highestBid": "49900",
///   "lowestAsk": "50100",
///   "changePercentage": "2.5",
///   "baseVolume": "1000",
///   "quoteVolume": "50000000"
/// }
/// ```
pub fn parse_ticker(data: &Value) -> Result<Ticker> {
    let symbol = data["currency_pair"]
        .as_str()
        .ok_or_else(|| Error::parse("Missing currency_pair"))?
        .replace('_', "/");

    let last = parse_decimal(data, "last")
        .ok_or_else(|| Error::parse("Missing last price"))?;

    Ok(Ticker {
        symbol,
        timestamp: parse_timestamp(data, "timestamp").unwrap_or(0),
        datetime: None,
        high: parse_decimal(data, "high24h").map(Price::new),
        low: parse_decimal(data, "low24h").map(Price::new),
        bid: parse_decimal(data, "highestBid").map(Price::new),
        bid_volume: parse_decimal(data, "bidVolume").map(Amount::new),
        ask: parse_decimal(data, "lowestAsk").map(Price::new),
        ask_volume: parse_decimal(data, "askVolume").map(Amount::new),
        vwap: None,
        open: parse_decimal(data, "open").map(Price::new),
        close: Some(Price::new(last)),
        last: Some(Price::new(last)),
        previous_close: None,
        change: parse_decimal(data, "change").map(Price::new),
        percentage: parse_decimal(data, "changePercentage"),
        average: None,
        base_volume: parse_decimal(data, "baseVolume").map(Amount::new),
        quote_volume: parse_decimal(data, "quoteVolume").map(Amount::new),
        info: data.clone(),
    })
}

/// Parse ticker from WebSocket message.
/// 
/// ⚠️ WebSocket format differs from REST!
/// 
/// Spot format:
/// ```json
/// {
///   "currency_pair": "BTC_USDT",
///   "last": "50000"
/// }
/// ```
/// 
/// Futures format (array!):
/// ```json
/// [{
///   "contract": "BTC_USDT",
///   "last": "50000"
/// }]
/// ```
pub fn parse_ws_ticker(data: &Value) -> Result<Ticker> {
    // Handle array format (futures)
    let ticker_data = if data.is_array() {
        data.as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::parse("Empty ticker array"))?
    } else {
        data
    };

    // Extract symbol (spot uses currency_pair, futures uses contract)
    let symbol = if let Some(cp) = ticker_data["currency_pair"].as_str() {
        cp.replace('_', "/")
    } else if let Some(c) = ticker_data["contract"].as_str() {
        format!("{}:USDT", c.replace('_', "/"))
    } else {
        return Err(Error::parse("Missing currency_pair or contract"));
    };

    let last = parse_decimal(ticker_data, "last")
        .ok_or_else(|| Error::parse("Missing last price"))?;

    Ok(Ticker {
        symbol,
        // ... 其他字段解析
        last: Some(Price::new(last)),
        info: data.clone(),
    })
}

/// Helper: parse decimal from JSON.
fn parse_decimal(data: &Value, key: &str) -> Option<Decimal> {
    data[key]
        .as_str()
        .and_then(|s| s.parse::<Decimal>().ok())
        .or_else(|| data[key].as_str().and_then(|s| s.parse().ok()))
}

/// Helper: parse timestamp from JSON.
fn parse_timestamp(data: &Value, key: &str) -> Option<i64> {
    data[key]
        .as_i64()
        .or_else(|| data[key].as_str().and_then(|s| s.parse().ok()))
}
```

**⚠️ 关键陷阱**（来自 Gate 实战经验）：
- ❌ 假设 REST 和 WebSocket 格式相同
- ❌ 没有处理合约返回数组的情况
- ✅ **正确做法**：分别实现 `parse_ticker()` 和 `parse_ws_ticker()`，并兼容对象/数组两种格式

#### 4.3 使用 Parser Helpers

项目提供了通用解析辅助函数，减少重复代码：

```rust
use ccxt_exchanges::common::parser_helpers::{ParseHelper, parse_amount_safe, parse_price_safe};

// 多键回退解析
let price = ParseHelper::decimal_any(data, &["price", "lastPrice", "px"]);

// 安全解析（带默认值）
let amount = parse_amount_safe(data, "amount", Decimal::ZERO);

// 时间戳解析
let timestamp = ParseHelper::timestamp_any(data, &["timestamp", "time", "ts"]);
```

---

### 阶段 5：REST API 实现（2-3 小时）

#### 5.1 公开 API（`rest/mod.rs`）

```rust
//! Gate.io REST API implementation.

use crate::gate::Gate;
use ccxt_core::types::market::MarketType;
use ccxt_core::{Error, Result};
use serde_json::Value;

impl Gate {
    /// Public GET request (no authentication).
    pub(crate) async fn public_get(
        &self,
        endpoint: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        let base_url = GateEndpointRouter::rest_endpoint(
            &self.options,
            MarketType::Spot,
        );
        
        let mut url = format!("{}{}", base_url, endpoint);
        
        if let Some(params) = params {
            let query = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{}?{}", url, query);
        }

        let response = self.base().http_client.get(&url, None).await?;
        self.check_response(response)
    }

    /// Check API response for errors.
    fn check_response(&self, response: Value) -> Result<Value> {
        // Gate returns errors in this format:
        // {"label": "ERROR_LABEL", "message": "Error description"}
        if let Some(label) = response.get("label") {
            return Err(Error::exchange(
                label.as_str().unwrap_or("UNKNOWN"),
                response["message"].as_str().unwrap_or("Unknown error"),
            ));
        }
        Ok(response)
    }
}
```

#### 5.2 MarketData Trait 实现（`impls/market_data.rs`）

```rust
//! MarketData trait implementation.

use async_trait::async_trait;
use ccxt_core::traits::MarketData;
use ccxt_core::types::{Market, Ohlcv, OrderBook, Ticker, Trade};
use ccxt_core::types::trading::params::OhlcvParams;
use ccxt_core::Result;

use crate::gate::Gate;

#[async_trait]
impl MarketData for Gate {
    async fn fetch_markets(&self) -> Result<Vec<Market>> {
        let data = self.public_get("/spot/currency_pairs", None).await?;
        let markets = if data.is_array() {
            data.as_array()
                .unwrap()
                .iter()
                .filter_map(|item| parse_market(item).ok())
                .collect()
        } else {
            vec![]
        };
        Ok(markets)
    }

    async fn fetch_ticker(&self, symbol: &str) -> Result<Ticker> {
        // Convert symbol to exchange format
        let exchange_id = self.to_exchange_id(symbol)?;
        
        let params = [("currency_pair", exchange_id.as_str())];
        let data = self.public_get("/spot/tickers", Some(&params)).await?;
        
        if data.is_array() {
            let arr = data.as_array().unwrap();
            if arr.is_empty() {
                return Err(Error::bad_symbol(format!("Ticker not found: {}", symbol)));
            }
            crate::gate::parser::parse_ticker(&arr[0])
        } else {
            crate::gate::parser::parse_ticker(&data)
        }
    }

    async fn fetch_order_book(&self, symbol: &str, limit: Option<u32>) -> Result<OrderBook> {
        // TODO: Implement
        todo!()
    }

    async fn fetch_ohlcv_with_params(
        &self,
        symbol: &str,
        params: OhlcvParams,
    ) -> Result<Vec<Ohlcv>> {
        // TODO: Implement
        todo!()
    }

    async fn fetch_trades(&self, symbol: &str, since: Option<i64>, limit: Option<u32>) -> Result<Vec<Trade>> {
        // TODO: Implement
        todo!()
    }
}

/// Parse market from JSON.
fn parse_market(data: &Value) -> Result<Market> {
    let id = data["id"]
        .as_str()
        .ok_or_else(|| Error::parse("Missing market id"))?
        .to_string();
    
    let base = data["base"]
        .as_str()
        .ok_or_else(|| Error::parse("Missing base currency"))?
        .to_string();
    
    let quote = data["quote"]
        .as_str()
        .ok_or_else(|| Error::parse("Missing quote currency"))?
        .to_string();

    let symbol = format!("{}/{}", base, quote);

    Ok(Market {
        id,
        symbol,
        base,
        quote,
        // ... 其他字段
        ..Default::default()
    })
}
```

#### 5.3 Trading Trait 实现（`impls/trading.rs`）

```rust
//! Trading trait implementation.

use async_trait::async_trait;
use ccxt_core::traits::Trading;
use ccxt_core::types::order::{Order, OrderSide, OrderType};
use ccxt_core::types::trading::params::OrderParams;
use ccxt_core::Result;

use crate::gate::Gate;

#[async_trait]
impl Trading for Gate {
    async fn create_order(&self, params: OrderParams) -> Result<Order> {
        self.check_required_credentials()?;
        
        // TODO: Implement order creation
        // 1. Convert symbol to exchange format
        // 2. Build request body
        // 3. Sign and send request
        // 4. Parse response
        
        todo!()
    }

    async fn cancel_order(&self, id: &str, symbol: &str) -> Result<Order> {
        self.check_required_credentials()?;
        
        // TODO: Implement
        todo!()
    }

    async fn fetch_order(&self, id: &str, symbol: &str) -> Result<Order> {
        self.check_required_credentials()?;
        
        // TODO: Implement
        todo!()
    }

    async fn fetch_open_orders(&self, symbol: Option<&str>, since: Option<i64>, limit: Option<u32>) -> Result<Vec<Order>> {
        self.check_required_credentials()?;
        
        // TODO: Implement
        todo!()
    }

    async fn fetch_history_orders(&self, symbol: Option<&str>, since: Option<i64>, limit: Option<u32>) -> Result<Vec<Order>> {
        self.check_required_credentials()?;
        
        // TODO: Implement
        // ⚠️ 注意：是 fetch_history_orders，不是 fetch_orders
        // 不要合并 open + closed 订单
        todo!()
    }
}
```

**⚠️ 重要提醒**：
- ❌ **不要实现 `fetch_orders` 来合并 open + closed 订单**
- ✅ 只实现 `fetch_open_orders` 和 `fetch_history_orders`
- ✅ 如果应用层需要"所有订单"，在调用方实现，不在交易所适配层实现

---

### 阶段 6：WebSocket 实现（2-3 小时）

#### 6.1 使用 GenericWsClient 架构

**⚠️ 推荐使用新架构**，不要使用旧的独立连接架构。

**新架构优势**：
- ✅ 单一连接 + 消息广播（旧架构每个订阅独立连接）
- ✅ 内置自动重连 + 订阅恢复
- ✅ 引用计数支持同一频道多次订阅
- ✅ 代码量减少约 85%（1500 行 → 200 行）

#### 6.2 WebSocket 模块入口（`ws/mod.rs`）

```rust
//! Gate.io WebSocket implementation using GenericWsClient.

mod builder;
mod parser;
mod auth;

pub use builder::GateSubscriptionBuilder;
pub use parser::GateStreamParser;
pub use auth::GateWsAuth;

use crate::gate::network::endpoint_router::GateWsEndpointProvider;
use ccxt_core::ws::{GenericWsClient, NoAuth};
use ccxt_core::types::market::MarketType;

/// Gate.io WebSocket client (public channels).
pub type GateWsClient = GenericWsClient<
    GateSubscriptionBuilder,
    GateStreamParser,
    GateWsEndpointProvider,
    NoAuth,
>;

/// Gate.io WebSocket client with authentication (private channels).
pub type GateWsClientAuth = GenericWsClient<
    GateSubscriptionBuilder,
    GateStreamParser,
    GateWsEndpointProvider,
    GateWsAuth,
>;

/// Create WebSocket client (public channels).
pub fn create_gate_ws_client(is_testnet: bool) -> GateWsClient {
    GateWsClient::new(
        GateSubscriptionBuilder,
        GateStreamParser,
        GateWsEndpointProvider::new(is_testnet, MarketType::Spot),
        NoAuth,
    )
}

/// Create WebSocket client with authentication (private channels).
pub fn create_gate_ws_client_auth(is_testnet: bool, auth: GateWsAuth) -> GateWsClientAuth {
    GateWsClientAuth::new(
        GateSubscriptionBuilder,
        GateStreamParser,
        GateWsEndpointProvider::new(is_testnet, MarketType::Spot),
        auth,
    )
}
```

#### 6.3 SubscriptionBuilder（`ws/builder.rs`）

```rust
//! Subscription builder for Gate.

use ccxt_core::ws::{SubscriptionBuilder, SubscriptionChannel};
use ccxt_core::Result;
use serde_json::{json, Value};

/// Gate.io subscription builder.
pub struct GateSubscriptionBuilder;

impl SubscriptionBuilder for GateSubscriptionBuilder {
    /// Build subscribe message.
    /// 
    /// Gate format:
    /// ```json
    /// {
    ///   "channel": "spot.tickers",
    ///   "event": "subscribe",
    ///   "payload": ["BTC_USDT"]
    /// }
    /// ```
    fn build_subscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let payload: Vec<String> = channels
            .iter()
            .map(|ch| {
                // Convert symbol: BTC/USDT → BTC_USDT
                ch.symbol.replace('/', "_")
            })
            .collect();

        Ok(json!({
            "channel": channels[0].channel_type.to_string(),
            "event": "subscribe",
            "payload": payload
        }))
    }

    /// Build unsubscribe message.
    fn build_unsubscribe(&self, channels: &[SubscriptionChannel]) -> Result<Value> {
        let payload: Vec<String> = channels
            .iter()
            .map(|ch| ch.symbol.replace('/', "_"))
            .collect();

        Ok(json!({
            "channel": channels[0].channel_type.to_string(),
            "event": "unsubscribe",
            "payload": payload
        }))
    }

    /// Extract channel identifier from message for routing.
    /// 
    /// ⚠️ This is critical! Must handle all possible symbol fields.
    /// 
    /// Gate uses different fields for different channels:
    /// - Ticker/Trade: `currency_pair` or `contract`
    /// - OrderBook: `s`
    /// - Candlesticks: `n` (format: "1m_BTC_USDT")
    fn extract_channel(&self, msg: &Value) -> Option<String> {
        let channel = msg["channel"].as_str()?;
        let result = &msg["result"];

        // Try all possible symbol fields
        let symbol = if let Some(currency_pair) = result["currency_pair"].as_str() {
            Some(currency_pair.replace('_', "/"))
        } else if let Some(s) = result["s"].as_str() {
            // OrderBook uses 's' field
            Some(s.replace('_', "/"))
        } else if let Some(n) = result["n"].as_str() {
            // Candlesticks uses 'n' field (format: "1m_BTC_USDT")
            if let Some(pos) = n.find('_') {
                let symbol = &n[pos + 1..];
                Some(symbol.replace('_', "/"))
            } else {
                Some(n.to_string())
            }
        } else {
            None
        };

        // Add suffix for futures
        if channel.starts_with("futures.") {
            Some(format!("{}:USDT", symbol?))
        } else {
            symbol
        }
    }
}
```

**⚠️ 关键陷阱**（来自 Gate 实战经验）：
- ❌ 只处理 `currency_pair` 字段，导致 K线和订单簿无法路由
- ❌ 没有处理合约的数组格式
- ✅ **正确做法**：按优先级尝试所有可能的字段（`currency_pair` → `s` → `n`）

**调试方法**：
```rust
// 在 ccxt-core/src/network/ws_client/mod.rs 中添加原始消息日志
#[cfg(test)]
println!("[WS-RAW-RECV] {}", text);
println!("[WS-RAW-SEND] {}", text);
```

#### 6.4 StreamParser（`ws/parser.rs`）

```rust
//! Stream parser for Gate.

use ccxt_core::ws::{StreamParser, StreamParserExt, WsMessage, Parseable};
use ccxt_core::types::{Ticker, OrderBook, Trade, Ohlcv};
use ccxt_core::Result;
use serde_json::Value;

/// Gate.io stream parser.
pub struct GateStreamParser;

impl StreamParser for GateStreamParser {
    /// Parse incoming message.
    fn parse(&self, msg: &Value) -> Result<WsMessage> {
        // Check for heartbeat
        if msg.get("event").and_then(|e| e.as_str()) == Some("ping") {
            return Ok(WsMessage::Heartbeat);
        }

        // Route to specific parser based on channel
        if let Some(channel) = msg.get("channel").and_then(|c| c.as_str()) {
            match channel {
                c if c.contains("tickers") => {
                    self.parse_ticker(&msg["result"]).map(WsMessage::Ticker)
                }
                c if c.contains("order_book") => {
                    self.parse_orderbook(&msg["result"]).map(WsMessage::OrderBook)
                }
                c if c.contains("trades") => {
                    self.parse_trades(&msg["result"]).map(WsMessage::Trades)
                }
                c if c.contains("candlesticks") => {
                    self.parse_ohlcv(&msg["result"]).map(WsMessage::Ohlcv)
                }
                _ => Ok(WsMessage::Other(msg.clone())),
            }
        } else {
            Ok(WsMessage::Heartbeat)
        }
    }

    fn parse_ticker(&self, msg: &Value) -> Result<Ticker> {
        // Use the WebSocket-specific parser
        crate::gate::parser::parse_ws_ticker(msg)
    }

    fn parse_orderbook(&self, msg: &Value) -> Result<OrderBook> {
        // TODO: Implement
        todo!()
    }

    fn parse_trades(&self, msg: &Value) -> Result<Vec<Trade>> {
        // TODO: Implement
        todo!()
    }

    fn parse_ohlcv(&self, msg: &Value) -> Result<Vec<Ohlcv>> {
        // TODO: Implement
        // ⚠️ Must handle both object (spot) and array (futures) formats
        todo!()
    }
}

impl StreamParserExt for GateStreamParser {
    fn parse_as<T: Parseable>(&self, msg: &Value) -> Result<T> {
        T::parse_from(msg, self)
    }
}
```

#### 6.5 WebSocket 认证（`ws/auth.rs`）

```rust
//! WebSocket authentication for Gate.

use ccxt_core::ws::auth::{WsAuthCore, MessageAuthenticator, AuthMode};
use ccxt_core::Result;
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Gate.io WebSocket authentication.
pub struct GateWsAuth {
    api_key: String,
    secret: String,
}

impl GateWsAuth {
    pub fn new(api_key: String, secret: String) -> Self {
        Self { api_key, secret }
    }
}

impl WsAuthCore for GateWsAuth {
    fn auth_mode(&self) -> AuthMode {
        AuthMode::Message // Authentication via message
    }
}

impl MessageAuthenticator for GateWsAuth {
    fn build_auth_message(&self) -> Result<Value> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Build sign string
        let sign_string = format!("{}\n{}", timestamp, "login");
        
        // Sign
        let auth = crate::gate::auth::GateAuth::new(
            self.api_key.clone(),
            self.secret.clone(),
        );
        let signature = auth.sign(&sign_string)?;

        Ok(json!({
            "time": timestamp,
            "channel": "spot.profile",
            "event": "subscribe",
            "api_key": self.api_key,
            "auth_key": self.api_key,
            "signature": signature
        }))
    }
}
```

---

### 阶段 7：测试与验证（1-2 小时）

#### 7.1 单元测试

在每个模块中添加单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_conversion() {
        let converter = GateSymbolConverter;
        
        // Spot: BTC/USDT → BTC_USDT
        let symbol = ParsedSymbol::spot("BTC", "USDT");
        assert_eq!(converter.to_exchange_id(&symbol), "BTC_USDT");
        
        // Back: BTC_USDT → BTC/USDT
        let parsed = converter.from_exchange_id("BTC_USDT", SymbolMarketType::Spot);
        assert_eq!(parsed.base, "BTC");
        assert_eq!(parsed.quote, "USDT");
    }

    #[test]
    fn test_ticker_parsing() {
        let data = serde_json::json!({
            "currency_pair": "BTC_USDT",
            "last": "50000",
            "highestBid": "49900",
            "lowestAsk": "50100",
        });

        let ticker = parse_ticker(&data).unwrap();
        assert_eq!(ticker.symbol, "BTC/USDT");
        assert_eq!(ticker.last.unwrap().0, dec!(50000));
    }
}
```

#### 7.2 集成测试

创建测试文件：

```bash
mkdir -p ccxt-exchanges/tests/gate
touch ccxt-exchanges/tests/gate/mod.rs
touch ccxt-exchanges/tests/gate/market_data.rs
touch ccxt-exchanges/tests/gate/websocket.rs
```

```rust
// tests/gate/market_data.rs
use ccxt_exchanges::gate::Gate;
use ccxt_core::ExchangeConfig;

#[tokio::test]
async fn test_fetch_ticker() {
    let exchange = Gate::builder()
        .default_type(ccxt_core::types::common::default_type::DefaultType::Spot)
        .build()
        .unwrap();

    let ticker = exchange.fetch_ticker("BTC/USDT").await;
    assert!(ticker.is_ok());
    
    let ticker = ticker.unwrap();
    assert_eq!(ticker.symbol, "BTC/USDT");
    assert!(ticker.last.is_some());
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_create_order() {
    // TODO: Implement with real API credentials
}
```

#### 7.3 WebSocket 测试

**⚠️ 关键原则**：
1. **超时必须 panic**，不能静默失败
2. **必须验证收到数据**，不能只验证订阅成功
3. **使用 short timeout 快速失败**（10秒）

```rust
#[tokio::test]
async fn test_watch_ticker() {
    let exchange = Gate::builder()
        .default_type(DefaultType::Spot)
        .build()
        .unwrap();

    // 1. Connect
    exchange.ws_connect().await.expect("Should connect");

    // 2. Subscribe
    let mut stream = exchange
        .watch_ticker("BTC/USDT")
        .await
        .expect("Should subscribe"); // Fail immediately if subscription fails

    // 3. Wait for data (with timeout)
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        stream.next()
    ).await;

    // 4. Verify received data
    match result {
        Ok(Some(ticker)) => {
            assert_eq!(ticker.symbol, "BTC/USDT");
            assert!(ticker.last.is_some());
            println!("✓ Ticker received: {:?}", ticker.last);
        }
        Ok(None) => panic!("Stream ended unexpectedly"),
        Err(_) => panic!("Ticker timeout after 10s - no messages received"),
    }
}
```

---

## 四、常见陷阱与解决方案

### 4.1 Symbol 格式错误

**问题**：假设所有交易所使用相同的 Symbol 格式。

**解决方案**：
```rust
// ❌ 错误：假设都是 BTCUSDT
let symbol = "BTCUSDT";

// ✅ 正确：查看实际 API 文档
// Binance: BTCUSDT
// Gate:    BTC_USDT
// OKX:     BTC-USDT
```

**调试方法**：
```rust
// 打印 API 返回的实际 symbol 格式
println!("[API] Symbol format: {}", data["symbol"]);
```

### 4.2 WebSocket 消息路由失败

**问题现象**：WebSocket 连接成功，订阅成功，但收不到数据。

**根本原因**：`extract_channel()` 无法从消息中提取正确的频道键。

**调试方法**：
```rust
// 添加原始消息日志
#[cfg(test)]
println!("[WS-RAW-RECV] {}", text);
```

**实际案例**（Gate K线）：
```json
{
  "channel": "spot.candlesticks",
  "result": {
    "n": "1m_BTC_USDT",  // ← symbol 在 n 字段！
    "t": "1777143120"
  }
}
```

**修复**：
```rust
fn extract_channel(&self, msg: &Value) -> Option<String> {
    let result = &msg["result"];
    
    // Try all possible fields
    if let Some(n) = result["n"].as_str() {
        // Handle "1m_BTC_USDT" format
        if let Some(pos) = n.find('_') {
            return Some(n[pos + 1..].replace('_', "/"));
        }
    }
    // ... other fields
}
```

### 4.3 现货与合约数据格式差异

**问题**：同一个解析函数处理现货和合约数据时失败。

**实际案例**（Gate OHLCV）：
- 现货：`result` 是对象 `{"t": "...", "v": "..."}`
- 合约：`result` 是数组 `[{"t": ..., "v": ...}]`

**解决方案**：
```rust
pub fn parse_ws_ohlcv(data: &Value) -> Result<Ohlcv> {
    // Handle both object and array formats
    let candle_data = if data.is_array() {
        data.as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::parse("Empty candle array"))?
    } else {
        data
    };

    // Unified parsing logic
    let timestamp = if let Some(ts) = candle_data["t"].as_str() {
        ts.parse::<i64>()? * 1000  // Spot: string
    } else if let Some(ts) = candle_data["t"].as_i64() {
        ts * 1000  // Futures: number
    } else {
        return Err(Error::parse("Missing timestamp"));
    };
    
    // ... parse other fields
}
```

### 4.4 合约市价单参数错误

**问题**：Gate 合约市价单下单失败。

**Gate 合约要求**：
- `price` 必须为 `"0"`
- `tif` 必须为 `"ioc"` (Immediate or Cancel)
- `size` 的正负表示方向（正数=买入，负数=卖出）
- **不要传 `side` 字段**

**正确实现**：
```rust
if request.order_type == OrderType::Market {
    body.insert("price".to_string(), "0".to_string().into());
    body.insert("tif".to_string(), "ioc".to_string().into());
    
    // Size sign indicates direction
    let size = match request.side {
        OrderSide::Buy => request.amount.to_string(),
        OrderSide::Sell => format!("-{}", request.amount),
    };
    body.insert("size".to_string(), size.into());
} else {
    // Limit order
    body.insert("price".to_string(), request.price.to_string().into());
    body.insert("size".to_string(), request.amount.to_string().into());
    body.insert("side".to_string(), request.side.to_string().into());
}
```

### 4.5 测试网配置错误

**问题**：合约测试使用测试网时报认证失败。

**根本原因**：创建测试实例时未显式配置 `testnet` 标志。

**正确配置**：
```rust
// ❌ 错误：缺少 testnet 配置
fn create_test_swap() -> Gate {
    Gate::builder()
        .default_type(DefaultType::Swap)
        .api_key("xxx")
        .secret("xxx")
        .build()
        .unwrap()
}

// ✅ 正确：显式启用 testnet
fn create_test_swap() -> Gate {
    Gate::builder()
        .default_type(DefaultType::Swap)
        .testnet(true)  // ← 必须添加
        .api_key("xxx")
        .secret("xxx")
        .build()
        .unwrap()
}
```

**测试网限制处理**：
```rust
match exchange.create_order(&request).await {
    Ok(order) => { /* verify order */ }
    Err(e) if e.to_string().contains("INVALID_PROTOCOL") => {
        eprintln!("[SKIP] Testnet doesn't support FOK orders");
        return;  // Gracefully skip
    }
    Err(e) => panic!("Unexpected error: {}", e),
}
```

---

## 五、测试规范

### 5.1 测试分类

```
tests/<exchange>/
├── market_data.rs       # 公开行情测试（不需要 API Key）
├── websocket.rs         # WebSocket 实时数据测试
├── spot/                # 现货测试
│   ├── market_data.rs
│   └── order_types.rs
├── swap/                # 合约测试
│   ├── market_data.rs
│   └── order_types.rs
└── conformance/         # 一致性测试
    └── capabilities.rs
```

### 5.2 测试命名规范

```rust
// 公开 API 测试
#[tokio::test]
async fn test_public_ticker() { }
async fn test_public_orderbook() { }
async fn test_public_ohlcv() { }

// 私有 API 测试（需要 API Key）
#[tokio::test]
#[ignore]
async fn test_private_balance() { }
async fn test_private_order() { }

// WebSocket 测试
#[tokio::test]
async fn test_ws_ticker() { }
async fn test_ws_orderbook() { }
async fn test_ws_ohlcv() { }

// 边界场景测试
#[test]
async fn test_edge_empty_response() { }
async fn test_edge_invalid_symbol() { }
```

### 5.3 集中测试命令

```bash
# 运行特定交易所的所有测试
cargo test -p ccxt-exchanges gate

# 仅运行公开 API 测试
cargo test -p ccxt-exchanges test_public

# 仅运行 WebSocket 测试
cargo test -p ccxt-exchanges test_ws

# 运行边界场景测试
cargo test -p ccxt-exchanges test_edge

# 运行 conformance 测试
cargo test -p ccxt-exchanges conformance

# 运行文档测试
cargo test -p ccxt-exchanges --doc

# 全量测试
cargo test --workspace
```

### 5.4 调试日志规范

**使用 `println!` 而非 `info!`**：
```rust
// ❌ 错误：测试环境日志系统未初始化
info!("Subscribing to {}", channel);

// ✅ 正确：直接打印到终端
println!("[TEST] Subscribing to {}", channel);
println!("[WS-RAW-RECV] {}", message);
```

**推荐的调试日志格式**：
```rust
#[cfg(test)]
println!("[DEBUG Gate public_get] Base URL: {}", base_url);
#[cfg(test)]
println!("[DEBUG Gate public_get] Full URL: {}", full_url);
```

---

## 六、验证清单

完成接入后，使用以下清单验证：

### 6.1 功能验证

#### P0 - 核心接口（必须）

- [ ] 可以创建交易所实例（现货）
- [ ] `fetch_ticker` - 可以获取行情数据
- [ ] `fetch_order_book` - 可以获取订单簿
- [ ] `fetch_ohlcv` - 可以获取 K线数据
- [ ] `fetch_markets` - 可以获取市场列表

#### P1 - 交易接口（如有交易功能）

- [ ] `create_order` - 可以创建订单（限价/市价）
- [ ] `cancel_order` - 可以取消订单
- [ ] `fetch_order` - 可以查询单个订单
- [ ] `fetch_open_orders` - 可以查询未结订单
- [ ] `fetch_history_orders` - 可以查询历史订单
  - ⚠️ **注意**：不是 `fetch_orders`，不要合并 open+closed
- [ ] `fetch_balance` - 可以获取账户余额

#### P2 - WebSocket（如支持）

- [ ] WebSocket 可以连接
- [ ] `watch_ticker` - 可以订阅行情
- [ ] `watch_order_book` - 可以订阅订单簿
- [ ] `watch_ohlcv` - 可以订阅 K线

### 6.2 代码质量验证

- [ ] 所有测试通过：`cargo test --workspace`
- [ ] 文档测试通过：`cargo test --doc`
- [ ] 无 Clippy 警告：`cargo clippy --workspace -- -D warnings`
- [ ] 代码格式化通过：`cargo fmt --all -- --check`

### 6.3 文档验证

- [ ] 模块文档完整
- [ ] 公共 API 有文档注释
- [ ] 示例代码可运行
- [ ] 错误处理有文档

---

## 七、参考实现

### 7.1 推荐参考顺序

1. **Gate** - 最佳入门参考
   - 目录结构清晰
   - 完整的 REST + WebSocket 实现
   - 有详细的实战经验文档（`docs/gate-integration-experience.md`）

2. **HyperLiquid** - 简单参考
   - 目录结构简单
   - EIP-712 签名认证

3. **OKX** - 标准参考
   - 完整的 WebSocket 认证策略
   - 支持 Margin trading

4. **Binance** - 复杂参考
   - 多市场类型支持
   - 复杂的认证机制

### 7.2 参考路径

```
ccxt-exchanges/src/
├── gate/              # 最佳参考（推荐先看）
│   ├── mod.rs         # 结构体 + Options + Builder
│   ├── auth/          # HMAC-SHA512 认证
│   ├── core/          # Symbol 转换 + Builder
│   ├── network/       # 端点路由
│   ├── parser/        # REST + WS 解析分离
│   ├── rest/          # REST API 实现
│   ├── ws/            # WebSocket 实现（新架构）
│   └── impls/         # Trait 实现
│
├── hyperliquid/       # 简单参考
├── okx/               # 标准参考
└── common/            # 共享构建块
    ├── parser_helpers.rs  # 解析辅助函数
    ├── metadata.rs        # 元数据 helper
    └── environment.rs     # URL 解析器
```

### 7.3 相关文档

- [Gate.io 接入经验总结](./gate-integration-experience.md) - 详细的实战经验和陷阱
- [rust-exchanges README](../README.md) - 项目概述

---

## 附录：快速检查表

### 接入前

- [ ] 已阅读 API 文档（REST + WebSocket）
- [ ] 已确认认证方式（HMAC/ RSA /Ed25519）
- [ ] 已收集端点 URL（生产 + 测试网）
- [ ] 已确认 Symbol 格式

### 实现中

- [ ] 基础模块完成（mod.rs + Builder + 端点路由）
- [ ] Symbol 转换器完成并测试
- [ ] 认证模块完成并测试
- [ ] 解析器完成（REST + WebSocket 分离）
- [ ] REST API 完成（公开 + 私有）
- [ ] WebSocket 完成（使用 GenericWsClient）
- [ ] Trait 实现完成（MarketData、Trading）

### 测试

- [ ] 单元测试通过
- [ ] 集成测试通过（公开 API）
- [ ] WebSocket 测试通过
- [ ] 全量测试通过：`cargo test --workspace`
- [ ] 无 Clippy 警告
- [ ] 代码格式化通过

### 完成

- [ ] 文档注释完善
- [ ] 示例代码可运行
- [ ] 提交代码并创建 PR

---

**文档版本**: 4.0  
**最后更新**: 2026-04-25  
**维护者**: CCXT Rust Team  
**更新说明**:
- v4.0: 重写文档，整合 Gate 实战经验
  - ✅ 明确模块实现优先级（基础 → 核心 → 扩展）
  - ✅ 提供清晰的执行步骤和时间估算
  - ✅ 整合 Gate 实战中的 5 个常见陷阱
  - ✅ 完善测试规范和验证清单
  - ✅ 删除过时内容（如旧的 WebSocket 架构）
- v3.1: 新增订单系统改造经验总结
- v3.0: 强调 `fetch_orders` 合并 open+closed 是错误做法

# <center> rust-exchanges </center>

> **致谢**: 本项目代码参考于 [https://github.com/Praying/ccxt-rust](https://github.com/Praying/ccxt-rust)

___

[![Rust](https://img.shields.io/badge/rust-1.91%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust CI](https://github.com/Praying/rust-exchanges/actions/workflows/rust.yml/badge.svg)](https://github.com/Praying/rust-exchanges/actions/workflows/rust.yml)
[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://docs.rs/rust-exchanges)

CCXT 库的专业级 Rust 实现，提供统一、类型安全的接口访问主流加密货币交易所。

[English](README.md) | [简体中文](README_CN.md)

## 🎯 支持的交易所

| 交易所             | 市场数据 | 交易 API | WebSocket |
|-----------------|------|--------|-----------|
| **Binance**     | ✅    | ✅      | ✅         |
| **Bitget**      | ✅    | ✅      | ✅         |
| **Hyperliquid** | ✅    | ✅      | ✅         |
| **OKX**         | ✅    | ✅      | ✅         |
| **Bybit**       | ✅    | ✅      | ✅         |

> **图例**: ✅ 已支持, 🚧 开发中

## 🌟 核心特性

- **🛡️ 类型安全与异步**: 基于 `Tokio` 和 `rust_decimal` 构建，确保高性能与金融计算安全。
- **🔄 统一接口**: 所有交易所均实现统一的 `Exchange` trait。
- **⚡ 实时数据**: 强大的 WebSocket 支持，具备自动重连功能。
- **📦 功能全面**:
  - **行情**: Ticker, 深度图, K线 (OHLCV), 成交记录。
  - **交易**: 现货, 杠杆, 合约, 批量下单, OCO。
  - **账户**: 余额查询, 资金划转, 杠杆管理。

## 🚀 快速开始

### 安装

```bash
cargo add rust-exchanges
```

### 基本用法

```rust
use ccxt_exchanges::binance::Binance;
use ccxt_core::exchange::Exchange;
use rust_decimal_macros::dec;
use ccxt_core::types::{OrderType, OrderSide};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // 1. 初始化 (建议使用环境变量)
    dotenv::dotenv().ok();
    let exchange = Binance::builder()
            .api_key(std::env::var("BINANCE_API_KEY").ok())
            .secret(std::env::var("BINANCE_SECRET").ok())
        .build()?;

  // 2. 获取行情
    exchange.fetch_markets().await?;
    let ticker = exchange.fetch_ticker("BTC/USDT").await?;
    println!("BTC/USDT 价格: {:?}", ticker.last);

  // 3. 下单 (如提供了 API Key)
  if exchange.has_private_api() {
    let order = exchange.create_order(
      "BTC/USDT",
      OrderType::Limit,
      OrderSide::Buy,
      dec!(0.001),
      Some(dec!(50000)),
    ).await?;
    println!("下单成功: {}", order.id);
  }

  Ok(())
}
```

更多 WebSocket 和高级用法示例请查看 [`examples/`](examples/) 目录。

## 🏗️ 架构

项目采用模块化工作空间结构：

- **`ccxt-core`**: 定义统一的 `Exchange` 和 `WsExchange` trait、标准类型及错误处理逻辑。
- **`ccxt-exchanges`**: 包含具体交易所的实现 (Binance, OKX 等)。

## 📋 交易所 API 方法列表

### 1. 行情数据模块 (MarketData)

| 方法名 | 说明 | 返回数据 |
|--------|------|----------|
| `fetch_markets` | 获取所有交易对信息 | `Vec<Market>` - 交易对列表 |
| `load_markets` | 加载并缓存交易对 | `Arc<HashMap<String, Arc<Market>>>` - 交易对映射 |
| `reload_markets` | 强制刷新交易对缓存 | 同上 |
| `market` | 获取指定交易对信息 | `Arc<Market>` - 单个交易对详情 |
| `markets` | 获取缓存的所有交易对 | 交易对映射 |
| `has_symbol` | 检查交易对是否存在且可用 | `bool` |
| `fetch_ticker` | 获取单个交易对行情 | `Ticker` - 最新价格、涨跌幅、成交量等 |
| `fetch_tickers` | 获取多个交易对行情 | `Vec<Ticker>` - 行情列表 |
| `fetch_all_tickers` | 获取所有交易对行情 | 同上 |
| `fetch_order_book` | 获取订单簿 | `OrderBook` - 买卖盘口 |
| `fetch_order_book_with_depth` | 获取指定深度订单簿 | 同上 |
| `fetch_trades` | 获取最新成交记录 | `Vec<Trade>` - 成交列表 |
| `fetch_ohlcv` | 获取K线数据 | `Vec<Ohlcv>` - OHLCV蜡烛图数据 |

### 2. 交易模块 (Trading)

| 方法名 | 说明 | 返回数据 |
|--------|------|----------|
| `create_order` | 创建订单 | `Order` - 订单详情 |
| `market_buy` | 市价买入 | 同上 |
| `market_sell` | 市价卖出 | 同上 |
| `limit_buy` | 限价买入 | 同上 |
| `limit_sell` | 限价卖出 | 同上 |
| `cancel_order` | 取消订单 | `Order` - 已取消订单 |
| `cancel_all_orders` | 取消所有订单 | `Vec<Order>` - 已取消订单列表 |
| `fetch_order` | 查询订单详情 | `Order` - 订单状态 |
| `fetch_open_orders` | 查询未成交订单 | `Vec<Order>` - 订单列表 |
| `fetch_history_orders` | 查询历史订单 | 同上 |

### 3. 账户模块 (Account)

| 方法名 | 说明 | 返回数据 |
|--------|------|----------|
| `fetch_balance` | 获取账户余额 | `Balance` - 所有币种余额 |
| `fetch_balance_with_params` | 带参数获取余额 | 同上 |
| `get_balance` | 获取指定币种余额 | `BalanceEntry` - 单个币种详情 |
| `fetch_account_trades` | 获取成交历史 | `Vec<Trade>` - 成交记录 |
| `fetch_account_trades_since` | 按时间获取成交历史 | 同上 |

### 4. 杠杆/合约模块 (Margin)

| 方法名 | 说明 | 返回数据 |
|--------|------|----------|
| `fetch_positions` | 获取所有持仓 | `Vec<Position>` - 持仓列表 |
| `fetch_position` | 获取指定交易对持仓 | `Position` - 单个持仓详情 |
| `set_leverage` | 设置杠杆倍数 | `()` |
| `get_leverage` | 获取当前杠杆 | `u32` |
| `set_margin_mode` | 设置保证金模式 | `()` |
| `fetch_funding_rate` | 获取资金费率 | `FundingRate` - 当前资金费率 |
| `fetch_funding_rates` | 获取多个交易对资金费率 | `Vec<FundingRate>` |
| `fetch_funding_rate_history` | 获取资金费率历史 | `Vec<FundingRateHistory>` |

### 5. 资金模块 (Funding)

| 方法名 | 说明 | 返回数据 |
|--------|------|----------|
| `fetch_deposit_address` | 获取充值地址 | `DepositAddress` - 地址信息 |
| `fetch_deposit_address_on_network` | 获取指定网络充值地址 | 同上 |
| `withdraw` | 提现 | `Transaction` - 提现记录 |
| `transfer` | 账户间划转 | `Transfer` - 划转记录 |
| `fetch_deposits` | 获取充值历史 | `Vec<Transaction>` - 充值记录列表 |
| `fetch_withdrawals` | 获取提现历史 | `Vec<Transaction>` - 提现记录列表 |

### 6. 元数据方法 (PublicExchange)

| 方法名 | 说明 | 返回数据 |
|--------|------|----------|
| `id` | 交易所ID | `&str` - 如 "binance" |
| `name` | 交易所名称 | `&str` - 如 "Binance" |
| `version` | API版本 | `&str` |
| `certified` | 是否CCXT认证 | `bool` |
| `capabilities` | 功能支持列表 | `ExchangeCapabilities` |
| `timeframes` | 支持的K线周期 | `Vec<Timeframe>` |
| `rate_limit` | 速率限制 | `u32` |
| `has_websocket` | 是否支持WebSocket | `bool` |

## 🏢 各交易所具体实现方法

### Binance (币安)

#### 现货交易 (spot.rs)
| 方法名 | 说明 |
|--------|------|
| `create_order` | 创建订单（支持限价、市价、止损、止盈等） |
| `cancel_order` | 取消单个订单 |
| `cancel_all_orders` | 取消所有订单 |
| `cancel_orders` | 批量取消订单 |
| `fetch_order` | 查询订单详情 |
| `fetch_open_orders` | 查询未成交订单 |
| `fetch_history_orders` | 查询历史订单 |
| `fetch_orders` | 查询所有订单 |
| `create_stop_loss_order` | 创建止损订单 |
| `create_take_profit_order` | 创建止盈订单 |

#### 账户管理 (account.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_balance` | 获取账户余额（支持多账户类型） |
| `fetch_balance_with_params` | 带参数获取余额 |
| `fetch_cross_margin_balance` | 获取全仓杠杆余额 |
| `fetch_isolated_margin_balance` | 获取逐仓杠杆余额 |
| `fetch_account_trades` | 获取成交历史 |
| `fetch_my_recent_trades` | 获取最近成交 |
| `fetch_currencies` | 获取币种信息 |
| `fetch_trading_fee` | 获取交易手续费率 |
| `fetch_trading_fees` | 获取所有交易对手续费 |
| `create_listen_key` | 创建用户数据流密钥 |
| `refresh_listen_key` | 刷新用户数据流密钥 |
| `delete_listen_key` | 删除用户数据流密钥 |

#### 杠杆交易 (margin.rs)
| 方法名 | 说明 |
|--------|------|
| `borrow_cross_margin` | 全仓杠杆借币 |
| `borrow_isolated_margin` | 逐仓杠杆借币 |
| `repay_cross_margin` | 全仓杠杆还币 |
| `repay_isolated_margin` | 逐仓杠杆还币 |
| `fetch_margin_loan_history` | 获取借币历史 |
| `fetch_margin_repay_history` | 获取还币历史 |
| `fetch_margin_interest_history` | 获取利息历史 |
| `fetch_margin_force_liquidation` | 获取强平记录 |
| `fetch_margin_adjustments` | 获取保证金调整记录 |
| `fetch_cross_margin_max_borrowable` | 获取最大可借额度 |

#### 合约交易 (futures/)
| 方法名 | 说明 |
|--------|------|
| `fetch_position` | 获取持仓详情 |
| `fetch_positions` | 获取所有持仓 |
| `fetch_positions_risk` | 获取持仓风险 |
| `set_leverage` | 设置杠杆 |
| `fetch_leverage` | 获取杠杆 |
| `fetch_leverages` | 获取所有杠杆设置 |
| `fetch_leverage_bracket` | 获取杠杆档位 |
| `fetch_leverage_tiers` | 获取杠杆层级 |
| `set_margin_mode` | 设置保证金模式 |
| `set_position_mode` | 设置持仓模式（单向/双向） |
| `fetch_position_mode` | 获取持仓模式 |
| `modify_isolated_position_margin` | 修改逐仓保证金 |
| `fetch_funding_rate` | 获取资金费率 |
| `fetch_funding_rates` | 获取所有资金费率 |
| `fetch_funding_rate_history` | 获取资金费率历史 |
| `fetch_funding_history` | 获取资金结算历史 |

#### 交割合约 (delivery.rs)
| 方法名 | 说明 |
|--------|------|
| `set_position_mode_dapi` | 设置DAPI持仓模式 |
| `fetch_position_mode_dapi` | 获取DAPI持仓模式 |
| `fetch_dapi_account` | 获取DAPI账户信息 |
| `fetch_dapi_income` | 获取DAPI收益记录 |
| `fetch_dapi_commission_rate` | 获取DAPI手续费率 |
| `fetch_dapi_adl_quantile` | 获取ADL量化值 |
| `fetch_dapi_force_orders` | 获取强平订单 |

#### 资金管理 (funding.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_deposit_address` | 获取充值地址 |
| `withdraw` | 提现 |
| `transfer` | 账户间划转 |
| `fetch_deposits` | 获取充值历史 |
| `fetch_withdrawals` | 获取提现历史 |
| `fetch_deposit_withdraw_fees` | 获取充提手续费 |
| `fetch_funding_wallet` | 获取资金钱包 |
| `fetch_asset_dividend` | 获取资产分红 |

---

### OKX

#### 行情数据 (market_data.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_markets` | 获取交易对 |
| `load_markets` | 加载交易对 |
| `fetch_ticker` | 获取行情 |
| `fetch_tickers` | 获取多个行情 |
| `fetch_order_book` | 获取订单簿 |
| `fetch_trades` | 获取成交记录 |
| `fetch_ohlcv` | 获取K线数据 |

#### 交易 (trading.rs)
| 方法名 | 说明 |
|--------|------|
| `create_order` | 创建订单 |
| `cancel_order` | 取消订单 |
| `fetch_order` | 查询订单 |
| `fetch_open_orders` | 查询未成交订单 |
| `fetch_history_orders` | 查询历史订单 |

#### 账户 (account.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_balance` | 获取余额 |
| `fetch_account_trades` | 获取成交历史 |

#### 合约 (futures/)
| 方法名 | 说明 |
|--------|------|
| `fetch_position_impl` | 获取持仓 |
| `fetch_positions_impl` | 获取所有持仓 |
| `set_leverage_impl` | 设置杠杆 |
| `get_leverage_impl` | 获取杠杆 |
| `set_margin_mode_impl` | 设置保证金模式 |
| `fetch_funding_rate_impl` | 获取资金费率 |
| `fetch_funding_rates_impl` | 获取多个资金费率 |
| `fetch_funding_rate_history_impl` | 获取资金费率历史 |

---

### Bybit

#### 行情数据 (market_data.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_markets` | 获取交易对 |
| `load_markets` | 加载交易对 |
| `fetch_ticker` | 获取行情 |
| `fetch_tickers` | 获取多个行情 |
| `fetch_order_book` | 获取订单簿 |
| `fetch_trades` | 获取成交记录 |
| `fetch_ohlcv` | 获取K线数据 |

#### 交易 (trading.rs)
| 方法名 | 说明 |
|--------|------|
| `create_order` | 创建订单 |
| `cancel_order` | 取消订单 |
| `fetch_order` | 查询订单 |
| `fetch_open_orders` | 查询未成交订单 |
| `fetch_history_orders` | 查询历史订单 |

#### 账户 (account.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_balance` | 获取余额 |
| `fetch_account_trades` | 获取成交历史 |

---

### Bitget

#### 行情数据 (market_data.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_markets` | 获取交易对 |
| `load_markets` | 加载交易对 |
| `fetch_ticker` | 获取行情 |
| `fetch_tickers` | 获取多个行情 |
| `fetch_order_book` | 获取订单簿 |
| `fetch_trades` | 获取成交记录 |
| `fetch_ohlcv` | 获取K线数据 |

#### 交易 (trading.rs)
| 方法名 | 说明 |
|--------|------|
| `create_order` | 创建订单 |
| `cancel_order` | 取消订单 |
| `fetch_order` | 查询订单 |
| `fetch_open_orders` | 查询未成交订单 |
| `fetch_history_orders` | 查询历史订单 |

#### 账户 (account.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_balance` | 获取余额 |
| `fetch_account_trades` | 获取成交历史 |

#### 合约 (futures/)
| 方法名 | 说明 |
|--------|------|
| `fetch_position_impl` | 获取持仓 |
| `fetch_positions_impl` | 获取所有持仓 |
| `set_leverage_impl` | 设置杠杆 |
| `get_leverage_impl` | 获取杠杆 |
| `set_margin_mode_impl` | 设置保证金模式 |
| `fetch_funding_rate_impl` | 获取资金费率 |
| `fetch_funding_rates_impl` | 获取多个资金费率 |
| `fetch_funding_rate_history_impl` | 获取资金费率历史 |

---

### Hyperliquid

#### 行情数据 (market_data.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_markets` | 获取交易对 |
| `load_markets` | 加载交易对 |
| `fetch_ticker` | 获取行情 |
| `fetch_tickers` | 获取多个行情 |
| `fetch_order_book` | 获取订单簿 |
| `fetch_trades` | 获取成交记录 |
| `fetch_ohlcv` | 获取K线数据 |
| `fetch_funding_rate` | 获取资金费率 |

#### 交易 (trading.rs)
| 方法名 | 说明 |
|--------|------|
| `create_order` | 创建订单 |
| `cancel_order` | 取消订单 |
| `cancel_all_orders` | 取消所有订单 |
| `set_leverage` | 设置杠杆 |

#### 账户 (account.rs)
| 方法名 | 说明 |
|--------|------|
| `fetch_balance` | 获取余额 |
| `fetch_positions` | 获取持仓 |
| `fetch_open_orders` | 查询未成交订单 |

## 🚩 功能标志 (Feature Flags)

| 标志           | 说明                  | 默认开启 |
|--------------|---------------------|------|
| `rest`       | REST API 支持         | ✅    |
| `websocket`  | WebSocket 支持        | ✅    |
| `rustls-tls` | 使用 RustLS (推荐)      | ✅    |
| `native-tls` | 使用 OpenSSL/系统原生 TLS | ❌    |

## 🛠️ 开发与测试

```bash
# 运行全部测试（仅公共/部分 Mock 场景，不依赖 API 密钥）
cargo test

# 为测试配置环境变量（建议复制 .env.example 为 .env 并填写以下字段）
# - SKIP_PRIVATE_TESTS=false      # 允许执行私有 API 测试（如不想跑私有测试可设为 true）
# - BINANCE_API_KEY=...          # Binance API 密钥（测试网或生产环境，推荐直接把测试网密钥写进来）
# - BINANCE_API_SECRET=...       # Binance API 密钥
# - USE_TESTNET=true             # 可选：如使用测试网，配合上面的测试网密钥使用

# 仅运行 Binance 公共场景集成测试
cargo test -p ccxt-exchanges --test binance_integration_test public_api::

# 仅运行 Binance 私有场景（带密钥）测试
cargo test -p ccxt-exchanges --test binance_advanced_test test_binance_private_

# 仅运行 Binance 内部转账等私有集成测试
cargo test --test test_transfer test_binance_private_

# 代码检查
cargo clippy --all-targets -- -D warnings

# 生成文档
cargo doc --open
```

## 📝 许可证与支持

MIT License. 详见 [LICENSE](LICENSE).

- **问题反馈**: [GitHub Issues](https://github.com/Praying/rust-exchanges/issues)
- **文档**: [docs.rs](https://docs.rs/rust-exchanges)

## ⚠️ 免责声明

本项目仅供学习和研究使用。作者和贡献者不对因使用本软件而产生的任何财务损失或损害负责。加密货币交易风险极高，请谨慎交易。

---
**状态**: 🚧 开发中 (v0.1.4) | **捐赠 (BSC)**: `0x8e5d858f92938b028065d39450421d0e080d15f7`

//! Symbol 转换示例 - 展示统一 Symbol 与各交易所格式的转换
//!
//! 本示例展示如何将统一的 CCXT Symbol 转换为各交易所特定的格式
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --example symbol_conversion_demo
//! ```

#![allow(clippy::disallowed_methods)]

use ccxt_core::symbol::{ExpiryDate, ParsedSymbol, SymbolContext, SymbolConverter};

// 引入 common 模块的宏
#[macro_use]
mod common;

fn main() {
    log_title!("Symbol 转换示例");

    // =================================================================
    // 1. 统一 Symbol 格式说明
    // =================================================================
    log_section!("统一 Symbol 格式 (CCXT Standard)");
    log_info!("现货: BASE/QUOTE");
    log_info!("正向永续: BASE/QUOTE:QUOTE (例如: BTC/USDT:USDT)");
    log_info!("反向永续: BASE/QUOTE:BASE (例如: BTC/USD:BTC)");
    log_info!("正向期货: BASE/QUOTE:QUOTE-YYMMDD (例如: BTC/USDT:USDT-241231)");
    log_info!("反向期货: BASE/QUOTE:BASE-YYMMDD (例如: BTC/USD:BTC-241231)");

    // =================================================================
    // 2. Binance Symbol 转换
    // =================================================================
    log_section!("Binance Symbol 转换");
    {
        use ccxt_exchanges::binance::symbol::BinanceSymbolConverter;

        // Spot
        let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let binance_spot = BinanceSymbolConverter::to_exchange_id(&spot);
        log_success!("现货 BTC/USDT -> {}", binance_spot);

        // Linear Swap
        let linear_swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let binance_linear = BinanceSymbolConverter::to_exchange_id(&linear_swap);
        log_success!("正向永续 BTC/USDT:USDT -> {}", binance_linear);

        // Inverse Swap
        let inverse_swap = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        let binance_inverse = BinanceSymbolConverter::to_exchange_id(&inverse_swap);
        log_success!("反向永续 BTC/USD:BTC -> {}", binance_inverse);

        // Futures
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let futures = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        let binance_futures = BinanceSymbolConverter::to_exchange_id(&futures);
        log_success!("期货 BTC/USDT:USDT-241231 -> {}", binance_futures);

        // 检查是否为永续合约
        log_info!(
            "BTCUSD_PERP 是永续合约: {}",
            BinanceSymbolConverter::is_perpetual("BTCUSD_PERP")
        );
        log_info!(
            "BTCUSDT_241231 是期货: {}",
            BinanceSymbolConverter::is_futures("BTCUSDT_241231")
        );
    }

    // =================================================================
    // 3. OKX Symbol 转换
    // =================================================================
    log_section!("OKX Symbol 转换");
    {
        use ccxt_exchanges::okx::core::symbol::OkxSymbolConverter;

        // Spot
        let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let okx_spot = OkxSymbolConverter::to_exchange_id(&spot);
        log_success!("现货 BTC/USDT -> {}", okx_spot);

        // Linear Swap
        let linear_swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let okx_linear = OkxSymbolConverter::to_exchange_id(&linear_swap);
        log_success!("正向永续 BTC/USDT:USDT -> {}", okx_linear);

        // Inverse Swap
        let inverse_swap = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        let okx_inverse = OkxSymbolConverter::to_exchange_id(&inverse_swap);
        log_success!("反向永续 BTC/USD:BTC -> {}", okx_inverse);

        // Futures
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let futures = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        let okx_futures = OkxSymbolConverter::to_exchange_id(&futures);
        log_success!("期货 BTC/USDT:USDT-241231 -> {}", okx_futures);

        // 检查市场类型
        log_info!(
            "BTC-USDT-SWAP 是永续: {}",
            OkxSymbolConverter::is_swap("BTC-USDT-SWAP")
        );
        log_info!(
            "BTC-USDT-241231 是期货: {}",
            OkxSymbolConverter::is_futures("BTC-USDT-241231")
        );
        log_info!(
            "BTC-USDT 是现货: {}",
            OkxSymbolConverter::is_spot("BTC-USDT")
        );
    }

    // =================================================================
    // 4. Bybit Symbol 转换
    // =================================================================
    log_section!("Bybit Symbol 转换");
    {
        use ccxt_exchanges::bybit::core::symbol::BybitSymbolConverter;

        // Spot
        let spot = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let bybit_spot = BybitSymbolConverter::to_exchange_id(&spot);
        log_success!("现货 BTC/USDT -> {}", bybit_spot);

        // Linear Swap
        let linear_swap = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let bybit_linear = BybitSymbolConverter::to_exchange_id(&linear_swap);
        log_success!("正向永续 BTC/USDT:USDT -> {}", bybit_linear);

        // Inverse Swap
        let inverse_swap = ParsedSymbol::inverse_swap("BTC".to_string(), "USD".to_string());
        let bybit_inverse = BybitSymbolConverter::to_exchange_id(&inverse_swap);
        log_success!("反向永续 BTC/USD:BTC -> {}", bybit_inverse);

        // Futures
        let expiry = ExpiryDate::new(24, 12, 31).unwrap();
        let futures = ParsedSymbol::futures(
            "BTC".to_string(),
            "USDT".to_string(),
            "USDT".to_string(),
            expiry,
        );
        let bybit_futures = BybitSymbolConverter::to_exchange_id(&futures);
        log_success!("期货 BTC/USDT:USDT-241231 -> {}", bybit_futures);
    }

    // =================================================================
    // 5. Bitget Symbol 转换
    // =================================================================
    log_section!("Bitget Symbol 转换");
    {
        use ccxt_exchanges::bitget::core::symbol::BitgetSymbolConverter;

        // Spot
        let unified_spot = "BTC/USDT";
        let bitget_spot = BitgetSymbolConverter::unified_to_exchange(unified_spot);
        let product_type = BitgetSymbolConverter::product_type_from_symbol(unified_spot);
        log_success!(
            "现货 BTC/USDT -> {} (产品类型: {})",
            bitget_spot,
            product_type
        );

        // Linear Swap
        let unified_linear = "BTC/USDT:USDT";
        let bitget_linear = BitgetSymbolConverter::unified_to_exchange(unified_linear);
        let product_type_linear = BitgetSymbolConverter::product_type_from_symbol(unified_linear);
        log_success!(
            "正向永续 BTC/USDT:USDT -> {} (产品类型: {})",
            bitget_linear,
            product_type_linear
        );

        // Inverse Swap
        let unified_inverse = "BTC/USD:BTC";
        let bitget_inverse = BitgetSymbolConverter::unified_to_exchange(unified_inverse);
        let product_type_inverse = BitgetSymbolConverter::product_type_from_symbol(unified_inverse);
        log_success!(
            "反向永续 BTC/USD:BTC -> {} (产品类型: {})",
            bitget_inverse,
            product_type_inverse
        );

        // 检查是否为合约
        log_info!(
            "BTC/USDT:USDT 是合约: {}",
            BitgetSymbolConverter::is_contract("BTC/USDT:USDT")
        );
        log_info!(
            "BTC/USDT 是现货: {}",
            BitgetSymbolConverter::is_spot("BTC/USDT")
        );
    }

    // =================================================================
    // 6. 各交易所支持情况总结
    // =================================================================
    log_section!("各交易所市场类型支持情况");

    log_subsection!("Binance");
    log_item!("现货 (Spot): 支持");
    log_item!("永续合约 (Swap): 支持 (正向/反向)");
    log_item!("期货 (Futures): 支持 (交割合约)");
    log_item!("期权 (Option): 支持 (欧洲式期权)");

    log_subsection!("OKX");
    log_item!("现货 (Spot): 支持");
    log_item!("永续合约 (Swap): 支持 (正向/反向)");
    log_item!("期货 (Futures): 支持");
    log_item!("期权 (Option): 支持");

    log_subsection!("Bybit");
    log_item!("现货 (Spot): 支持");
    log_item!("永续合约 (Swap): 支持 (正向/反向)");
    log_item!("期货 (Futures): 支持");
    log_item!("期权 (Option): 支持");

    log_subsection!("Bitget");
    log_item!("现货 (Spot): 支持");
    log_item!("永续合约 (Swap): 支持 (正向/反向)");
    log_item!("期货 (Futures): 支持");
    log_item!("期权 (Option): 不支持 (回退到现货)");

    log_subsection!("HyperLiquid");
    log_item!("现货 (Spot): 不支持");
    log_item!("永续合约 (Swap): 支持 (仅 USDC 正向合约)");
    log_item!("期货 (Futures): 不支持");
    log_item!("期权 (Option): 不支持");
    log_info!("说明: HyperLiquid 官方仅支持 USDC 保证金的线性合约，不支持反向合约");

    // =================================================================
    // 7. 关于不支持的类型
    // =================================================================
    log_section!("关于不支持的类型");
    log_info!("如果交易所不支持某种市场类型:");
    log_item!("1. Symbol 转换仍然可以工作 (格式转换)");
    log_item!("2. 但 API 调用可能会失败或返回错误");
    log_item!("3. 建议在使用前检查交易所的 capabilities");
    log_item!("4. 或者捕获 API 错误并优雅处理");

    log_warning!("示例: HyperLiquid 只支持永续合约");
    log_info!("如果尝试在 HyperLiquid 上使用现货 Symbol:");
    log_item!("- Symbol 转换: 可以工作");
    log_item!("- API 调用: 可能会失败，因为 HyperLiquid 没有现货市场");

    // =================================================================
    // 8. 使用统一 SymbolConverter trait
    // =================================================================
    log_section!("统一 SymbolConverter trait 使用示例");
    log_info!("新的 SymbolConverter trait 提供了统一的接口:");

    {
        use ccxt_exchanges::binance::symbol::BinanceSymbolConverter;

        let converter = BinanceSymbolConverter;

        // 使用 trait 方法转换
        let symbol = ParsedSymbol::spot("BTC".to_string(), "USDT".to_string());
        let exchange_id = converter.to_exchange_id(&symbol);
        log_success!("trait to_exchange_id: BTC/USDT -> {}", exchange_id);

        // 反向转换 (需要上下文)
        let ctx = SymbolContext::spot().with_base_quote("BTC", "USDT");
        let parsed = converter.from_exchange_id("BTCUSDT", ctx).unwrap();
        log_success!("trait from_exchange_id: BTCUSDT -> {}", parsed);

        // 推断式转换 (用于 WebSocket)
        let unified = converter.exchange_to_unified_inferred("BTCUSD_PERP");
        log_success!(
            "trait exchange_to_unified_inferred: BTCUSD_PERP -> {}",
            unified
        );

        // 市场类型检测
        let market_type = converter.detect_market_type("BTCUSDT_241231");
        log_info!("detect_market_type(BTCUSDT_241231): {:?}", market_type);
    }

    {
        use ccxt_exchanges::okx::core::symbol::OkxSymbolConverter;

        let converter = OkxSymbolConverter;

        // 使用统一接口
        let symbol = ParsedSymbol::linear_swap("BTC".to_string(), "USDT".to_string());
        let exchange_id = converter.to_exchange_id(&symbol);
        log_success!("OKX trait to_exchange_id: BTC/USDT:USDT -> {}", exchange_id);

        // 推断式转换
        let unified = converter.exchange_to_unified_inferred("BTC-USDT-SWAP");
        log_success!(
            "OKX trait exchange_to_unified_inferred: BTC-USDT-SWAP -> {}",
            unified
        );
    }

    log_complete!();
}

//! Gate.io PublicExchange trait implementation.

use ccxt_core::{exchange::ExchangeCapabilities, traits::PublicExchange, types::Timeframe};

use crate::gate::Gate;

impl PublicExchange for Gate {
    fn id(&self) -> &'static str {
        "gate"
    }

    fn name(&self) -> &'static str {
        "Gate.io"
    }

    fn version(&self) -> &'static str {
        "v4"
    }

    fn is_verified(&self) -> bool {
        false // Not yet verified
    }

    fn capabilities(&self) -> ExchangeCapabilities {
        // Gate.io supports spot trading with full market data, trading, and account capabilities
        // Contract (swap/futures) and margin capabilities will be enabled when implemented
        ExchangeCapabilities::builder()
            .market_data()
            .trading()
            .account()
            // TODO: Enable margin when contract API is implemented (stage 3)
            // .margin()
            .build()
    }

    fn timeframes(&self) -> &'static [Timeframe] {
        // Gate 现货官方 API 支持 12 个 timeframe
        // 参考: https://www.gate.com/docs/developers/apiv4/zh_CN/#现货市场-k-线图
        // GET /spot/candlesticks: 1s, 10s, 1m, 5m, 15m, 30m, 1h, 4h, 8h, 1d, 7d, 30d
        // 注意:
        // - 单次请求最大返回 1000 个点
        // - 30d 代表自然月，不是按30天对齐
        // - from/to 参数使用秒级 Unix 时间戳
        &[
            Timeframe::S1,
            Timeframe::M1,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H4,
            Timeframe::H8,
            Timeframe::D1,
            Timeframe::W1,
            Timeframe::Mon1,
        ]
    }

    fn requests_per_second(&self) -> u32 {
        // Gate.io rate limit: ~10 requests per second for spot
        if self.options.is_testnet() { 5 } else { 10 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::GateBuilder;

    #[test]
    fn test_gate_public_exchange_trait() {
        let gate = GateBuilder::default().build().unwrap();

        assert_eq!(gate.id(), "gate");
        assert_eq!(gate.name(), "Gate.io");
        assert_eq!(gate.version(), "v4");
        assert!(!gate.is_verified());
        assert_eq!(gate.requests_per_second(), 10);
    }

    #[test]
    fn test_gate_timeframes() {
        let gate = GateBuilder::default().build().unwrap();

        let timeframes = gate.timeframes();
        assert!(timeframes.contains(&Timeframe::M1));
        assert!(timeframes.contains(&Timeframe::H1));
        assert!(timeframes.contains(&Timeframe::D1));
    }
}

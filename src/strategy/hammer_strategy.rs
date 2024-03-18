use std::error::Error;
use std::sync::Arc;
use tinkoff_invest_api::tcs::{PortfolioResponse, Share};
use crate::order::OrderServiceSandboxImpl;
use crate::state::candle_state::CandleState;
use crate::strategy::strategy::{OpenedPattern, Strategy};

pub struct HummerStrategy {
    statistic: Arc<CandleState>,
    order_service: OrderServiceSandboxImpl,
    instrument: Share,
}

impl HummerStrategy {
    fn new(statistic: Arc<CandleState>, order_service: OrderServiceSandboxImpl, instrument: Share) -> Self {
        Self { statistic, order_service, instrument }
    }
}

impl Strategy for HummerStrategy {
    type Statistic = CandleState;

    async fn warm_up(&self, positions: PortfolioResponse) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    async fn update(&mut self) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    async fn signal_buy(&self, stat: &Self::Statistic) -> Vec<OpenedPattern> {
        todo!()
    }

    async fn check_pattern(&self, instrument: &Share, stat: &Self::Statistic) -> Option<OpenedPattern> {
        todo!()
    }

    async fn signal_sell(&self, stat: &Self::Statistic) -> Vec<OpenedPattern> {
        todo!()
    }
}

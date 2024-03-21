use std::error::Error;
use std::sync::{Arc, RwLock};
use tinkoff_invest_api::tcs::{OrderType, PortfolioResponse, Share};
use crate::order::{OrderService, OrderServiceHistBoxImpl, OrderServiceSandboxImpl};
use crate::state::candle_state::CandleState;
use crate::strategy::strategy::{OpenedPattern, Strategy};

pub struct HummerStrategy {
    statistic: Arc<CandleState>,
    order_service: OrderServiceHistBoxImpl,
    instrument: Share,
    opened_patterns: Vec<OpenedPattern>,
}

impl HummerStrategy {
    pub fn new(statistic: Arc<CandleState>, order_service: OrderServiceHistBoxImpl, instrument: Share) -> Self {
        Self { statistic, order_service, instrument, opened_patterns: Vec::new() }
    }
}

impl Strategy for HummerStrategy {
    type Statistic = CandleState;

    async fn warm_up(&self, positions: PortfolioResponse) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    async fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let orders_to_buy = self.signal_buy(&self.statistic).await;
        let orders_to_sell = self.signal_sell(&self.statistic).await;
        for order in orders_to_buy {
            let order_response = self.order_service.order_buy(
                order.figi.clone(),
                order.instrument_id.clone(),
                order.quantity.clone(),
                order.price.clone(),
                OrderType::Market,
            ).await;
            match order_response {
                Ok(_) => {
                    self.opened_patterns.push(order);
                }
                _ => eprint!("Error in orders_to_buy")
            }
        }
        let mut closed_orders_index = Vec::new();
        for order in orders_to_sell {
            let closed_order = self.order_service.order_sell(
                order.figi.clone(),
                order.instrument_id.clone(),
                order.quantity.clone(),
                order.price.clone(),
                OrderType::Market,
            ).await;
            if closed_order.is_ok() {
                let _closed_order = closed_order.unwrap().into_inner();
                let index_to_remove = self.opened_patterns.iter().position(|x| x.instrument_id == order.instrument_id).unwrap();
                closed_orders_index.push(index_to_remove);
            }
        }
        for order_index in closed_orders_index {
            self.opened_patterns.remove(order_index);
        }
        Ok(())
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

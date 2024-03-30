use std::sync::{Arc, RwLock};
use tinkoff_invest_api::tcs::{OrderType, PortfolioResponse, Quotation, Share};
use crate::service::order_service::{OrderService, OrderServiceSandboxImpl};
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};
use crate::strategy::strategy::{map_position_to_pattern, OpenedPattern, Strategy};

// стратегия -- купить дешевле, продать дороже. Для обкатки модели.
pub struct FirstStrategy {
    statistic: Arc<LastPriceState>,
    // order_service: Arc<OrderServiceImpl>,
    order_service: OrderServiceSandboxImpl,
    instruments: Vec<Share>,
    opened_patterns: RwLock<Vec<OpenedPattern>>,
}

impl FirstStrategy {
    pub fn new(statistic: Arc<LastPriceState>, order_service: OrderServiceSandboxImpl, instruments: Vec<Share>) -> Self {
        Self { statistic, order_service, instruments, opened_patterns: RwLock::new(Vec::new()) }
    }
}

impl Strategy for FirstStrategy {
    type Statistic = LastPriceState;

    async fn warm_up(&self, positions: PortfolioResponse) -> Result<(), Box<dyn std::error::Error>> {
        for position in positions.positions {
            for instrument in &self.instruments {
                if instrument.uid == position.instrument_uid {
                    let mut _opened_patterns = self.opened_patterns.write().unwrap();
                    println!("Warm_up={:#?}", position.clone());
                    _opened_patterns.push(map_position_to_pattern(position.clone()));
                }
            }
        }
        Ok(())
    }

    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let orders_to_buy = self.signal_buy(&self.statistic).await;
        let orders_to_sell = self.signal_sell(&self.statistic).await;
        for order in orders_to_buy {
            let order_response = self.order_service.order_buy(
                order.figi.clone(),
                order.instrument_id.clone(),
                order.quantity.clone(),
                order.price_open.clone(),
                OrderType::Market,
            ).await;
            match order_response {
                Ok(response) => {
                    let mut opened_patterns = self.opened_patterns.write().unwrap();
                    let _response = response.into_inner();
                    println!("BUY executed_order_price={:#?}", _response.clone().executed_order_price);
                    opened_patterns.push(order);
                }
                err => eprint!("Error in orders_to_buy")
            }
        }

        let mut closed_orders_index = Vec::new();
        for order in orders_to_sell {
            println!("order_to_sell={:#?}", order.clone());
            let closed_order = self.order_service.order_sell(
                order.figi.clone(),
                order.instrument_id.clone(),
                order.quantity.clone(),
                order.price_open.clone(),
                OrderType::Market,
            ).await;
            if closed_order.is_ok() {
                let _closed_order = closed_order.unwrap().into_inner();
                println!("GOT PROFIT={:?}-{:?}", &order.price_open, &_closed_order.executed_order_price);
                let index_to_remove = self.opened_patterns.read().unwrap().iter().position(|x| x.instrument_id == order.instrument_id).unwrap();
                closed_orders_index.push(index_to_remove);
            }
        }
        for order_index in closed_orders_index {
            self.opened_patterns.write().unwrap().remove(order_index);
        }
        Ok(())
    }

    async fn signal_buy(&self, stat: &Self::Statistic) -> Vec<OpenedPattern> {
        let mut request_to_open = Vec::new();
        for instrument in &self.instruments {
            match self.check_pattern(instrument, &stat).await {
                Some(order) => request_to_open.push(order),
                None => continue
            }
        }
        request_to_open
    }

    async fn check_pattern(&self, instrument: &Share, stat: &Self::Statistic) -> Option<OpenedPattern> {
        let min_target = Quotation {
            units: 300,
            nano: 100,
        };
        let curr_price = stat.get_last_price(&instrument.uid).await;
        if curr_price.is_some() &&
            self.opened_patterns.read().unwrap().is_empty() &&
            curr_price.clone().unwrap().units >= min_target.units &&
            curr_price.clone().unwrap().nano > min_target.nano {
            Some(OpenedPattern {
                figi: instrument.figi.clone(),
                quantity: 1,
                price_open: None,
                price_close: None,
                instrument_id: instrument.clone().uid,
            })
        } else {
            None
        }
    }

    async fn signal_sell(&self, stat: &Self::Statistic) -> Vec<OpenedPattern> {
        let mut close_request = Vec::new();
        let _opened_patterns = self.opened_patterns.read().unwrap().clone();
        for order in _opened_patterns {
            let curr_price = stat.get_last_price(&order.instrument_id).await;
            match (curr_price, order.price_open.clone()) {
                (Some(c_price), Some(o_price)) => {
                    if c_price.units > o_price.units ||
                        (c_price.units == o_price.units &&
                            c_price.nano > o_price.nano) {
                        close_request.push(order.clone());
                    }
                }
                _ => { continue; }
            }
        }
        close_request
    }
}
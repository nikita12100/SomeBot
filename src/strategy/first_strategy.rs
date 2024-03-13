use std::ops::Index;
use std::sync::{Arc, RwLock};
use tinkoff_invest_api::tcs::{OrderDirection, OrderType, PostOrderRequest, PostOrderResponse, Quotation, Share};
use uuid::Uuid;
use crate::order::{OrderRequest, OrderService, OrderServiceImpl, OrderServiceSandboxImpl};
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};
use crate::strategy::strategy::Strategy;

// стратегия -- купить дешевле, продать дороже. Для обкатки модели.
pub struct FirstStrategy {
    statistic: Arc<LastPriceState>,
    // order_service: Arc<OrderServiceImpl>,
    order_service: OrderServiceSandboxImpl,
    instruments: Vec<Share>,
    opened_patterns: RwLock<Vec<PostOrderResponse>>,
}

impl FirstStrategy {
    pub fn new(statistic: Arc<LastPriceState>, order_service: OrderServiceSandboxImpl, instruments: Vec<Share>) -> Self {
        Self { statistic, order_service, instruments, opened_patterns: RwLock::new(Vec::new()) }
    }
}

impl Strategy for FirstStrategy {
    type Statistic = LastPriceState;
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("update");
        let orders_to_buy = self.signal_buy(&self.statistic).await;
        let orders_to_sell = self.signal_sell(&self.statistic).await;
        // for order in orders_to_buy {
        //     let order_response = self.order_service.order(order).await;
        //     match order_response {
        //         Ok(response) => {
        //             let mut opened_patterns = self.opened_patterns.write().unwrap();
        //             let _response = response.into_inner();
        //             println!("BUY executed_order_price={:#?}", _response.clone().executed_order_price);
        //             opened_patterns.push(_response);
        //         }
        //         err => eprint!("Error in orders_to_buy")
        //     }
        // }

        let mut closed_orders_index = Vec::new();
        for order in orders_to_sell {
            println!("order_to_sell={:#?}", order.clone());
            let closed_order = self.order_service.order(OrderRequest {
                figi: order.figi.clone(),
                quantity: 1,
                price: None,
                direction: OrderDirection::Sell,
                order_type: OrderType::Market,
                order_id: order.order_id.clone(),
                instrument_id: order.instrument_uid.clone(),
            }).await;
            if closed_order.is_ok() {
                let _closed_order = closed_order.unwrap().into_inner();
                println!(" GOT PROFIT={:?}-{:?}", &order.executed_order_price, &_closed_order.executed_order_price);
                println!(" GOT COMMISSION={:?}", &order.executed_commission);
                let index_to_remove = self.opened_patterns.read().unwrap().iter().position(|x| *x == order).unwrap();
                closed_orders_index.push(index_to_remove);
            }
        }
        for order_index in closed_orders_index {
            self.opened_patterns.write().unwrap().remove(order_index);
        }
        Ok(())
    }

    async fn signal_buy(&self, stat: &Self::Statistic) -> Vec<OrderRequest> {
        let mut request_to_open = Vec::new();
        for instrument in &self.instruments {
            match self.check_pattern(instrument, &stat).await {
                Some(order) => request_to_open.push(order),
                None => continue
            }
        }
        request_to_open
    }

    async fn check_pattern(&self, instrument: &Share, stat: &Self::Statistic) -> Option<OrderRequest> {
        let min_target = Quotation {
            units: 300,
            nano: 100,
        };
        let curr_price = stat.get_last_price(&instrument.uid).await;
        if curr_price.is_some() &&
            self.opened_patterns.read().unwrap().is_empty() &&
            curr_price.clone().unwrap().units >= min_target.units &&
            curr_price.clone().unwrap().nano > min_target.nano {
            Some(OrderRequest {
                figi: instrument.figi.clone(),
                quantity: 1,
                price: None,
                direction: OrderDirection::Buy,
                order_type: OrderType::Market,
                order_id: Uuid::new_v4().to_string(),
                instrument_id: instrument.clone().uid,
            })
        } else {
            None
        }
    }

    async fn signal_sell(&self, stat: &Self::Statistic) -> Vec<PostOrderResponse> {
        let mut close_request = Vec::new();
        let _opened_patterns = self.opened_patterns.read().unwrap().clone();
        for order in _opened_patterns {
            let curr_price = stat.get_last_price(&order.instrument_uid).await;
            if curr_price.is_some() &&
                curr_price.clone().unwrap().units >= order.executed_order_price.clone().unwrap().units ||
                curr_price.clone().unwrap().nano > order.executed_order_price.clone().unwrap().nano {
                close_request.push(order.clone());
            }
        }
        close_request
    }
}
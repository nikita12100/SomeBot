use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
#[cfg(not(test))]
use std::time::SystemTime;
#[cfg(test)]
use mock_instant::SystemTime;
use prost_types::Timestamp;
use tinkoff_invest_api::tcs::{OrderType, PortfolioResponse, Quotation, Share, SubscriptionInterval};
use crate::order_service::{OrderService, OrderServiceHistBoxImpl, OrderServiceSandboxImpl};
use crate::state::candle_state::{CandleState, CandleStateStatistic, SizedRange};
use crate::strategy::strategy::{OpenedPattern, Strategy};
use crate::trading_cfg::HammerStrategySettings;
use crate::utils::wrapper_mock_system_time::WrapperMockSystemTime;

pub struct HammerStrategy {
    statistic: Arc<CandleState>,
    order_service: OrderServiceHistBoxImpl,
    instrument: Share,
    opened_patterns: Vec<OpenedPattern>,
    settings: HammerStrategySettings,
}

impl HammerStrategy {
    pub fn new(
        statistic: Arc<CandleState>,
        order_service: OrderServiceHistBoxImpl,
        instrument: Share,
        settings: HammerStrategySettings,
    ) -> Self {
        Self { statistic, order_service, instrument, opened_patterns: Vec::new(), settings }
    }
}

impl Strategy for HammerStrategy {
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
                order.price_open.clone(),
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
                order.price_close.clone(),
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
        let mut to_buy = Vec::new();
        let range = if cfg!(test) { // for hist training
            let window_time_end = WrapperMockSystemTime(mock_instant::SystemTime::now());
            let window_time_start = WrapperMockSystemTime(window_time_end.0 - Duration::from_secs(self.settings.window_size_min * 60));
            SizedRange::new_1m(Timestamp::from(window_time_start), Timestamp::from(window_time_end))
        } else {
            let window_time_end = std::time::SystemTime::now();
            let window_time_start = window_time_end - Duration::from_secs(self.settings.window_size_min * 60);
            SizedRange::new_1m(Timestamp::from(window_time_start), Timestamp::from(window_time_end))
        };

        let is_trend_bearish = stat.is_trend_bearish(&self.settings.trend_cfg, &self.instrument.uid, range).await;
        let last_candle = stat.get_last_candle(&self.instrument.uid, SubscriptionInterval::OneMinute).await.unwrap();
        let is_hammer_bullish = stat.is_hammer_bullish(&self.settings.hammer_cfg, last_candle.clone()).await;
        if is_trend_bearish && is_hammer_bullish {
            to_buy.push(OpenedPattern {
                figi: self.instrument.figi.clone(),
                quantity: 1, // fixme more quantity if it's more powerful signal
                price_open: None,
                price_close: Some(Quotation {
                    units: last_candle.high.clone().unwrap().units + (last_candle.high.clone().unwrap().units - last_candle.low.clone().unwrap().units) * 2,
                    nano: 0,
                }
                ), // fixme define close price
                instrument_id: self.instrument.uid.clone(),
            });
        }
        println!("-------------------------------------------");
        // println!("signal_buy window_time_end ={:?}", window_time_end);
        // println!("signal_buy window_time_start ={:?}", window_time_start);
        println!("signal_buy is_trend_bearish ={:?}", is_trend_bearish);
        println!("signal_buy is_hammer_bullish ={:?}", is_hammer_bullish);
        println!("signal_buy to_buy ={:?}", to_buy);
        println!("-------------------------------------------");
        to_buy
    }

    async fn check_pattern(&self, instrument: &Share, stat: &Self::Statistic) -> Option<OpenedPattern> {
        None
    }

    // fixme look at last price for faster sell
    async fn signal_sell(&self, stat: &Self::Statistic) -> Vec<OpenedPattern> {
        let mut close_request = Vec::new();
        let last_candle = stat.get_last_candle(&self.instrument.uid, SubscriptionInterval::OneMinute).await.unwrap();
        let last_price = last_candle.close.unwrap().units.min(last_candle.open.unwrap().units);
        for order in self.opened_patterns.clone() {
            if last_price > order.clone().price_close.unwrap().units {
                close_request.push(order.clone());
            }
        }
        close_request
    }
}

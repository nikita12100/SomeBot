use std::collections::HashMap;
use tinkoff_invest_api::tcs::{LastPrice, Quotation};
use crate::state::state::State;

pub struct LastPriceState {
    state_by_instrument_uid: HashMap<String, LastPrice>,
}

pub trait LastPriceStateStatistic {
    fn get_last_price(&self, instrument_uid: &String) -> Option<Quotation>;
}

impl State<LastPrice> for LastPriceState {
    fn new() -> Self {
        LastPriceState {
            state_by_instrument_uid: HashMap::new(),
        }
    }
    fn update(&mut self, last_price: LastPrice) -> Result<(), &'static str> {
        self.state_by_instrument_uid.insert(last_price.instrument_uid.clone(), last_price);
        Ok(())
    }
}

impl LastPriceStateStatistic for LastPriceState {
    fn get_last_price(&self, instrument_uid: &String) -> Option<Quotation> {
        self.state_by_instrument_uid.get(instrument_uid).map(|value| value.price.clone()).flatten()
    }
}
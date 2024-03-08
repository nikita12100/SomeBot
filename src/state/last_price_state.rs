use std::collections::HashMap;
use std::sync::RwLock;
use tinkoff_invest_api::tcs::{LastPrice, Quotation};
use crate::state::state::State;

pub struct LastPriceState {
    price_by_instrument_uid: RwLock<HashMap<String, LastPrice>>,
}

pub trait LastPriceStateStatistic {
    async fn get_last_price(&self, instrument_uid: &String) -> Option<Quotation>;
}

impl State<LastPrice> for LastPriceState {
    fn new() -> Self {
        LastPriceState {
            price_by_instrument_uid: RwLock::new(HashMap::new()),
        }
    }
    fn update(&self, event: &LastPrice) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.price_by_instrument_uid.write().unwrap();
        state.insert(event.instrument_uid.clone(), event.clone());
        Ok(())
    }
}

impl LastPriceStateStatistic for LastPriceState {
    async fn get_last_price(&self, instrument_uid: &String) -> Option<Quotation> {
        let state = self.price_by_instrument_uid.read().unwrap();
        state.get(instrument_uid).map(|value| value.price.clone()).flatten()
    }
}
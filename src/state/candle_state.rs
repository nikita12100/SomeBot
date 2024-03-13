use multimap::MultiMap;
use std::error::Error;
use std::sync::RwLock;
use prost_types::Timestamp;
use tinkoff_invest_api::tcs::{Candle, SubscriptionInterval};
use crate::state::state::State;

pub struct SizedRange {
    interval: SubscriptionInterval,
    start: Timestamp,
    end: Timestamp,
}

pub struct CandleState {
    candles_1_by_instrument_uid: RwLock<MultiMap<String, Candle>>,
    candles_5_by_instrument_uid: RwLock<MultiMap<String, Candle>>,
}

pub trait CandleStateStatistic {
    async fn get_candles(&self, instrument_uid: &String, range: SizedRange) -> Option<Vec<Candle>>;
}

impl SizedRange {
    fn new(interval: SubscriptionInterval, start: Timestamp, end: Timestamp) -> Self {
        SizedRange { interval, start, end }
    }
    pub fn new_1m(start: Timestamp, end: Timestamp) -> Self {
        Self::new(SubscriptionInterval::OneMinute, start, end)
    }
    pub fn new_5m(start: Timestamp, end: Timestamp) -> Self {
        Self::new(SubscriptionInterval::FiveMinutes, start, end)
    }
}

impl State<Candle> for CandleState {
    fn new() -> Self {
        CandleState {
            candles_1_by_instrument_uid: RwLock::new(MultiMap::new()),
            candles_5_by_instrument_uid: RwLock::new(MultiMap::new()),
        }
    }

    fn update(&self, event: &Candle) -> Result<(), Box<dyn Error>> {
        if event.interval == SubscriptionInterval::OneMinute as i32 {
            let mut state = self.candles_1_by_instrument_uid.write().unwrap();
            state.insert(event.instrument_uid.clone(), event.clone());
            Ok(())
        } else if event.interval == SubscriptionInterval::FiveMinutes as i32 {
            let mut state = self.candles_5_by_instrument_uid.write().unwrap();
            state.insert(event.instrument_uid.clone(), event.clone());
            Ok(())
        } else {
            Err(Box::from("Unknown candle subscription interval."))
        }
    }
}

impl CandleStateStatistic for CandleState {
    async fn get_candles(&self, instrument_uid: &String, range: SizedRange) -> Option<Vec<Candle>> {
        if range.interval == SubscriptionInterval::OneMinute {
            let state = self.candles_1_by_instrument_uid.read().unwrap();
            state.get_vec(instrument_uid).map(|vec| vec.clone()) // todo add filter time
        } else if range.interval == SubscriptionInterval::FiveMinutes {
            let state = self.candles_5_by_instrument_uid.read().unwrap();
            state.get_vec(instrument_uid).map(|vec| vec.clone()) // todo add filter time
        } else {
            None
        }
    }
}
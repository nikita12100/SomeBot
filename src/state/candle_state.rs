// use std::collections::HashMap;
// use std::error::Error;
// use std::sync::RwLock;
// use prost_types::Timestamp;
// use tinkoff_invest_api::tcs::{Candle, SubscriptionInterval};
// use crate::state::state::State;
//
// pub struct SizedRange {
//     interval: SubscriptionInterval,
//     start: Timestamp,
//     end: Timestamp,
// }
//
// pub struct CandleState {
//     candles_by_time: RwLock<HashMap<SizedRange, Vec<Candle>>>,
//     candles_by_instrument_uid: RwLock<HashMap<String, Vec<Candle>>>
// }
//
// pub trait CandleStateStatistic {
//     async fn get_candles();
// }
//
// impl State<Candle> for CandleState {
//     fn new() -> Self {
//         CandleState {
//             candles_by_time: RwLock::new(HashMap::new()),
//         }
//     }
//
//     fn update(&self, event: &Candle) -> Result<(), Box<dyn Error>> {
//         todo!()
//     }
// }
//
// impl CandleStateStatistic for CandleState {
//     async fn get_candles() {
//         todo!()
//     }
// }
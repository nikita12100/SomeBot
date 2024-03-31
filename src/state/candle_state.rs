use multimap::MultiMap;
use std::error::Error;
use std::sync::RwLock;
use prost_types::Timestamp;
use tinkoff_invest_api::tcs::{Candle, SubscriptionInterval};
use crate::state::state::State;
use crate::trading_cfg::{HammerCfg, TrendCfg};
use crate::utils::candle::CandleExtension;
use crate::utils::cmp::Cmp;
use crate::utils::quotation::QuotationExtension;

#[derive(Debug, Clone)]
pub struct SizedRange {
    interval: SubscriptionInterval,
    start: Timestamp,
    end: Timestamp,
}

pub struct CandleState {
    candles_1_by_day_time: RwLock<MultiMap<String, Candle>>,
    // todo partition by date-time
    candles_1_by_instrument_uid: RwLock<MultiMap<String, Candle>>,
    candles_5_by_instrument_uid: RwLock<MultiMap<String, Candle>>,
}

pub trait CandleStateStatistic {
    async fn get_last_candle(&self, instrument_uid: &String, interval: SubscriptionInterval) -> Option<Candle>;
    async fn get_candles(&self, instrument_uid: &String, range: SizedRange) -> Option<Vec<Candle>>;
    // бычий молот, рынок пойдет вверх
    async fn is_hammer_bullish(&self, hammer_cfg: &HammerCfg, candle: Candle) -> bool;
    // медвежий молот, рынок пойдет вниз
    async fn is_hammer_bearish(&self, hammer_cfg: &HammerCfg, candle: Candle) -> bool;
    // боковое движение когда мин-макс последующих свеч находятся в пределах предудущей
    async fn is_trend_flat(&self, trend_cfg: &TrendCfg, instrument_uid: &String, range: SizedRange) -> bool;
    // нисходящий(медвежий) тренд -- каждый последующий минимум обновляет предыдущий
    async fn is_trend_bearish(&self, trend_cfg: &TrendCfg, instrument_uid: &String, range: SizedRange) -> bool;
    // восходящий(бычий) тренд -- каждый последующий максимум обновляет предыдущий
    async fn is_trend_bullish(&self, trend_cfg: &TrendCfg, instrument_uid: &String, range: SizedRange) -> bool;
}

impl SizedRange {
    fn new(interval: SubscriptionInterval, start: Timestamp, end: Timestamp) -> Self {
        if start._le(&end) {
            SizedRange { interval, start, end }
        } else {
            panic!("Error while creating SizedRange start={:#?} must be > end=={:#?}", start, end)
        }
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
            candles_1_by_day_time: RwLock::new(MultiMap::new()),
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
    async fn get_last_candle(&self, instrument_uid: &String, interval: SubscriptionInterval) -> Option<Candle> {
        match interval {
            SubscriptionInterval::OneMinute =>
                self.candles_1_by_instrument_uid.read().unwrap().get_vec(instrument_uid).unwrap().last().cloned(),
            SubscriptionInterval::FiveMinutes =>
                self.candles_5_by_instrument_uid.read().unwrap().get_vec(instrument_uid).unwrap().last().cloned(),
            SubscriptionInterval::Unspecified => None
        }
    }

    async fn get_candles(&self, instrument_uid: &String, range: SizedRange) -> Option<Vec<Candle>> {
        let state = match range.interval {
            SubscriptionInterval::OneMinute => Some(self.candles_1_by_instrument_uid.read().unwrap()),
            SubscriptionInterval::FiveMinutes => Some(self.candles_5_by_instrument_uid.read().unwrap()),
            SubscriptionInterval::Unspecified => None,
        };
        match state {
            Some(state) => {
                let candles = state.get_vec(instrument_uid).unwrap();
                let mut answer = Vec::new();
                // мы смотрим от текущего времени в прошлое, поэтому оптимальнее идти с конца
                for candle in candles.iter().rev() {
                    // range.start <= candle.time <= range.end
                    if range.start._leq(candle.time.as_ref().unwrap()) && range.end._geq(candle.time.as_ref().unwrap()) {
                        answer.push(candle.clone());
                    } else {
                        break;
                    }
                }
                Some(answer)
            }
            None => None
        }
    }

    async fn is_hammer_bullish(&self, hammer_cfg: &HammerCfg, candle: Candle) -> bool {
        if candle.is_bullish() {
            let open_prc = candle.percentage_open();
            let close_prc = candle.percentage_close();
            if open_prc >= hammer_cfg.bottom_start && open_prc <= hammer_cfg.bottom_end &&
                close_prc >= hammer_cfg.up_start && close_prc <= hammer_cfg.up_end {
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    async fn is_hammer_bearish(&self, hammer_cfg: &HammerCfg, candle: Candle) -> bool {
        if candle.is_bearish() {
            let open_prc = candle.percentage_open();
            let close_prc = candle.percentage_close();
            if close_prc >= hammer_cfg.bottom_start && close_prc <= hammer_cfg.bottom_end &&
                open_prc >= hammer_cfg.up_start && close_prc <= hammer_cfg.up_end {
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    async fn is_trend_flat(&self, trend_cfg: &TrendCfg, instrument_uid: &String, range: SizedRange) -> bool {
        let candles = self.get_candles(instrument_uid, range).await.unwrap();

        let _low = candles.get(0).unwrap().clone().low.unwrap();
        let _high = candles.get(0).unwrap().clone().high.unwrap();
        let mut is_trend_flat = true;
        let mut candles_to_skip = trend_cfg.max_candle_skip;
        for candle in candles {
            if _low.wr() <= candle.low.unwrap().wr() && _high.wr() >= candle.high.unwrap().wr() {
                candles_to_skip = trend_cfg.max_candle_skip;
                continue;
            } else {
                candles_to_skip -= 1;
                if candles_to_skip < 0 {
                    is_trend_flat = false;
                    break;
                }
            }
        }
        if candles_to_skip < trend_cfg.max_candle_skip {  // проверка чтобы последняя свеча была в тренде
            is_trend_flat = false;
        }

        is_trend_flat
    }

    async fn is_trend_bearish(&self, trend_cfg: &TrendCfg, instrument_uid: &String, range: SizedRange) -> bool {
        let candles = self.get_candles(instrument_uid, range).await.unwrap();

        if candles.len() == 0 {
            false
        } else {
            let mut _low = candles.get(0).unwrap().clone().low.unwrap();
            let mut is_trend_bearish = true;
            let mut candles_to_skip = trend_cfg.max_candle_skip;
            for candle in candles {
                if candle.low.clone().unwrap().wr() <= _low.wr() {
                    _low = candle.low.unwrap();
                    candles_to_skip = trend_cfg.max_candle_skip;
                    continue;
                } else {
                    candles_to_skip -= 1;
                    if candles_to_skip < 0 {
                        is_trend_bearish = false;
                        break;
                    }
                }
            }
            if candles_to_skip < trend_cfg.max_candle_skip {  // проверка чтобы последняя свеча была в тренде
                is_trend_bearish = false;
            }

            is_trend_bearish
        }
    }

    async fn is_trend_bullish(&self, trend_cfg: &TrendCfg, instrument_uid: &String, range: SizedRange) -> bool {
        let candles = self.get_candles(instrument_uid, range).await.unwrap();

        if candles.len() == 0 {
            false
        } else {
            let mut _high = candles.get(0).unwrap().clone().high.unwrap();
            let mut is_trend_bullish = true;
            let mut candles_to_skip = trend_cfg.max_candle_skip;
            for candle in candles {
                if candle.high.clone().unwrap().wr() >= _high.wr() {
                    _high = candle.high.unwrap();
                    candles_to_skip = trend_cfg.max_candle_skip;
                    continue;
                } else {
                    candles_to_skip -= 1;
                    if candles_to_skip < 0 {
                        is_trend_bullish = false;
                        break;
                    }
                }
            }
            if candles_to_skip < trend_cfg.max_candle_skip {  // проверка чтобы последняя свеча была в тренде
                is_trend_bullish = false;
            }

            is_trend_bullish
        }
    }
}
use std::sync::Arc;
use std::time::Duration;
use flume::Sender;
use tinkoff_invest_api::tcs::{CandleInstrument, LastPriceInstrument, market_data_request, MarketDataRequest, MarketDataResponse, SubscribeCandlesRequest, SubscriptionInterval};
use tinkoff_invest_api::tcs::market_data_response;
use tinkoff_invest_api::tcs::{Share, SubscribeLastPriceRequest, SubscriptionAction};
use tinkoff_invest_api::TinkoffInvestService;
use tokio::{task, time};
use tokio::task::JoinHandle;
use crate::prepare_md_stream;
use crate::state::candle_state::CandleState;
use crate::state::last_price_state::LastPriceState;
use crate::state::state::State;

pub mod state;
pub mod last_price_state;
pub mod candle_state;

fn map_to_candle_subscribe_request(shares: &Vec<Share>) -> Vec<CandleInstrument> {
    let mut res = Vec::new();
    shares.iter().for_each(|share| {
        res.push(CandleInstrument {
            figi: share.figi.to_string(),
            interval: SubscriptionInterval::OneMinute as i32,
            instrument_id: share.uid.to_string(),
        });
        res.push(CandleInstrument {
            figi: share.figi.to_string(),
            interval: SubscriptionInterval::FiveMinutes as i32,
            instrument_id: share.uid.to_string(),
        });
    });
    res
}

fn map_to_last_price_subscribe_request(shares: &Vec<Share>) -> Vec<LastPriceInstrument> {
    shares.iter().map(|share| {
        LastPriceInstrument {
            figi: share.figi.to_string(),
            instrument_id: share.uid.to_string(),
        }
    }).collect()
}

pub async fn run_handling_messages_last_price(service: &TinkoffInvestService, instruments: Vec<Share>, state: Arc<LastPriceState>) -> (Sender<MarketDataRequest>, JoinHandle<()>) {
    let request = MarketDataRequest {
        payload: Some(market_data_request::Payload::SubscribeCandlesRequest(SubscribeCandlesRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: map_to_candle_subscribe_request(&instruments),
            waiting_close: true,
        })),
    };
    let (tx, mut streaming) = prepare_md_stream(service, request).await;

    let updater = task::spawn(async move {
        loop {
            match streaming.message().await.unwrap() {
                Some(next_message) => {
                    let payload = next_message.payload.clone().unwrap();
                    match payload {
                        market_data_response::Payload::LastPrice(last_price) => {
                            state.update(&last_price)
                                .unwrap_or_else(|err| eprintln!("Error updating last_price_state: {}", err));
                        }
                        _ => {
                            println!("MarketData unknown message payload: {:?}", next_message);
                        }
                    }
                }
                _ => {
                    println!("fail parse streaming message");
                    time::sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }
    );

    (tx, updater)
}

pub async fn run_handling_messages_candles(service: &TinkoffInvestService, instruments: Vec<Share>, state: Arc<CandleState>) -> (Sender<MarketDataRequest>, JoinHandle<()>) {
    let request = MarketDataRequest {
        payload: Some(market_data_request::Payload::SubscribeLastPriceRequest(SubscribeLastPriceRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: map_to_last_price_subscribe_request(&instruments),
        })),
    };
    let (tx, mut streaming) = prepare_md_stream(service, request).await;

    let updater = task::spawn(async move {
        loop {
            match streaming.message().await.unwrap() {
                Some(next_message) => {
                    let payload = next_message.payload.clone().unwrap();
                    match payload {
                        market_data_response::Payload::Candle(candle) => {
                            state.update(&candle)
                                .unwrap_or_else(|err| eprintln!("Error updating candle_state: {}", err));
                        }
                        _ => {
                            println!("MarketData unknown message payload: {:?}", next_message);
                        }
                    }
                }
                _ => {
                    println!("fail parse streaming message");
                    time::sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }
    );

    (tx, updater)
}

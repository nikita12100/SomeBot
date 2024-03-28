use std::sync::Arc;
use std::time::Duration;
use flume::Sender;
use tinkoff_invest_api::tcs::{CandleInstrument, LastPriceInstrument, MarketDataRequest, PortfolioResponse, SubscriptionInterval};
use tinkoff_invest_api::tcs::Share;
use tinkoff_invest_api::tcs::market_data_request::Payload::{SubscribeCandlesRequest, SubscribeLastPriceRequest};
use tinkoff_invest_api::tcs::market_data_response::Payload::{Candle, LastPrice, SubscribeCandlesResponse, SubscribeLastPriceResponse};
use tinkoff_invest_api::tcs::SubscriptionAction::Subscribe;
use tinkoff_invest_api::{tcs, TinkoffInvestService};
use tokio::{task, time};
use tokio::task::JoinHandle;
use crate::operations_service::OperationsServiceSandBoxImpl;
use crate::prepare_md_stream;
use crate::order_service::OrderServiceSandboxImpl;
use crate::state::candle_state::CandleState;
use crate::state::last_price_state::LastPriceState;
use crate::state::state::State;
use crate::strategy::first_strategy::FirstStrategy;
use crate::strategy::strategy::Strategy;

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

pub async fn run_updater_last_price(
    service: &TinkoffInvestService,
    instruments: Vec<Share>,
    state: Arc<LastPriceState>,
    order_service: OrderServiceSandboxImpl,
    positions: PortfolioResponse,
) -> (Sender<MarketDataRequest>, JoinHandle<()>) {
    let request = MarketDataRequest {
        payload: Some(SubscribeLastPriceRequest(tcs::SubscribeLastPriceRequest {
            subscription_action: Subscribe as i32,
            instruments: map_to_last_price_subscribe_request(&instruments),
        })),
    };
    let (tx, mut streaming) = prepare_md_stream(service, request).await;

    let updater = task::spawn(async move {
        let mut first_strategy = FirstStrategy::new(Arc::clone(&state), order_service, instruments.clone());
        first_strategy.warm_up(positions).await.expect("Error while warm up first_strategy");

        loop {
            match streaming.message().await.unwrap() {
                Some(next_message) => {
                    let payload = next_message.payload.clone().unwrap();
                    match payload {
                        SubscribeLastPriceResponse(subscribe_response) => {
                            println!("Successfully subscribed to last price streaming.\n{:#?}", subscribe_response);
                        }
                        LastPrice(last_price) => {
                            state.update(&last_price)
                                .unwrap_or_else(|err| eprintln!("Error updating last_price_state: {}", err));

                            first_strategy.update().await.expect("Error updating first strategy");
                        }
                        _ => {
                            println!("MarketData last_price unknown message payload: {:#?}", payload);
                        }
                    }
                }
                _ => {
                    println!("fail parse last_price streaming message");
                    time::sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }
    );
    (tx, updater)
}

pub async fn run_updater_candles(service: &TinkoffInvestService, instruments: Vec<Share>, state: Arc<CandleState>) -> (Sender<MarketDataRequest>, JoinHandle<()>) {
    let request = MarketDataRequest {
        payload: Some(SubscribeCandlesRequest(tcs::SubscribeCandlesRequest {
            subscription_action: Subscribe as i32,
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
                        SubscribeCandlesResponse(subscribe_response) => {
                            println!("Successfully subscribed to candle streaming.\n{:#?}", subscribe_response);
                        }
                        Candle(candle) => {
                            state.update(&candle)
                                .unwrap_or_else(|err| eprintln!("Error updating candle_state: {}", err));
                        }
                        _ => {
                            println!("MarketData candle unknown message payload: {:#?}", payload);
                        }
                    }
                }
                _ => {
                    println!("fail parse candle streaming message");
                    time::sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }
    );
    (tx, updater)
}

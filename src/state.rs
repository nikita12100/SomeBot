use std::sync::{Arc, Mutex};
use std::time::Duration;
use flume::Sender;
use tinkoff_invest_api::tcs::{market_data_request, MarketDataRequest, MarketDataResponse};
use tinkoff_invest_api::tcs::market_data_response;
use tinkoff_invest_api::tcs::{LastPrice, Share, SubscribeLastPriceRequest, SubscriptionAction};
use tinkoff_invest_api::{TIError, TinkoffInvestService, TIResult};
use tokio::{task, time};
use tokio::task::JoinHandle;
use tonic::{Response, Streaming};
use tonic::transport::Channel;
use crate::{map_to_last_price_subscribe_request, prepare_channel};
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};
use crate::state::state::State;

pub mod state;
pub mod last_price_state;
mod candle_state;

async fn prepare_stream_md_last_price(service: TinkoffInvestService, instruments: Vec<Share>) -> (Sender<MarketDataRequest>, Streaming<MarketDataResponse>) {
    let channel = prepare_channel(false).await.unwrap();
    let mut marketdata_stream = service.marketdata_stream(channel).await.unwrap();
    let (tx, rx) = flume::unbounded();
    let request = tinkoff_invest_api::tcs::MarketDataRequest {
        payload: Some(market_data_request::Payload::SubscribeLastPriceRequest(SubscribeLastPriceRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: map_to_last_price_subscribe_request(&instruments),
        })),
    };
    tx.send(request).unwrap();
    let response: Response<Streaming<MarketDataResponse>> = marketdata_stream
        .market_data_stream(rx.into_stream())
        .await.unwrap();
    (tx, response.into_inner())
}

pub async fn run_handling_messages(service: TinkoffInvestService, instruments: Vec<Share>, state: Arc<LastPriceState>) -> (Sender<MarketDataRequest>, JoinHandle<()>) {
    let (tx, mut streaming) = prepare_stream_md_last_price(service, instruments.clone()).await;

    let updater = task::spawn(async move {
        loop {
            if let Some(next_message) = streaming.message().await.unwrap() {
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
            } else {
                println!("fail parse streaming message");
                time::sleep(Duration::from_millis(1000)).await;
            }
        }
    }
    );

    (tx, updater)
}

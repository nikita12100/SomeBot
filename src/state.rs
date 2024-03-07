use std::sync::{Arc, Mutex};
use std::time::Duration;
use tinkoff_invest_api::tcs::{market_data_request, MarketDataResponse};
use tinkoff_invest_api::tcs::market_data_response;
use tinkoff_invest_api::tcs::{LastPrice, Share, SubscribeLastPriceRequest, SubscriptionAction};
use tinkoff_invest_api::{TIError, TinkoffInvestService, TIResult};
use tokio::{task, time};
use tokio::task::JoinHandle;
use tonic::Streaming;
use tonic::transport::Channel;
use crate::{map_to_last_price_subscribe_request, prepare_channel};
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};
use crate::state::state::State;

pub mod state;
pub mod last_price_state;
mod candle_state;

pub async fn run_handling_messages(mut streaming: Streaming<MarketDataResponse>, state: Arc<LastPriceState>) -> JoinHandle<()> {
    task::spawn(async move {
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
    )
}

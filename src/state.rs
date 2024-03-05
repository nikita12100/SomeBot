use std::sync::{Arc, Mutex};
use std::time::Duration;
use tinkoff_invest_api::tcs::{market_data_request, MarketDataResponse};
use tinkoff_invest_api::tcs::market_data_response;
use tinkoff_invest_api::tcs::{LastPrice, Share, SubscribeLastPriceRequest, SubscriptionAction};
use tinkoff_invest_api::{TIError, TinkoffInvestService, TIResult};
use tokio::{task, time};
use crate::{map_to_last_price_subscribe_request, prepare_channel};
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};
use crate::state::state::State;

mod state;
pub mod last_price_state;
mod candle_state;

pub async fn run_last_price_state(service: &TinkoffInvestService, shares: Vec<Share>) -> TIResult<()> {
    let channel = prepare_channel(false).await?;
    let mut marketdata_stream = service.marketdata_stream(channel).await?;
    let (tx, rx) = flume::unbounded();

    let request = tinkoff_invest_api::tcs::MarketDataRequest {
        payload: Some(market_data_request::Payload::SubscribeLastPriceRequest(SubscribeLastPriceRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: map_to_last_price_subscribe_request(&shares),
        })),
    };
    tx.send(request).unwrap();

    let response = marketdata_stream
        .market_data_stream(rx.into_stream())
        .await?;

    let mut streaming = response.into_inner();

    let last_price_state = Arc::new(Mutex::new(LastPriceState::new()));

    let last_price_state_clone = Arc::clone(&last_price_state);
    let update_task = task::spawn(async move {
        loop {
            if let Some(next_message) = streaming.message().await.unwrap() {
                let payload = next_message.payload.clone().unwrap();
                match payload {
                    market_data_response::Payload::LastPrice(last_price) => {
                        let mut state = last_price_state.lock().unwrap();
                        state.update(&last_price)
                            .unwrap_or_else(|err| eprintln!("Error updating last_price_state: {}", err));
                        drop(state);
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

    loop { // fixme move it
        let state = last_price_state_clone.lock().unwrap();
        println!("Now price: {:?}", state.get_last_price(&shares.get(0).unwrap().uid));
        drop(state);
        time::sleep(Duration::from_millis(2000)).await;
    }

    // Ok(last_price_state_clone)
    Ok(())
}

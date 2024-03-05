mod local_tokens;
mod state;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use error_chain::ExitCode;
use tonic::{Streaming, transport::{Channel, ClientTlsConfig}};
use futures::{TryStreamExt};
use futures_util::{FutureExt};
use prost_types::Timestamp;
use tinkoff_invest_api::{tcs::{
    market_data_request::Payload, CandleInstrument,
    SubscribeCandlesRequest, SubscriptionAction,
    SubscriptionInterval,
}, TIResult, TinkoffInvestService, DefaultInterceptor};
use tinkoff_invest_api::tcs::market_data_stream_service_client::MarketDataStreamServiceClient;
use tinkoff_invest_api::tcs::{FindInstrumentRequest, FindInstrumentResponse, InstrumentIdType, InstrumentRequest, InstrumentsRequest, InstrumentStatus, InstrumentType, LastPriceInstrument, market_data_request, MarketDataResponse, Share, ShareResponse, SharesResponse, SubscribeLastPriceRequest};
use tinkoff_invest_api::tcs::instruments_service_client::InstrumentsServiceClient;
use tokio::{task, time};
use tonic::codegen::InterceptedService;
use crate::state::run_last_price_state;
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};

struct TimeRange {
    start: Option<Timestamp>,
    end: Option<Timestamp>,
}

async fn prepare_channel(is_prod: bool) -> TIResult<Channel> {
    let path = if is_prod {
        "https://invest-public-api.tinkoff.ru:443/"
    } else {
        "https://sandbox-invest-public-api.tinkoff.ru:443/"
    };
    let tls = ClientTlsConfig::new();

    Ok(Channel::from_static(path)
        .tls_config(tls)?
        .connect()
        .await?)
}

fn map_to_candle_subscribe_request(shares: &Vec<Share>) -> Vec<CandleInstrument> {
    shares.iter().map(|share| {
        CandleInstrument {
            figi: share.figi.to_string(),
            interval: SubscriptionInterval::OneMinute as i32,
            instrument_id: share.uid.to_string(),
        }
    }).collect()
}

fn map_to_last_price_subscribe_request(shares: &Vec<Share>) -> Vec<LastPriceInstrument> {
    shares.iter().map(|share| {
        LastPriceInstrument {
            figi: share.figi.to_string(),
            instrument_id: share.uid.to_string(),
        }
    }).collect()
}

// fixme remove old example, use state
async fn state_reader(service: &TinkoffInvestService, shares: Vec<Share>) -> TIResult<()> {
    let channel = prepare_channel(false).await?;
    let mut marketdata_stream = service.marketdata_stream(channel).await?;
    let (tx, rx) = flume::unbounded();

    // let request = tinkoff_invest_api::tcs::MarketDataRequest {
    //     payload: Some(Payload::SubscribeCandlesRequest(SubscribeCandlesRequest {
    //         subscription_action: SubscriptionAction::Subscribe as i32,
    //         instruments: map_to_candle_subscribe_request(&shares),
    //         waiting_close: true,
    //     })),
    // };
    let request = tinkoff_invest_api::tcs::MarketDataRequest {
        payload: Some(Payload::SubscribeLastPriceRequest(SubscribeLastPriceRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: map_to_last_price_subscribe_request(&shares),
        })),
    };

    tx.send(request).unwrap();

    let response = marketdata_stream
        .market_data_stream(rx.into_stream())
        .await?;

    let mut streaming = response.into_inner();

    loop {
        if let Some(next_message) = streaming.message().await? {
            println!("MarketData {:?}", next_message);
        }
    }

    Ok(())
}

async fn prepare_instruments(service: &TinkoffInvestService, tickers: Vec<&str>) -> Vec<Share> {
    let channel = prepare_channel(false).await.unwrap();
    let mut instrument_service_client = service.instruments(channel.clone()).await.unwrap();
    let all_instruments = instrument_service_client.shares(InstrumentsRequest {
        instrument_status: InstrumentStatus::Base as i32,
    }).await.unwrap().into_inner().instruments;

    all_instruments.into_iter().filter(|share| tickers.contains(&&**&share.ticker)).collect()
}

#[tokio::main]
async fn main() -> TIResult<()> {
    let (prod_token, sandbox_token) = local_tokens::get_local_tokens();
    let tickers = vec!["SBER", "TCSG"];

    let service = TinkoffInvestService::new(sandbox_token.parse().unwrap());

    let instruments = prepare_instruments(&service, tickers).await;
    println!("{:?}", instruments);

    // let last_price_state = run_last_price_state(&service, instruments.clone()).await.expect("TODO: panic message");
    run_last_price_state(&service, instruments.clone()).await.expect("TODO: panic message");

    // loop {
    //     let state = last_price_state.lock().unwrap();
    //     println!("Now price: {:?}", state.get_last_price(&instruments.get(0).unwrap().uid));
    //     drop(state);
    //     time::sleep(Duration::from_millis(2000)).await;
    // }

    Ok(())
}
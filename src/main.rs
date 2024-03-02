mod local_tokens;

use std::env;
use tonic::{transport::{Channel, ClientTlsConfig}};
use futures::{TryStreamExt};
use futures_util::{FutureExt};
use tinkoff_invest_api::{tcs::{
    market_data_request::Payload, CandleInstrument,
    SubscribeCandlesRequest, SubscriptionAction,
    SubscriptionInterval,
}, TIResult, TinkoffInvestService};

async fn test_example_prod(token: &str) -> TIResult<()> {
    let service = TinkoffInvestService::new(token.parse().unwrap());
    let channel = service.create_channel().await?;
    let mut marketdata_stream = service.marketdata_stream(channel).await?;

    let (tx, rx) = flume::unbounded();
    let request = tinkoff_invest_api::tcs::MarketDataRequest {
        payload: Some(Payload::SubscribeCandlesRequest(SubscribeCandlesRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: vec![CandleInstrument {
                figi: "BBG00YFSF9D7".to_string(),
                interval: SubscriptionInterval::OneMinute as i32,
                instrument_id: "figi".to_string(),
            }],
            waiting_close: true,
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

async fn test_example_sandbox(token: &str) -> TIResult<()> {
    let service = TinkoffInvestService::new(token.parse().unwrap());

    let tls = ClientTlsConfig::new();
    let channel = Channel::from_static("https://sandbox-invest-public-api.tinkoff.ru:443/")
        .tls_config(tls)?
        .connect()
        .await?;
    let mut marketdata_stream = service.marketdata_stream(channel).await?;

    let (tx, rx) = flume::unbounded();
    let request = tinkoff_invest_api::tcs::MarketDataRequest {
        payload: Some(Payload::SubscribeCandlesRequest(SubscribeCandlesRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: vec![CandleInstrument {
                figi: "BBG00YFSF9D7".to_string(),
                interval: SubscriptionInterval::OneMinute as i32,
                instrument_id: "figi".to_string(),
            }],
            waiting_close: true,
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

#[tokio::main]
async fn main() -> TIResult<()> {
    let (prod_token, sandbox_token) = local_tokens::get_local_tokens();

    // test_example_prod(&prod_token).await.expect("TODO: panic message");
    test_example_sandbox(&sandbox_token).await.expect("TODO: panic message");

    Ok(())
}
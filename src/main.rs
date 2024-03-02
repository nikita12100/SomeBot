mod local_tokens;

use tonic::{transport::{Channel, ClientTlsConfig}};
use futures::{TryStreamExt};
use futures_util::{FutureExt};
use tinkoff_invest_api::{tcs::{
    market_data_request::Payload, CandleInstrument,
    SubscribeCandlesRequest, SubscriptionAction,
    SubscriptionInterval,
}, TIResult, TinkoffInvestService, DefaultInterceptor};
use tinkoff_invest_api::tcs::market_data_stream_service_client::MarketDataStreamServiceClient;
use tinkoff_invest_api::tcs::{FindInstrumentRequest, FindInstrumentResponse, InstrumentType, LastPriceInstrument, SubscribeLastPriceRequest};
use tonic::codegen::InterceptedService;

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

async fn state_reader(service: &TinkoffInvestService) -> TIResult<()> {
    let channel = prepare_channel(false).await?;
    let mut marketdata_stream = service.marketdata_stream(channel).await?;
    let (tx, rx) = flume::unbounded();
    // let request = tinkoff_invest_api::tcs::MarketDataRequest {
    //     payload: Some(Payload::SubscribeCandlesRequest(SubscribeCandlesRequest {
    //         subscription_action: SubscriptionAction::Subscribe as i32,
    //         instruments: vec![CandleInstrument {
    //             figi: "BBG00YFSF9D7".to_string(),
    //             interval: SubscriptionInterval::OneMinute as i32,
    //             instrument_id: "figi".to_string(),
    //         }],
    //         waiting_close: true,
    //     })),
    // };
    let request = tinkoff_invest_api::tcs::MarketDataRequest {
        payload: Some(Payload::SubscribeLastPriceRequest(SubscribeLastPriceRequest {
            subscription_action: SubscriptionAction::Subscribe as i32,
            instruments: vec![LastPriceInstrument{
                figi: "RU000A1011U5".to_string(),
                instrument_id: "RU000A1011U5".to_string(),
            }]
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

async fn get_instruments(service: &TinkoffInvestService) -> TIResult<FindInstrumentResponse> {
    let channel = prepare_channel(false).await?;
    let mut instrument_service = service.instruments(channel.clone()).await?;

    let instruments = instrument_service.find_instrument(FindInstrumentRequest{
        query: "?ticker=IMOEX".to_string(), // fixme uncorrect query
        instrument_kind: InstrumentType::Share as i32,
        api_trade_available_flag: true,
    }).await?;

    Ok(instruments.into_inner())
}
#[tokio::main]
async fn main() -> TIResult<()> {
    let (prod_token, sandbox_token) = local_tokens::get_local_tokens();

    let service = TinkoffInvestService::new(sandbox_token.parse().unwrap());

    let instruments = get_instruments(&service).await?;
    println!("{:?}", instruments);

    state_reader(&service).await.expect("TODO: panic message");

    Ok(())
}
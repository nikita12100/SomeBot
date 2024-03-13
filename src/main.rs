mod local_tokens;
mod state;
mod user;
mod strategy;
mod order;

use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use flume::Sender;
use prost_types::Timestamp;
use tonic::{Response, Streaming, transport::{Channel, ClientTlsConfig}};
use tinkoff_invest_api::{TIResult, TinkoffInvestService};
use tinkoff_invest_api::tcs::{GetAccountsRequest, InstrumentsRequest, InstrumentStatus, market_data_request, MarketDataRequest, MarketDataResponse, Share, SubscribeLastPriceRequest, SubscriptionAction};
use tokio::{task, time};
use tokio::task::JoinHandle;
use crate::order::{OrderServiceImpl, OrderServiceSandboxImpl};
use crate::state::{run_daemon_handling_messages_last_price, run_daemon_handling_messages_candles};
use crate::state::candle_state::{CandleState, CandleStateStatistic, SizedRange};
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};
use crate::state::state::State;
use crate::strategy::first_strategy::FirstStrategy;
use crate::user::sandbox_user_demo;


async fn prepare_channel() -> TIResult<Channel> {
    let is_prod = false;
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

async fn prepare_instruments(service: &TinkoffInvestService, tickers: Vec<&str>) -> Vec<Share> {
    let channel = prepare_channel().await.unwrap();
    let mut instrument_service_client = service.instruments(channel).await.unwrap();
    let all_instruments = instrument_service_client.shares(InstrumentsRequest {
        instrument_status: InstrumentStatus::Base as i32,
    }).await.unwrap().into_inner().instruments;

    all_instruments.into_iter().filter(|share| tickers.contains(&&**&share.ticker)).collect()
}

async fn prepare_md_stream(service: &TinkoffInvestService, request: MarketDataRequest) -> (Sender<MarketDataRequest>, Streaming<MarketDataResponse>) {
    let channel = prepare_channel().await.unwrap();
    let mut marketdata_stream = service.marketdata_stream(channel).await.unwrap();
    let (tx, rx) = flume::unbounded();
    tx.send(request).unwrap();
    let response = marketdata_stream
        .market_data_stream(rx.into_stream())
        .await.unwrap();
    (tx, response.into_inner())
}

async fn prepare_order_service(service: &TinkoffInvestService) -> OrderServiceSandboxImpl {
    let channel = prepare_channel().await.unwrap();
    // let order_service = Arc::new(OrderService::new(account, client));

    let mut sandbox_client = service.sandbox(channel).await.unwrap();
    let accounts = sandbox_client.get_sandbox_accounts(GetAccountsRequest {}).await.unwrap().into_inner().accounts;

    OrderServiceSandboxImpl::new(accounts.get(0).unwrap().clone(), sandbox_client)
}

#[tokio::main]
async fn main() -> TIResult<()> {
    let (prod_token, sandbox_token) = local_tokens::get_local_tokens();
    let tickers = vec!["SBER", "TCSG"];

    let service = TinkoffInvestService::new(sandbox_token.parse().unwrap());
    let instruments = prepare_instruments(&service, tickers).await;


    let last_price_state = Arc::new(LastPriceState::new());
    let candle_state = Arc::new(CandleState::new());

    let (tx, _) = run_daemon_handling_messages_last_price(&service, instruments.clone(), Arc::clone(&last_price_state)).await;
    let (tx, _) = run_daemon_handling_messages_candles(&service, instruments.clone(), Arc::clone(&candle_state)).await;

    analysis_demo(last_price_state, candle_state, instruments.clone()).await;
    // sandbox_user_demo(&service).await;

    Ok(())
}

async fn analysis_demo(last_price_state: Arc<LastPriceState>, candle_state: Arc<CandleState>, instruments: Vec<Share>) {
    loop {
        println!("Now price: {:?}", last_price_state.get_last_price(&instruments.get(0).unwrap().uid).await);

        // let range = SizedRange::new_1m(Timestamp::from(SystemTime::now()), Timestamp::from(SystemTime::now()));
        // println!("Now candles(1): {:?}", candle_state.get_candles(&instruments.get(0).unwrap().uid, range).await);
        //
        // let range_2 = SizedRange::new_1m(Timestamp::from(SystemTime::now()), Timestamp::from(SystemTime::now()));
        // println!("Now candles(2): {:?}", candle_state.get_candles(&instruments.get(1).unwrap().uid, range_2).await);

        time::sleep(Duration::from_millis(2000)).await;
    }
}
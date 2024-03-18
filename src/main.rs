mod local_tokens;
mod state;
mod user;
mod strategy;
mod order;
mod operations;

use std::fs;
use std::sync::Arc;
use std::time::Duration;
use flume::Sender;
use tonic::{Streaming, transport::{Channel, ClientTlsConfig}};
use tinkoff_invest_api::{TIResult, TinkoffInvestService};
use tinkoff_invest_api::tcs::{Account, GetAccountsRequest, InstrumentsRequest, InstrumentStatus, MarketDataRequest, MarketDataResponse, Share};
use tokio::time;
use crate::operations::{OperationsServiceSandBoxImpl, OperationsService};
use crate::order::OrderServiceSandboxImpl;
use crate::state::{run_updater_last_price, run_updater_candles};
use crate::state::candle_state::{CandleState, CandleStateStatistic};
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};
use crate::state::state::State;
use crate::user::{BrokerAccountSandboxImpl, BrokerAccountService};


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

async fn chose_account(service: &TinkoffInvestService) -> Account {
    let channel = prepare_channel().await.unwrap();
    let mut account_client = service.sandbox(channel).await.unwrap();
    let accounts = account_client.get_sandbox_accounts(GetAccountsRequest {}).await.unwrap().into_inner().accounts;
    let chosen_acc = accounts.get(0).unwrap().clone();
    chosen_acc
}

async fn prepare_broker_account_service(service: &TinkoffInvestService, account: Account) -> BrokerAccountSandboxImpl {
    let channel = prepare_channel().await.unwrap();
    let broker_account_client = service.sandbox(channel).await.unwrap();
    BrokerAccountSandboxImpl::new(account, broker_account_client)
}

async fn prepare_order_service(service: &TinkoffInvestService, account: Account) -> OrderServiceSandboxImpl {
    let channel = prepare_channel().await.unwrap();
    let order_service = service.sandbox(channel).await.unwrap();
    OrderServiceSandboxImpl::new(account, order_service)
}

async fn prepare_operations_service(service: &TinkoffInvestService, account: Account) -> OperationsServiceSandBoxImpl {
    let channel = prepare_channel().await.unwrap();
    let operation_client = service.sandbox(channel).await.unwrap();
    OperationsServiceSandBoxImpl::new(account, operation_client)
}

#[tokio::main]
async fn main() -> TIResult<()> {
    let (prod_token, sandbox_token) = local_tokens::get_local_tokens();
    let tickers = vec!["SBER", "TCSG"];

    let service = TinkoffInvestService::new(sandbox_token.parse().unwrap());
    let instruments = prepare_instruments(&service, tickers).await;

    let account = chose_account(&service).await;
    let mut broker_account_service = prepare_broker_account_service(&service, account.clone()).await;
    let mut order_service_sandbox = prepare_order_service(&service, account.clone()).await;
    let mut operations_service_sandbox = prepare_operations_service(&service, account.clone()).await;

    let positions = operations_service_sandbox.get_portfolio().await;

    let last_price_state = Arc::new(LastPriceState::new());
    let candle_state = Arc::new(CandleState::new());

    let (tx, _) = run_updater_last_price(&service, instruments.clone(), Arc::clone(&last_price_state), order_service_sandbox, positions).await;
    let (tx, _) = run_updater_candles(&service, instruments.clone(), Arc::clone(&candle_state)).await;

    print_states(last_price_state, candle_state, instruments.clone()).await;

    // order_service_sandbox.get_orders().await;
    // operations_service_sandbox.get_positions().await;
    // operations_service_sandbox.get_portfolio().await;

    Ok(())
}

async fn print_states(last_price_state: Arc<LastPriceState>, candle_state: Arc<CandleState>, instruments: Vec<Share>) {
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

#[cfg(test)]
mod test {
    use std::cmp::Ordering;
    use super::*;
    use std::error::Error;
    use chrono::DateTime;
    use csv::{ReaderBuilder, StringRecord};
    use prost_types::Timestamp;
    use tinkoff_invest_api::tcs::{Candle, market_data_response, Quotation, SubscriptionInterval};


    fn read_quotation(data: &str) -> Quotation {
        let mut splited = data.split(".");
        Quotation {
            units: splited.next().unwrap().parse().unwrap(),
            nano: splited.next().unwrap().parse().unwrap(),
        }
    }

    fn read_candle(row: StringRecord, interval: SubscriptionInterval) -> Candle {
        let mut data = row.get(0).unwrap().split(";");
        let instrument_uid = data.next().unwrap().to_string();
        let time = match DateTime::parse_from_rfc3339(data.next().unwrap()) {
            Ok(dt) => Some(Timestamp {
                seconds: dt.timestamp(),
                nanos: dt.timestamp_subsec_nanos() as i32,
            }),
            _ => {
                eprint!("Error parsing time in hist data {:?}", data);
                None
            }
        };
        let open = read_quotation(data.next().unwrap());
        let close = read_quotation(data.next().unwrap());
        let high = read_quotation(data.next().unwrap());
        let low = read_quotation(data.next().unwrap());
        let volume = data.next().unwrap();
        Candle {
            figi: instrument_uid.clone(),
            interval: interval as i32,
            open: Some(open),
            high: Some(high),
            low: Some(low),
            close: Some(close),
            volume: volume.parse().unwrap(),
            time: time,
            last_trade_ts: None,
            instrument_uid: instrument_uid,
        }
    }

    fn candles_comparator(candle_1: &Candle, candle_2: &Candle) -> Ordering {
        let seconds_1 = candle_1.clone().time.unwrap().seconds;
        let seconds_2 = candle_2.clone().time.unwrap().seconds;
        let nanos_1 = candle_1.clone().time.unwrap().nanos;
        let nanos_2 = candle_2.clone().time.unwrap().nanos;
        if seconds_1 < seconds_2 || (seconds_1 == seconds_2 && nanos_1 < nanos_2) {
            Ordering::Less
        } else if seconds_1 > seconds_2 || (seconds_1 == seconds_2 && nanos_1 > nanos_2) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    // read from csv files, return candle stream
    async fn prepare_hist_data(dir_path: &str, interval: SubscriptionInterval) -> Result<Vec<Candle>, Box<dyn Error>> {
        let mut candles = Vec::new();
        let dir_entries = fs::read_dir(dir_path).unwrap();
        for entry in dir_entries {
            let file_path = entry.unwrap().path();
            if file_path.is_file() && file_path.extension().map(|ext| ext == "csv").unwrap_or(false) {
                let file_content = fs::read_to_string(file_path).unwrap();
                let mut csv_reader = ReaderBuilder::new().from_reader(file_content.as_bytes());
                match csv_reader.records().next() {
                    Some(candle_raw) => {
                        candles.push(read_candle(candle_raw.unwrap(), interval));
                        candles.sort_by(candles_comparator);
                    }
                    _ => eprint!("Error while parsing hist data, got unexpected value")
                }
            } else {
                eprint!("Empty dir")
            }
            break;
        }
        Ok(candles)
    }

    #[tokio::test]
    async fn test_hummer_strategy() {
        let stream = prepare_hist_data("hist_data/2023-SBER/", SubscriptionInterval::OneMinute).await.unwrap();

        println!("stream={:#?}", stream);

        assert_eq!(true, false);
    }
}
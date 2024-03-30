mod state;
mod strategy;
mod trading_cfg;
mod utils;
mod service;

use std::fs;
use std::sync::Arc;
use std::time::Duration;
use flume::Sender;
use prost_types::Timestamp;
use tonic::{Streaming, transport::{Channel, ClientTlsConfig}};
use tinkoff_invest_api::{TIResult, TinkoffInvestService};
use tinkoff_invest_api::tcs::{Account, GetAccountsRequest, InstrumentsRequest, InstrumentStatus, MarketDataRequest, MarketDataResponse, Quotation, Share};
use tokio::time;
use crate::service::operations_service::{OperationsService, OperationsServiceSandBoxImpl};
use crate::service::order_service::OrderServiceSandboxImpl;
use crate::service::user_service::BrokerAccountSandboxImpl;
use crate::state::{run_updater_last_price, run_updater_candles};
use crate::state::candle_state::{CandleState, CandleStateStatistic};
use crate::state::last_price_state::{LastPriceState, LastPriceStateStatistic};
use crate::state::state::State;
use crate::utils::local_tokens;


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

// -------------------------------------- TEST --------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use std::error::Error;
    use std::io::{self, Read, Write};
    use std::fs::File;
    use chrono::{DateTime, TimeZone, Utc};
    use csv::{ReaderBuilder, StringRecord};
    use mock_instant::{MockClock, SystemTime};
    use prost_types::Timestamp;
    use tinkoff_invest_api::tcs::{Candle, Quotation, SubscriptionInterval};
    use reqwest::header::{AUTHORIZATION, HeaderValue};
    use zip::ZipArchive;
    use crate::service::order_service::OrderServiceHistBoxImpl;
    use crate::strategy::hammer_strategy::HammerStrategy;
    use crate::strategy::strategy::Strategy;
    use crate::trading_cfg::{HammerCfg, HammerStrategySettings, TrendCfg};

    fn read_quotation(data: &str) -> Quotation {
        let mut splited = data.split(".");
        Quotation {
            units: splited.next().unwrap().parse::<i64>().unwrap(),
            nano: match splited.next().get_or_insert("0").parse::<i64>() {
                Ok(x) => x as i32,
                _ => {
                    println!("Error while parsing nano in data={:#?}", data);
                    0
                }
            },
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

    async fn download_data(bearer_token: &str, dir_path: &str, instrument: &Share, year: u32) -> Result<(), Box<dyn Error>> {
        let url =
            format!("https://invest-public-api.tinkoff.ru/history-data?figi={}&instrument_uid={}&year={}",
                    instrument.figi,
                    instrument.uid,
                    year
            );

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .header(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", bearer_token))?)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .expect("GET hist data failed");

        let file_name = format!("{}/{}-{}.zip", dir_path, instrument.ticker, year);
        let file = File::create(file_name.clone())?;
        fs::write(&file_name, resp).expect("error writing file");

        println!("File {:?} downloaded successfully!", file);

        Ok(())
    }

    async fn unzip_data(dir_path: &str, instrument: &Share, year: u32) -> Result<(), Box<dyn Error>> {
        let folder_name = format!("{}/{}-{}", dir_path, instrument.ticker, year);
        let archive_name = format!("{}/{}-{}.zip", dir_path, instrument.ticker, year);
        let archive = File::open(archive_name.clone())?;
        let mut archive = ZipArchive::new(archive)?;

        fs::create_dir_all(folder_name.clone()).expect("Error creating dir for zip data hist");

        for idx in 0..archive.len() {
            let mut file = archive.by_index(idx).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => format!("{}/{}", folder_name, path.display()),
                None => continue,
            };
            let mut outfile = File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }

        fs::remove_file(archive_name.clone()).expect(&*format!("Error while remove zip file {:?}", archive_name));

        Ok(())
    }

    async fn prepare_hist_data(bearer_token: &str, dir_path: &str, instrument: &Share, year: u32, interval: SubscriptionInterval) -> Result<Vec<Candle>, Box<dyn Error>> {
        let folder_name = format!("{}/{}-{}", dir_path, instrument.ticker, year);

        if fs::read_dir(folder_name.clone()).is_err() {
            download_data(bearer_token, dir_path, instrument, 2023).await.expect("Error while download_data");
            unzip_data(dir_path, instrument, 2023).await.expect("Error while unzip_data");
        }

        let mut candles = Vec::new();
        let mut dir_entries: Vec<_> = fs::read_dir(folder_name).unwrap().map(|r| r.unwrap()).collect();
        dir_entries.sort_by_key(|dir| dir.path());
        for entry in dir_entries {
            let file_path = entry.path();
            if file_path.is_file() && file_path.extension().map(|ext| ext == "csv").unwrap_or(false) {
                let file_content = fs::read_to_string(file_path).unwrap();
                let mut csv_reader = ReaderBuilder::new().from_reader(file_content.as_bytes());
                csv_reader.records().for_each(|row| {
                    candles.push(read_candle(row.unwrap(), interval));
                });
            } else {
                eprint!("Empty dir")
            }
        }
        Ok(candles)
    }

    #[tokio::test]
    async fn test_hammer_strategy() {
        let (_, sandbox_token) = local_tokens::get_local_tokens();
        let tickers = vec!["SBER"];

        let service = TinkoffInvestService::new(sandbox_token.parse().unwrap());
        let instruments = prepare_instruments(&service, tickers).await;

        let dir_path = "./hist_data";
        fs::create_dir_all(dir_path).expect("Error creating dir_path for data hist");

        let year = 2023;
        let sorted_stream = prepare_hist_data(&sandbox_token, dir_path, instruments.get(0).unwrap(), year, SubscriptionInterval::OneMinute).await.unwrap();
        println!("stream_len={:#?}", sorted_stream.len());
        // todo cross validation like a lot of slices from sorted_stream: sorted_stream[x..y]

        let start_balance = Quotation { units: 1000, nano: 0 };
        let commission = 30_u8; // percentage
        let trash_hold = 100_u64;
        let order_service_mock = OrderServiceHistBoxImpl::new(start_balance.clone(), commission, trash_hold);

        let hammer_settings = HammerStrategySettings {
            hammer_cfg: HammerCfg {
                bottom_start: 80,
                bottom_end: 95,
                up_start: 60,
                up_end: 75,
            },
            trend_cfg: TrendCfg {
                max_candle_skip: 1,
            },
            window_size_min: 5,
        };
        let state = Arc::new(CandleState::new());
        let mut hammer_strategy = HammerStrategy::new(Arc::clone(&state), order_service_mock, instruments.get(0).unwrap().clone(), hammer_settings);

        // move systemTime to start of hist data in the past, because strategy use SystemTime::now
        let first_candle_time = sorted_stream.get(0).unwrap().time.clone().unwrap();
        MockClock::advance_system_time(Duration::from_secs(first_candle_time.seconds as u64));

        let mut i = 0;
        let now = std::time::Instant::now();
        for candle in sorted_stream {
            MockClock::advance_system_time(Duration::from_secs(60)); // increment mock_instant::SystemTime
            state.update(&candle)
                .unwrap_or_else(|err| eprintln!("Error updating last_price_state: {}", err));

            hammer_strategy.update().await.expect("Error updating first strategy");
            i += 1;
            if i > 100 { // 261933 all
                break
            }
        }

        println!("Elapsed test time: {:.10?}", now.elapsed());
        println!("HammerStrategy for
          instruments={:?},
          year={:?}
          start_balance={:?}
          trash_hold={:?}
          finished.", instruments.iter().map(|s| s.ticker.clone()).collect::<Vec<_>>(), year, start_balance, trash_hold);

        assert_eq!(true, false);
    }
}
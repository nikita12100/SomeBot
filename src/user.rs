use tinkoff_invest_api::tcs::{GetAccountsRequest, GetOrdersRequest, OpenSandboxAccountRequest, PortfolioRequest, PositionsRequest};
use tinkoff_invest_api::tcs::portfolio_request::CurrencyRequest;
use tinkoff_invest_api::TinkoffInvestService;
use crate::prepare_channel;

pub async fn sandbox_demo(service: &TinkoffInvestService) -> () {
    let channel = prepare_channel().await.unwrap();
    let mut sandbox_client = service.sandbox(channel).await.unwrap();

    // let opened_id = sandbox_client.open_sandbox_account(OpenSandboxAccountRequest{}).await.unwrap().into_inner().account_id;
    // println!("opened_id={:?}", opened_id);

    let accounts = sandbox_client.get_sandbox_accounts(GetAccountsRequest{}).await.unwrap().into_inner().accounts;
    println!("accounts={:?}", accounts);

    let orders = sandbox_client.get_sandbox_orders(GetOrdersRequest{
        account_id: accounts.get(0).unwrap().id.clone()
    }).await.unwrap().into_inner().orders;
    println!("orders={:?}", orders);

    let positions = sandbox_client.get_sandbox_positions(PositionsRequest{
        account_id: accounts.get(0).unwrap().id.clone()
    }).await.unwrap().into_inner();
    println!("positions={:?}", positions);

    let portfolio = sandbox_client.get_sandbox_portfolio(PortfolioRequest{
        account_id: accounts.get(0).unwrap().id.clone(),
        currency: CurrencyRequest::Rub as i32
    }).await.unwrap().into_inner();
    println!("portfolio={:?}", portfolio);

    ()
}
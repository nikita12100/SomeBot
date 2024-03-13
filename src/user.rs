use tinkoff_invest_api::tcs::{Account, GetAccountsRequest, GetOrdersRequest, MoneyValue, OpenSandboxAccountRequest, PortfolioRequest, PositionsRequest, PostOrderRequest, PostOrderResponse, SandboxPayInRequest};
use tinkoff_invest_api::tcs::portfolio_request::CurrencyRequest;
use tinkoff_invest_api::{DefaultInterceptor, TinkoffInvestService, TIResult};
use tinkoff_invest_api::tcs::sandbox_service_client::SandboxServiceClient;
use tinkoff_invest_api::tcs::users_service_client::UsersServiceClient;
use tonic::codegen::InterceptedService;
use tonic::{Response, Status};
use tonic::transport::Channel;
use crate::prepare_channel;


struct BrokerAccount {
    account: Account,
    client: UsersServiceClient<InterceptedService<Channel, DefaultInterceptor>>,
}

struct BrokerAccountSandbox {
    account: Account,
    client: SandboxServiceClient<InterceptedService<Channel, DefaultInterceptor>>,
}

trait BrokerAccountShow {
    async fn show_orders(&mut self);
    async fn show_positions(&mut self);
    async fn show_portfolio(&mut self);
}

impl BrokerAccountSandbox {
    // fixme in sandbox all methods in same struct, separate into trait in future
    fn new(account: Account, client: SandboxServiceClient<InterceptedService<Channel, DefaultInterceptor>>) -> Self {
        Self { account, client }
    }

    async fn pay_in(&mut self, amount: MoneyValue) {
        let _ = self.client.sandbox_pay_in(SandboxPayInRequest {
            account_id: self.account.id.clone(),
            amount: Some(amount),
        }).await;
    }
    async fn post_order(&mut self, request: PostOrderRequest) -> Result<Response<PostOrderResponse>, Status> {
        self.client.post_sandbox_order(request).await
    }
}

impl BrokerAccountShow for BrokerAccountSandbox {
    async fn show_orders(&mut self) {
        let orders = self.client.get_sandbox_orders(GetOrdersRequest {
            account_id: self.account.id.clone()
        }).await.unwrap().into_inner().orders;
        println!("orders={:#?}", orders);
        println!("--------------------> ORDERS <--------------------");
        println!("{:#?}", orders);
        println!("<-------------------- ORDERS -------------------->");
    }

    async fn show_positions(&mut self) {
        let positions = self.client.get_sandbox_positions(PositionsRequest {
            account_id: self.account.id.clone()
        }).await.unwrap().into_inner();
        println!("positions={:#?}", positions);
        println!("--------------------> POSITIONS <--------------------");
        println!("{:#?}", positions);
        println!("<-------------------- POSITIONS -------------------->");
    }

    async fn show_portfolio(&mut self) {
        let portfolio = self.client.get_sandbox_portfolio(PortfolioRequest {
            account_id: self.account.id.clone(),
            currency: CurrencyRequest::Rub as i32,
        }).await.unwrap().into_inner();
        println!("--------------------> PORTFOLIO <--------------------");
        println!("{:#?}", portfolio);
        println!("<-------------------- PORTFOLIO -------------------->");
    }
}

pub async fn sandbox_user_demo(service: &TinkoffInvestService) -> () {
    let channel = prepare_channel().await.unwrap();
    let mut sandbox_client = service.sandbox(channel).await.unwrap();

    // let opened_id = sandbox_client.open_sandbox_account(OpenSandboxAccountRequest{}).await.unwrap().into_inner().account_id;
    // println!("opened_id={:?}", opened_id);

    let accounts = sandbox_client.get_sandbox_accounts(GetAccountsRequest {}).await.unwrap().into_inner().accounts;
    println!("accounts={:#?}", accounts);

    let mut broker_account = BrokerAccountSandbox::new(
        accounts.get(0).unwrap().clone(),
        sandbox_client,
    );

    // sandbox_client.sandbox_pay_in(SandboxPayInRequest {
    //     account_id: accounts.get(0).unwrap().id.clone(),
    //     amount: Some(MoneyValue{
    //         currency: "RUB".to_string(),
    //         units: 1000,
    //         nano: 0,
    //     }),
    // }).await.expect("TODO: panic message");

    broker_account.show_orders().await;
    broker_account.show_positions().await;
    broker_account.show_portfolio().await;

    ()
}
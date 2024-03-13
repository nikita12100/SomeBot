use tinkoff_invest_api::tcs::{Account, GetOrdersRequest, PortfolioRequest, PositionsRequest};
use tinkoff_invest_api::tcs::portfolio_request::CurrencyRequest;
use tinkoff_invest_api::DefaultInterceptor;
use tinkoff_invest_api::tcs::sandbox_service_client::SandboxServiceClient;
use tinkoff_invest_api::tcs::users_service_client::UsersServiceClient;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;

pub trait BrokerAccountService {
    async fn show_orders(&mut self);
    async fn show_positions(&mut self);
    async fn show_portfolio(&mut self);
}

pub struct BrokerAccountImpl {
    account: Account,
    client: UsersServiceClient<InterceptedService<Channel, DefaultInterceptor>>,
}

pub struct BrokerAccountSandboxImpl {
    account: Account,
    client: SandboxServiceClient<InterceptedService<Channel, DefaultInterceptor>>,
}

impl BrokerAccountSandboxImpl {
    pub fn new(account: Account, client: SandboxServiceClient<InterceptedService<Channel, DefaultInterceptor>>) -> Self {
        Self { account, client }
    }
}

impl BrokerAccountService for BrokerAccountSandboxImpl {
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

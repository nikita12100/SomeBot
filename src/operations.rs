use tinkoff_invest_api::DefaultInterceptor;
use tinkoff_invest_api::tcs::{Account, PortfolioRequest, PortfolioResponse, PositionsRequest, PositionsResponse};
use tinkoff_invest_api::tcs::operations_service_client::OperationsServiceClient;
use tinkoff_invest_api::tcs::portfolio_request::CurrencyRequest;
use tinkoff_invest_api::tcs::sandbox_service_client::SandboxServiceClient;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;

pub trait OperationsService {
    async fn get_portfolio(&mut self) -> PortfolioResponse;
    async fn get_positions(&mut self) -> PositionsResponse;

}

pub struct OperationsServiceImpl {
    account: Account,
    client: OperationsServiceClient<InterceptedService<Channel, DefaultInterceptor>>,
}

pub struct OperationsServiceSandBoxImpl {
    account: Account,
    client: SandboxServiceClient<InterceptedService<Channel, DefaultInterceptor>>,
}

impl OperationsServiceImpl {
    pub fn new(account: Account, client: OperationsServiceClient<InterceptedService<Channel, DefaultInterceptor>>) -> Self {
        Self { account, client }
    }
}

impl OperationsServiceSandBoxImpl {
    pub fn new(account: Account, client: SandboxServiceClient<InterceptedService<Channel, DefaultInterceptor>>) -> Self {
        Self { account, client }
    }
}

impl OperationsService for OperationsServiceSandBoxImpl {
    async fn get_portfolio(&mut self) -> PortfolioResponse {
        let portfolio = self.client.get_sandbox_portfolio(PortfolioRequest {
            account_id: self.account.id.clone(),
            currency: CurrencyRequest::Rub as i32,
        }).await.unwrap().into_inner();
        println!("--------------------> PORTFOLIO <--------------------");
        println!("{:#?}", portfolio);
        println!("<-------------------- PORTFOLIO -------------------->");
        println!();
        portfolio
    }

    async fn get_positions(&mut self) -> PositionsResponse {
        let positions = self.client.get_sandbox_positions(PositionsRequest {
            account_id: self.account.id.clone()
        }).await.unwrap().into_inner();
        println!("--------------------> POSITIONS <--------------------");
        println!("{:#?}", positions);
        println!("<-------------------- POSITIONS -------------------->");
        println!();
        positions
    }
}
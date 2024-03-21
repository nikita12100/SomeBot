use duplicate::duplicate_item;
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

#[duplicate_item(
service_impl                      _get_portfolio              _get_positions;
[ OperationsServiceImpl ]         [ get_portfolio ]          [ get_positions ];
[ OperationsServiceSandBoxImpl ]  [ get_sandbox_portfolio ]  [ get_sandbox_positions ];
)]
impl OperationsService for service_impl {
    async fn get_portfolio(&mut self) -> PortfolioResponse {
        self.client._get_portfolio(PortfolioRequest {
            account_id: self.account.id.clone(),
            currency: CurrencyRequest::Rub as i32,
        }).await.unwrap().into_inner()
    }

    async fn get_positions(&mut self) -> PositionsResponse {
        self.client._get_positions(PositionsRequest {
            account_id: self.account.id.clone()
        }).await.unwrap().into_inner()
    }
}
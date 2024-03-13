use tinkoff_invest_api::tcs::{Account, GetOrdersRequest, PortfolioRequest, PositionsRequest};
use tinkoff_invest_api::tcs::portfolio_request::CurrencyRequest;
use tinkoff_invest_api::DefaultInterceptor;
use tinkoff_invest_api::tcs::sandbox_service_client::SandboxServiceClient;
use tinkoff_invest_api::tcs::users_service_client::UsersServiceClient;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;

pub trait BrokerAccountService {

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


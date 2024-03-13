use std::sync::{Arc, RwLock};
use tinkoff_invest_api::DefaultInterceptor;
use tinkoff_invest_api::tcs::{Account, OrderDirection, OrderType, PostOrderRequest, PostOrderResponse, Quotation};
use tinkoff_invest_api::tcs::orders_service_client::OrdersServiceClient;
use tinkoff_invest_api::tcs::sandbox_service_client::SandboxServiceClient;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub figi: String,
    pub quantity: i64,
    pub price: Option<Quotation>,
    pub direction: OrderDirection,
    pub order_type: OrderType,
    pub order_id: String,
    pub instrument_id: String,
}

pub trait OrderService {
    async fn order(&mut self, request: OrderRequest) -> Result<tonic::Response<PostOrderResponse>, tonic::Status>;
}

pub struct OrderServiceImpl {
    account: Account,
    client: OrdersServiceClient<InterceptedService<Channel, DefaultInterceptor>>,
}

pub struct OrderServiceSandboxImpl {
    account: Account,
    client: SandboxServiceClient<InterceptedService<Channel, DefaultInterceptor>>,
}

impl OrderServiceImpl {
    pub fn new(account: Account, client: OrdersServiceClient<InterceptedService<Channel, DefaultInterceptor>>) -> Self {
        Self { account, client }
    }
}

impl OrderServiceSandboxImpl {
    pub fn new(account: Account, client: SandboxServiceClient<InterceptedService<Channel, DefaultInterceptor>>) -> Self {
        Self { account, client }
    }
}

impl OrderService for OrderServiceSandboxImpl {
    async fn order(&mut self, request: OrderRequest) -> Result<tonic::Response<PostOrderResponse>, tonic::Status> {
        println!("======== Order =======");
        println!("request={:#?}", request);
        let response = self.client.post_sandbox_order(PostOrderRequest {
            figi: request.figi,
            quantity: request.quantity,
            price: request.price,
            direction: request.direction as i32,
            account_id: self.account.id.clone(),
            order_type: request.order_type as i32,
            order_id: request.order_id,
            instrument_id: request.instrument_id,
        }).await;
        println!("Got response {:#?}", response);
        response
    }
}

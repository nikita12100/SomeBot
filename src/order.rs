use tinkoff_invest_api::DefaultInterceptor;
use tinkoff_invest_api::tcs::{Account, GetOrdersRequest, GetOrdersResponse, MoneyValue, OrderDirection, OrderState, OrderType, PostOrderRequest, PostOrderResponse, Quotation, SandboxPayInRequest, Share};
use tinkoff_invest_api::tcs::orders_service_client::OrdersServiceClient;
use tinkoff_invest_api::tcs::sandbox_service_client::SandboxServiceClient;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;
use uuid::Uuid;
use crate::strategy::strategy::OpenedPattern;

pub trait OrderService {
    async fn order_buy(&mut self, figi: String, instrument_id: String, quantity: i64, price: Option<Quotation>, order_type: OrderType) -> Result<tonic::Response<PostOrderResponse>, tonic::Status>;
    async fn order_sell(&mut self, figi: String, instrument_id: String, quantity: i64, price: Option<Quotation>, order_type: OrderType) -> Result<tonic::Response<PostOrderResponse>, tonic::Status>;
    async fn get_orders(&mut self) -> Vec<OrderState>;
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
    async fn pay_in(&mut self, amount: MoneyValue) {
        let _ = self.client.sandbox_pay_in(SandboxPayInRequest {
            account_id: self.account.id.clone(),
            amount: Some(amount),
        }).await;
    }
}

impl OrderService for OrderServiceSandboxImpl {
    async fn order_buy(&mut self, figi: String, instrument_id: String, quantity: i64, price: Option<Quotation>, order_type: OrderType) -> Result<tonic::Response<PostOrderResponse>, tonic::Status> {
        println!("======== Order =======");
        println!("request={:#?}, {:#?}, {:#?}, {:#?}, {:#?}", figi, instrument_id, quantity, price, order_type);
        let response = self.client.post_sandbox_order(PostOrderRequest {
            figi: figi,
            quantity: quantity,
            price: price,
            direction: OrderDirection::Buy as i32,
            account_id: self.account.id.clone(),
            order_type: order_type as i32,
            order_id: Uuid::new_v4().to_string(),
            instrument_id: instrument_id,
        }).await;
        println!("Got response {:#?}", response);
        response
    }

    async fn order_sell(&mut self, figi: String, instrument_id: String, quantity: i64, price: Option<Quotation>, order_type: OrderType) -> Result<tonic::Response<PostOrderResponse>, tonic::Status> {
        println!("======== Order =======");
        println!("request={:#?}, {:#?}, {:#?}, {:#?}, {:#?}", figi, instrument_id, quantity, price, order_type);
        let response = self.client.post_sandbox_order(PostOrderRequest {
            figi: figi,
            quantity: quantity,
            price: price,
            direction: OrderDirection::Sell as i32,
            account_id: self.account.id.clone(),
            order_type: order_type as i32,
            order_id: Uuid::new_v4().to_string(),
            instrument_id: instrument_id,
        }).await;
        println!("Got response {:#?}", response);
        response
    }

    async fn get_orders(&mut self) -> Vec<OrderState> {
        let orders = self.client.get_sandbox_orders(GetOrdersRequest {
            account_id: self.account.id.clone()
        }).await.unwrap().into_inner().orders;
        println!("--------------------> ORDERS <--------------------");
        println!("{:#?}", orders);
        println!("<-------------------- ORDERS -------------------->");
        println!();
        orders
    }
}

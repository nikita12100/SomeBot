use tinkoff_invest_api::DefaultInterceptor;
use tinkoff_invest_api::tcs::{Account, GetOrdersRequest, LastPrice, MoneyValue, OrderDirection, OrderState, OrderType, PostOrderRequest, PostOrderResponse, Quotation, SandboxPayInRequest};
use tinkoff_invest_api::tcs::orders_service_client::OrdersServiceClient;
use tinkoff_invest_api::tcs::sandbox_service_client::SandboxServiceClient;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;
use uuid::Uuid;
use duplicate::duplicate_item;
use tonic::{Code, Response, Status};
use crate::utils::quotation::QuotationExtension;

pub trait OrderService {
    async fn order_buy(&mut self, figi: String, instrument_id: String, quantity: i64, price: Option<Quotation>, order_type: OrderType) -> Result<Response<PostOrderResponse>, Status>;
    async fn order_sell(&mut self, figi: String, instrument_id: String, quantity: i64, price: Option<Quotation>, order_type: OrderType) -> Result<Response<PostOrderResponse>, Status>;
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

pub struct OrderServiceHistBoxImpl {
    commission: u8,
    // percentage 0-100
    pub balance: Quotation,
    // fixme map<instrument, Quotation> for multi instruments
    trash_hold: u64,
    pub current_price: Quotation,
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

impl OrderServiceHistBoxImpl {
    pub fn new(balance: Quotation, commission: u8, trash_hold: u64) -> Self { Self { commission, balance, trash_hold, current_price: Quotation { units: 0, nano: 0 } } }
    pub fn get_balance(&self) -> Quotation { self.balance.clone() }
}

#[duplicate_item(
service_impl                 _post_order             _get_orders;
[ OrderServiceImpl ]         [ post_order ]          [ get_orders ];
[ OrderServiceSandboxImpl ]  [ post_sandbox_order ]  [ get_sandbox_orders ];
)]
impl OrderService for service_impl {
    async fn order_buy(&mut self, figi: String, instrument_id: String, quantity: i64, price: Option<Quotation>, order_type: OrderType) -> Result<Response<PostOrderResponse>, Status> {
        self.client._post_order(PostOrderRequest {
            figi: figi,
            quantity: quantity,
            price: price,
            direction: OrderDirection::Buy as i32,
            account_id: self.account.id.clone(),
            order_type: order_type as i32,
            order_id: Uuid::new_v4().to_string(),
            instrument_id: instrument_id,
        }).await
    }

    async fn order_sell(&mut self, figi: String, instrument_id: String, quantity: i64, price: Option<Quotation>, order_type: OrderType) -> Result<Response<PostOrderResponse>, Status> {
        self.client._post_order(PostOrderRequest {
            figi: figi,
            quantity: quantity,
            price: price,
            direction: OrderDirection::Sell as i32,
            account_id: self.account.id.clone(),
            order_type: order_type as i32,
            order_id: Uuid::new_v4().to_string(),
            instrument_id: instrument_id,
        }).await
    }

    async fn get_orders(&mut self) -> Vec<OrderState> {
        self.client._get_orders(GetOrdersRequest {
            account_id: self.account.id.clone()
        }).await.unwrap().into_inner().orders
    }
}

impl OrderService for OrderServiceHistBoxImpl {
    async fn order_buy(&mut self, figi: String, instrument_id: String, quantity: i64, _price: Option<Quotation>, order_type: OrderType) -> Result<Response<PostOrderResponse>, Status> {
        if self.balance.units < self.trash_hold as i64 {
            panic!("Strategy lost: trying buy when balance:{:?} < trash_hold:{:?}", self.balance.units, self.trash_hold)
        }
        if self.balance.units > 0 && self.balance.nano >= 0 && quantity > 0 {
            let price = if _price.is_some() || order_type != OrderType::Market {
                panic!("Only market order support in hist order service")
            } else {
                self.current_price.clone()
            };

            if self.balance.units - price.clone().units * quantity < 0 {
                panic!("Not enough money for buy: balance={:#?}, price={:#?}", self.balance, price)
            }

            self.balance = (self.balance.wr() - price.clone().wr() * quantity).uwr();
            let commission = (self.commission as f64 / 100.0) as i64;
            self.balance = (self.balance.wr() - price.clone().wr() * quantity * commission).uwr();

            while self.balance.nano < 0 {
                self.balance.units -= 1;
                self.balance.nano += 1_000_000_000;
            }

            println!(" ======================== New balance after buy={},{} ========================", self.balance.units, self.balance.nano);

            Ok(Response::new(PostOrderResponse {
                order_id: "hist".to_string(),
                execution_report_status: 0,
                lots_requested: quantity,
                lots_executed: quantity,
                initial_order_price: None,
                executed_order_price: Some(MoneyValue {
                    currency: "".to_string(),
                    units: price.clone().units,
                    nano: price.clone().nano,
                }),
                total_order_amount: None,
                initial_commission: None,
                executed_commission: Some(MoneyValue {
                    currency: "".to_string(),
                    units: self.commission as i64,
                    nano: 0,
                }),
                aci_value: None,
                figi,
                direction: OrderDirection::Buy as i32,
                initial_security_price: None,
                order_type: 0,
                message: "hist training buy".to_string(),
                initial_order_price_pt: None,
                instrument_uid: instrument_id.clone(),
            }))
        } else {
            Err(Status::new(
                Code::Cancelled,
                format!("Not enough money balance={:?} while buying instrument_id={:?}, quantity={:?}, price={:?} ", self.balance, instrument_id, quantity, _price),
            ))
        }
    }

    async fn order_sell(&mut self, figi: String, instrument_id: String, quantity: i64, _price: Option<Quotation>, order_type: OrderType) -> Result<Response<PostOrderResponse>, Status> {
        if quantity > 0 {

            let price = if _price.is_some() || order_type != OrderType::Market {
                panic!("Only market order support in hist order service")
            } else {
                self.current_price.clone()
            };

            self.balance = (self.balance.wr() + price.clone().wr() * quantity).uwr();
            let commission = (self.commission as f64 / 100.0) as i64;
            self.balance = (self.balance.wr() - price.clone().wr() * quantity * commission).uwr();

            println!(" ======================== New balance after sell={},{} ========================", self.balance.units, self.balance.nano);

            Ok(Response::new(PostOrderResponse {
                order_id: "hist".to_string(),
                execution_report_status: 0,
                lots_requested: quantity,
                lots_executed: quantity,
                initial_order_price: None,
                executed_order_price: Some(MoneyValue {
                    currency: "".to_string(),
                    units: price.clone().units,
                    nano: price.clone().nano,
                }),
                total_order_amount: None,
                initial_commission: None,
                executed_commission: Some(MoneyValue {
                    currency: "".to_string(),
                    units: self.commission as i64,
                    nano: 0,
                }),
                aci_value: None,
                figi,
                direction: OrderDirection::Sell as i32,
                initial_security_price: None,
                order_type: 0,
                message: "hist training sell".to_string(),
                initial_order_price_pt: None,
                instrument_uid: instrument_id.clone(),
            }))
        } else {
            Err(Status::new(
                Code::Cancelled,
                format!("Incorrect price while selling instrument_id={:?}, quantity={:?}, price={:?} ", instrument_id, quantity, _price),
            ))
        }
    }

    async fn get_orders(&mut self) -> Vec<OrderState> {
        Vec::new()
    }
}

use tinkoff_invest_api::tcs::{OrderDirection, OrderType, PortfolioPosition, PortfolioResponse, PositionsResponse, PositionsSecurities, PostOrderRequest, PostOrderResponse, Quotation, Share};

pub trait Strategy {
    type Statistic;
    async fn warm_up(&self, positions: PortfolioResponse) -> Result<(), Box<dyn std::error::Error>>;
    // main logic here. Updating buy/sell signal and making buy/sell orders.
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn signal_buy(&self, stat: &Self::Statistic) -> Vec<OpenedPattern>;
    async fn check_pattern(&self, instrument: &Share, stat: &Self::Statistic) -> Option<OpenedPattern>;
    async fn signal_sell(&self, stat: &Self::Statistic) -> Vec<OpenedPattern>;
}

#[derive(Debug, Clone)]
pub struct OpenedPattern {
    pub figi: String,
    pub quantity: i64,
    pub price: Option<Quotation>,
    pub instrument_id: String,
}

pub fn map_position_to_pattern(position: PortfolioPosition) -> OpenedPattern {
    OpenedPattern {
        figi: position.figi,
        quantity: position.quantity.unwrap().units,
        price: Option::from(Quotation {
            units: position.average_position_price.clone().unwrap().units,
            nano: position.average_position_price.unwrap().nano,
        }),
        instrument_id: position.instrument_uid,
    }
}

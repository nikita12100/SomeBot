use std::fmt;
use tinkoff_invest_api::tcs::{PostOrderRequest, PostOrderResponse, Share};
use crate::order::OrderRequest;

pub trait Strategy {
    type Statistic;
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn signal_buy(&self, stat: &Self::Statistic) -> Vec<OrderRequest>;
    async fn check_pattern(&self, instrument: &Share, stat: &Self::Statistic) -> Option<OrderRequest>;
    async fn signal_sell(&self, stat: &Self::Statistic) -> Vec<PostOrderResponse>;
}

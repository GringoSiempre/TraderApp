// use crate::market_data::MarketData;
pub trait Subscriber {
    fn on_data(&mut self, data: &str);
}

pub trait Publisher {
    fn subscribe(&mut self, subscriber: Box<dyn Subscriber>);
    fn notify_subscribers(&self, data: &str);
}
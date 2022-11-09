use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::mpsc::{Receiver, Sender};

pub type Price = Decimal;
pub type Quantity = Decimal;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Quote {
    pub price: Price,
    pub quantity: Quantity,
}

impl Quote {
    pub fn new(price: Price, quantity: Quantity) -> Self {
        Self { price, quantity }
    }
}

pub type OrderbookUpdate = (String, Vec<Quote>, Vec<Quote>);
pub type OrderbookUpdateSender = Sender<OrderbookUpdate>;
pub type OrderbookUpdateReceiver = Receiver<OrderbookUpdate>;

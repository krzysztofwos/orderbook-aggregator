use rust_decimal::Decimal;
use tokio::sync::mpsc::{Receiver, Sender};

pub type Price = Decimal;
pub type Quantity = Decimal;
pub type Quote = (Price, Quantity);
pub type OrderbookUpdate = (String, Vec<Quote>, Vec<Quote>);
pub type OrderbookUpdateSender = Sender<OrderbookUpdate>;
pub type OrderbookUpdateReceiver = Receiver<OrderbookUpdate>;

use crate::types::{OrderbookUpdate, Price, Quantity};

pub type CombinedOrderbookLevel = (String, Price, Quantity);

fn update_side<F>(
    side: &mut Vec<(String, Price, Quantity)>,
    compare: F,
    exchange: &str,
    quotes: Vec<(Price, Quantity)>,
    orderbook_depth_limit: usize,
) where
    F: FnMut(&CombinedOrderbookLevel, &CombinedOrderbookLevel) -> std::cmp::Ordering,
{
    side.retain(|(level_exchange, _, _)| level_exchange != exchange);
    quotes.into_iter().for_each(|(price, quantity)| {
        side.push((exchange.to_string(), price, quantity));
    });
    side.sort_unstable_by(compare);
    side.truncate(orderbook_depth_limit);
}

pub struct CombinedOrderbook {
    pub bids: Vec<CombinedOrderbookLevel>,
    pub asks: Vec<CombinedOrderbookLevel>,
    spread: Option<Price>,
    orderbook_depth_limit: usize,
}

impl CombinedOrderbook {
    pub fn new(orderbook_depth_limit: usize) -> Self {
        Self {
            bids: vec![],
            asks: vec![],
            spread: None,
            orderbook_depth_limit,
        }
    }

    pub fn bids(&self) -> &[(String, Price, Quantity)] {
        &self.bids
    }

    pub fn asks(&self) -> &[(String, Price, Quantity)] {
        &self.asks
    }

    pub fn update(&mut self, orderbook_update: OrderbookUpdate) {
        let (exchange, bids, asks) = orderbook_update;
        update_side(
            &mut self.bids,
            |lhs, rhs| rhs.1.partial_cmp(&lhs.1).unwrap(), // Highest bid first
            &exchange,
            bids,
            self.orderbook_depth_limit,
        );
        update_side(
            &mut self.asks,
            |lhs, rhs| lhs.1.partial_cmp(&rhs.1).unwrap(), // Lowest ask first
            &exchange,
            asks,
            self.orderbook_depth_limit,
        );
        self.update_spread();
    }

    fn update_spread(&mut self) {
        self.spread = if self.bids.is_empty() || self.asks.is_empty() {
            None
        } else {
            Some(self.asks[0].1 - self.bids[0].1)
        }
    }

    pub fn spread(&self) -> Option<Price> {
        self.spread
    }
}

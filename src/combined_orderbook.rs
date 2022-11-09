use crate::types::{OrderbookUpdate, Price, Quantity};

pub type CombinedOrderbookLevel = (String, Price, Quantity);

fn update_side<F>(
    side: &mut Vec<CombinedOrderbookLevel>,
    compare: F,
    exchange: &str,
    quotes: Vec<(Price, Quantity)>,
) where
    F: FnMut(&CombinedOrderbookLevel, &CombinedOrderbookLevel) -> std::cmp::Ordering,
{
    side.retain(|(level_exchange, _, _)| level_exchange != exchange);
    quotes.into_iter().for_each(|(price, quantity)| {
        side.push((exchange.to_string(), price, quantity));
    });
    side.sort_unstable_by(compare);
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
        &self.bids[..self.orderbook_depth_limit]
    }

    pub fn asks(&self) -> &[(String, Price, Quantity)] {
        &self.asks[..self.orderbook_depth_limit]
    }

    pub fn update(&mut self, orderbook_update: OrderbookUpdate) {
        let (exchange, bids, asks) = orderbook_update;
        update_side(
            &mut self.bids,
            // FIXME: Sort by (price, quanity) descending
            |lhs, rhs| rhs.1.partial_cmp(&lhs.1).unwrap(), // Highest bid first
            &exchange,
            bids,
        );
        update_side(
            &mut self.asks,
            // FIXME: Sort by price ascending, quantity descending
            |lhs, rhs| lhs.1.partial_cmp(&rhs.1).unwrap(), // Lowest ask first
            &exchange,
            asks,
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

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn update_best_bid() {
        let mut combined_orderbook = CombinedOrderbook::new(1);
        combined_orderbook.update(("binance".to_string(), vec![(dec!(90), dec!(100))], vec![]));
        combined_orderbook.update(("bitstamp".to_string(), vec![(dec!(89), dec!(100))], vec![]));
        combined_orderbook.update(("binance".to_string(), vec![(dec!(87), dec!(100))], vec![]));
        assert_eq!(
            combined_orderbook.bids(),
            [("bitstamp".to_string(), dec!(89), dec!(100))]
        );
    }

    #[test]
    fn update_best_ask() {
        let mut combined_orderbook = CombinedOrderbook::new(1);
        combined_orderbook.update(("binance".to_string(), vec![], vec![(dec!(90), dec!(100))]));
        combined_orderbook.update(("bitstamp".to_string(), vec![], vec![(dec!(91), dec!(100))]));
        combined_orderbook.update(("binance".to_string(), vec![], vec![(dec!(92), dec!(100))]));
        assert_eq!(
            combined_orderbook.asks(),
            [("bitstamp".to_string(), dec!(91), dec!(100))]
        );
    }
}

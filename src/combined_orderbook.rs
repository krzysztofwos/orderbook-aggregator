use crate::types::{OrderbookUpdate, Price, Quote};

#[derive(Debug, Eq, PartialEq)]
pub struct CombinedOrderbookLevel {
    pub exchange: String,
    pub quote: Quote,
}

impl CombinedOrderbookLevel {
    pub fn new(exchange: String, quote: Quote) -> Self {
        Self { exchange, quote }
    }
}

fn update_side<F>(
    side: &mut Vec<CombinedOrderbookLevel>,
    compare: F,
    exchange: &str,
    quotes: Vec<Quote>,
) where
    F: FnMut(&CombinedOrderbookLevel, &CombinedOrderbookLevel) -> std::cmp::Ordering,
{
    side.retain(|level| level.exchange != exchange);
    quotes.into_iter().for_each(|quote| {
        side.push(CombinedOrderbookLevel {
            exchange: exchange.to_string(),
            quote,
        });
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

    pub fn bids(&self) -> &[CombinedOrderbookLevel] {
        &self.bids[..self.orderbook_depth_limit]
    }

    pub fn asks(&self) -> &[CombinedOrderbookLevel] {
        &self.asks[..self.orderbook_depth_limit]
    }

    pub fn update(&mut self, orderbook_update: OrderbookUpdate) {
        let (exchange, bids, asks) = orderbook_update;
        update_side(
            &mut self.bids,
            // FIXME: Sort by (price, quanity) descending
            |lhs, rhs| rhs.quote.price.cmp(&lhs.quote.price), // Highest bid first
            &exchange,
            bids,
        );
        update_side(
            &mut self.asks,
            // FIXME: Sort by price ascending, quantity descending
            |lhs, rhs| lhs.quote.price.cmp(&rhs.quote.price), // Lowest ask first
            &exchange,
            asks,
        );
        self.update_spread();
    }

    fn update_spread(&mut self) {
        self.spread = if self.bids.is_empty() || self.asks.is_empty() {
            None
        } else {
            Some(self.asks[0].quote.price - self.bids[0].quote.price)
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
        combined_orderbook.update((
            "binance".to_string(),
            vec![Quote::new(dec!(90), dec!(100))],
            vec![],
        ));
        combined_orderbook.update((
            "bitstamp".to_string(),
            vec![Quote::new(dec!(89), dec!(100))],
            vec![],
        ));
        combined_orderbook.update((
            "binance".to_string(),
            vec![Quote::new(dec!(87), dec!(100))],
            vec![],
        ));
        assert_eq!(
            combined_orderbook.bids(),
            [CombinedOrderbookLevel::new(
                "bitstamp".to_string(),
                Quote::new(dec!(89), dec!(100))
            )]
        );
    }

    #[test]
    fn update_best_ask() {
        let mut combined_orderbook = CombinedOrderbook::new(1);
        combined_orderbook.update((
            "binance".to_string(),
            vec![],
            vec![Quote::new(dec!(90), dec!(100))],
        ));
        combined_orderbook.update((
            "bitstamp".to_string(),
            vec![],
            vec![Quote::new(dec!(91), dec!(100))],
        ));
        combined_orderbook.update((
            "binance".to_string(),
            vec![],
            vec![Quote::new(dec!(92), dec!(100))],
        ));
        assert_eq!(
            combined_orderbook.asks(),
            [CombinedOrderbookLevel::new(
                "bitstamp".to_string(),
                Quote::new(dec!(91), dec!(100))
            )]
        );
    }
}

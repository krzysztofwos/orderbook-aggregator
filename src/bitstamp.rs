use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::types::{OrderbookUpdateSender, Quote};

#[derive(Debug, Deserialize)]
pub struct BitstampEvent<T> {
    pub data: T,
    pub channel: String,
    pub event: String,
}

#[derive(Debug, Deserialize)]
pub struct BitstampOrderbookEventData {
    pub timestamp: String,
    pub microtimestamp: String,
    pub bids: Vec<Quote>,
    pub asks: Vec<Quote>,
}

pub async fn bitstamp_orderbook_listener(
    websocket_url: &str,
    symbol: &str,
    depth_limit: usize,
    tx: OrderbookUpdateSender,
) -> Result<()> {
    let (ws_stream, _) = connect_async(websocket_url).await?;
    let (mut ws_write, mut ws_read) = ws_stream.split();
    ws_write
        .send(Message::Text(serde_json::to_string(&json!(
            {
                "event": "bts:subscribe",
                "data": {
                    "channel": format!("order_book_{}", symbol.to_lowercase()),
                }
            }
        ))?))
        .await?;
    let _subscription_request_response = ws_read.next().await.unwrap()?; // FIXME

    loop {
        let message = ws_read.next().await.unwrap()?; // FIXME

        if message.is_ping() {
            ws_write.send(Message::Pong(message.into_data())).await?;
            continue;
        }

        if message.is_text() {
            let event: BitstampEvent<BitstampOrderbookEventData> =
                serde_json::from_slice(&message.into_data())?;
            let mut bids = event.data.bids;
            bids.truncate(depth_limit);
            let mut asks = event.data.asks;
            asks.truncate(depth_limit);
            tx.send(("bitstamp".to_string(), bids, asks)).await?;
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn deserialize_bitstamp_orderbook_event() {
        let json = r#"{
            "data":{
                "timestamp":"1666190126",
                "microtimestamp":"1666190126442462",
                "bids":[["19176","0.39108459"]],
                "asks":[["19181","0.31188246"]]
            },
            "channel":"order_book_btcusdt",
            "event":"data"
        }"#;
        let event: Result<BitstampEvent<BitstampOrderbookEventData>, serde_json::Error> =
            serde_json::from_str(json);
        assert!(event.is_ok());
        let data = event.unwrap().data;
        assert_eq!(data.bids, vec![Quote::new(dec!(19176.0), dec!(0.39108459))]);
        assert_eq!(data.asks, vec![Quote::new(dec!(19181.0), dec!(0.31188246))]);
    }
}

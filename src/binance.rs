use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::types::{OrderbookUpdateSender, Quote};

#[derive(Debug, Deserialize)]
pub struct BinanceBookDepth {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<Quote>,
    pub asks: Vec<Quote>,
}

pub async fn binance_orderbook_listener(
    websocket_url: &str,
    symbol: &str,
    update_interval: u64,
    depth_limit: usize,
    tx: OrderbookUpdateSender,
) -> Result<()> {
    let (ws_stream, _) = connect_async(websocket_url).await?;
    let (mut ws_write, mut ws_read) = ws_stream.split();
    ws_write
        .send(Message::Text(serde_json::to_string(&json!(
        {
            "method": "SUBSCRIBE",
            "params": [format!("{}@depth20@{}ms", symbol.to_lowercase(), update_interval)],
            "id": 0
        }))?))
        .await?;
    let _subscription_request_response = ws_read.next().await.unwrap()?; // FIXME

    loop {
        let message = ws_read.next().await.unwrap()?; // FIXME

        if message.is_ping() {
            ws_write.send(Message::Pong(message.into_data())).await?;
            continue;
        }

        if message.is_text() {
            let book_depth: BinanceBookDepth = serde_json::from_slice(&message.into_data())?;
            let mut bids = book_depth.bids;
            bids.truncate(depth_limit);
            let mut asks = book_depth.asks;
            asks.truncate(depth_limit);
            tx.send(("binance".to_string(), bids, asks)).await?;
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn deserialize_binance_book_depth() {
        let json = r#"{
            "lastUpdateId":25945836327,
            "bids":[["19255.06000000","0.10000000"]],
            "asks":[["19255.30000000","0.00055000"]]
        }"#;
        let book_depth: Result<BinanceBookDepth, serde_json::Error> = serde_json::from_str(json);
        assert!(book_depth.is_ok());
        let book_depth = book_depth.unwrap();
        assert_eq!(
            book_depth.bids,
            vec![(dec!(19255.06000000), dec!(0.10000000))]
        );
        assert_eq!(
            book_depth.asks,
            vec![(dec!(19255.30000000), dec!(0.00055000))]
        );
    }
}

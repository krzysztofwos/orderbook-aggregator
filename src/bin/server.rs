pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use std::pin::Pin;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use futures::Stream;
use rust_decimal::prelude::ToPrimitive;
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

use orderbook::{
    orderbook_aggregator_server::{self, OrderbookAggregatorServer},
    Empty, Level, Summary,
};

use orderbook_aggregator::{
    binance::binance_orderbook_listener,
    bitstamp::bitstamp_orderbook_listener,
    combined_orderbook::{CombinedOrderbook, CombinedOrderbookLevel},
    types::OrderbookUpdate,
};

#[derive(Debug)]
pub struct OrderbookAggregator {
    summary_broadcast_tx: broadcast::Sender<Summary>,
}

#[tonic::async_trait]
impl orderbook_aggregator_server::OrderbookAggregator for OrderbookAggregator {
    type BookSummaryStream = Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send>>;

    async fn book_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let mut summary_broadcast_rx = self.summary_broadcast_tx.subscribe();
        // FIXME: Is it possible to avoid the extra channel here?
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            loop {
                let summary = summary_broadcast_rx.recv().await.unwrap(); // FIXME
                tx.send(Ok(summary)) // FIXME: Use `try_send` to prevent broadcast receiver from falling behind?
                    .await
                    .expect("receive half of the output channel is closed"); // FIXME
            }
        });
        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::BookSummaryStream
        ))
    }
}

/// Orderbook Aggregator Server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Address to listen on
    #[arg(long, default_value = "0.0.0.0")]
    address: String,

    /// Port to listen on
    #[arg(long, default_value_t = 50051)]
    port: u16,

    /// Orderbook depth limit
    #[arg(long, default_value_t = 10)]
    orderbook_depth_limit: usize,

    /// Orderbook listener channel capacity
    #[arg(long, default_value_t = 128)]
    orderbook_listener_channel_capacity: usize,

    /// Summary broadcast channel capacity
    #[arg(long, default_value_t = 128)]
    summary_broadcast_channel_capacity: usize,

    /// Binance symbol
    #[arg(long, default_value = "BTCUSDT")]
    binance_symbol: String,

    /// Binance WebSocket URL
    #[arg(long, default_value = "wss://stream.binance.com:9443/ws")]
    binance_websocket_url: String,

    /// Bitstamp symbol
    #[arg(long, default_value = "BTCUSDT")]
    bitstamp_symbol: String,

    /// Bitstamp WebSocket URL
    #[arg(long, default_value = "wss://ws.bitstamp.net")]
    bitstamp_websocket_url: String,
}

fn spawn_binance_orderbook_listener(
    args: &Args,
    tx: mpsc::Sender<OrderbookUpdate>,
) -> JoinHandle<Result<()>> {
    tokio::spawn({
        let websocket_url = args.binance_websocket_url.clone();
        let symbol = args.binance_symbol.clone();
        let orderbook_depth_limit = args.orderbook_depth_limit;
        async move {
            binance_orderbook_listener(&websocket_url, &symbol, orderbook_depth_limit, tx)
                .await
                .context("Binance orderbook listener error")
        }
    })
}

fn spawn_bitstamp_orderbook_listener(
    args: &Args,
    tx: mpsc::Sender<OrderbookUpdate>,
) -> JoinHandle<Result<()>> {
    tokio::spawn({
        let websocket_url = args.bitstamp_websocket_url.clone();
        let symbol = args.bitstamp_symbol.clone();
        let orderbook_depth_limit = args.orderbook_depth_limit;
        async move {
            bitstamp_orderbook_listener(&websocket_url, &symbol, orderbook_depth_limit, tx)
                .await
                .context("Bitstamp orderbook listener error")
        }
    })
}

fn to_level(combined_orderbook_level: &CombinedOrderbookLevel) -> Result<Level> {
    let (exchange, price, amount) = combined_orderbook_level;
    Ok(Level {
        exchange: exchange.clone(),
        price: price
            .to_f64()
            .ok_or_else(|| anyhow!("value '{}' cannot be represented by an f64", price))?,
        amount: amount
            .to_f64()
            .ok_or_else(|| anyhow!("value '{}' cannot be represented by an f64", amount))?,
    })
}

fn to_summary(combined_orderbook: &CombinedOrderbook) -> Result<Summary> {
    let spread = match combined_orderbook.spread() {
        Some(spread) => spread
            .to_f64()
            .ok_or_else(|| anyhow!("value '{}' cannot be represented by an f64", spread))?,
        None => f64::NAN,
    };
    let mut bids = vec![];
    bids.reserve(combined_orderbook.bids().len());

    for bid in combined_orderbook.bids() {
        bids.push(to_level(bid)?);
    }

    let mut asks = vec![];
    asks.reserve(combined_orderbook.asks().len());

    for ask in combined_orderbook.asks() {
        asks.push(to_level(ask)?);
    }

    Ok(Summary { spread, bids, asks })
}

fn spawn_summary_publisher(
    mut orderbook_updates_rx: mpsc::Receiver<OrderbookUpdate>,
    summary_broadcast_tx: broadcast::Sender<Summary>,
    orderbook_depth_limit: usize,
) -> JoinHandle<Result<()>> {
    let mut combined_orderbook = CombinedOrderbook::new(orderbook_depth_limit);
    tokio::spawn({
        async move {
            while let Some(orderbook_update) = orderbook_updates_rx.recv().await {
                combined_orderbook.update(orderbook_update);
                let summary = to_summary(&combined_orderbook)?; // FIXME
                summary_broadcast_tx.send(summary)?; // FIXME
            }

            Ok(())
        }
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let addr = format!("{}:{}", args.address, args.port).parse()?;
    let (orderbook_updates_tx, orderbook_updates_rx) =
        mpsc::channel(args.orderbook_listener_channel_capacity);
    let mut binance_orderbook_listener_join_handle =
        spawn_binance_orderbook_listener(&args, orderbook_updates_tx.clone());
    let mut bitstamp_orderbook_listener_join_handle =
        spawn_bitstamp_orderbook_listener(&args, orderbook_updates_tx.clone());
    let (summary_broadcast_tx, _summary_broadcast_rx) =
        broadcast::channel(args.summary_broadcast_channel_capacity);
    let mut summary_publisher_join_handle = spawn_summary_publisher(
        orderbook_updates_rx,
        summary_broadcast_tx.clone(),
        args.orderbook_depth_limit,
    );
    let orderbook_aggregator = OrderbookAggregator {
        summary_broadcast_tx: summary_broadcast_tx.clone(),
    };
    let mut orderbook_aggregator_server_join_handle = tokio::spawn(async move {
        Server::builder()
            .add_service(OrderbookAggregatorServer::new(orderbook_aggregator))
            .serve(addr)
            .await
            .context("Orderbook aggregator server error")
    });
    tokio::select! {
        result = &mut binance_orderbook_listener_join_handle => {
            result?
        }
        result = &mut bitstamp_orderbook_listener_join_handle => {
            result?
        }
        result = &mut summary_publisher_join_handle => {
            result?
        }
        result = &mut orderbook_aggregator_server_join_handle => {
            result?.map_err(|e| anyhow!(e))
        }
    }
}

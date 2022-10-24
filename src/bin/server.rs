pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use std::{pin::Pin, time::Duration};

use anyhow::Result;
use clap::Parser;
use futures::Stream;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status};

use orderbook::{
    orderbook_aggregator_server::{self, OrderbookAggregatorServer},
    Empty, Summary,
};

use orderbook_aggregator::{binance::binance_orderbook_listener, types::OrderbookUpdate};

#[derive(Debug, Default)]
pub struct OrderbookAggregator {}

#[tonic::async_trait]
impl orderbook_aggregator_server::OrderbookAggregator for OrderbookAggregator {
    type BookSummaryStream = Pin<Box<dyn Stream<Item = Result<Summary, Status>> + Send>>;

    async fn book_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let repeat = std::iter::repeat(Summary {
            spread: f64::NAN,
            bids: vec![],
            asks: vec![],
        });
        let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(200)));
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {}
                    Err(_item) => {
                        break;
                    }
                }
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

    /// Binance symbol
    #[arg(long, default_value = "BTCUSDT")]
    binance_symbol: String,

    /// Binance WebSocket URL
    #[arg(long, default_value = "wss://stream.binance.com:9443/ws")]
    binance_websocket_url: String,
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
            binance_orderbook_listener(&websocket_url, &symbol, orderbook_depth_limit, tx).await
        }
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let addr = format!("{}:{}", args.address, args.port).parse()?;
    let (orderbook_updates_tx, mut orderbook_updates_rx) =
        mpsc::channel(args.orderbook_listener_channel_capacity);
    let _binance_orderbook_listener_join_handle =
        spawn_binance_orderbook_listener(&args, orderbook_updates_tx.clone());
    tokio::spawn({
        async move {
            while let Some(orderbook_update) = orderbook_updates_rx.recv().await {
                println!("{:?}", orderbook_update);
            }
        }
    });
    let orderbook_aggregator = OrderbookAggregator::default();
    Server::builder()
        .add_service(OrderbookAggregatorServer::new(orderbook_aggregator))
        .serve(addr)
        .await?;
    Ok(())
}

pub mod orderbook {
    tonic::include_proto!("orderbook");
}

use clap::Parser;
use tokio_stream::StreamExt;

use orderbook::{orderbook_aggregator_client::OrderbookAggregatorClient, Empty};

/// Orderbook Aggregator Client
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server URL
    #[arg(long, default_value = "htttp://0.0.0.0:50051")]
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut client = OrderbookAggregatorClient::connect(args.url).await.unwrap();
    let mut stream = client.book_summary(Empty {}).await.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        println!("{:?}", item.unwrap());
    }

    Ok(())
}

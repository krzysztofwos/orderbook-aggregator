pub mod binance;
pub mod bitstamp;
pub mod combined_orderbook;
pub mod orderbook {
    tonic::include_proto!("orderbook");
}
pub mod types;

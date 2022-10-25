# Orderbook Aggregator

## Building

Install Protocol Buffers compiler:

```bash
sudo apt update
sudo apt install -y protobuf-compiler
```

Build with Cargo:

```bash
cargo build
```

## Running

Start the server:

```bash
cargo run --bin server -- --address 0.0.0.0 --port 50051
```

Start the client:

```bash
cargo run --bin client --url "http://0.0.0.0:50051"
```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute("orderbook.Level", "#[derive(serde::Serialize)]")
        .type_attribute("orderbook.Summary", "#[derive(serde::Serialize)]")
        .compile(&["proto/orderbook.proto"], &["proto"])?;
    Ok(())
}

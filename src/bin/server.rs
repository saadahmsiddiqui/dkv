use dkv::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:6379".to_string());

    server::run(address.as_str()).await
}
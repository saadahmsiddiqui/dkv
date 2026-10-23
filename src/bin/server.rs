use dkv::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    server::run("127.0.0.1:6379").await
}

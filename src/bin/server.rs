use dkv::Cache;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    println!("RustCache listening on 127.0.0.1:6379");
    let cache = Arc::new(Mutex::new(Cache::new()));

    loop {
        let (stream, address) = listener.accept().await?;
        println!("Client connected: {address}");

        let cache = Arc::clone(&cache);
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, cache).await {
                eprintln!("Error handling client {address}: {error}");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    cache: Arc<Mutex<Cache>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut request = String::new();

    loop {
        request.clear();

        let bytes_read = reader.read_line(&mut request).await?;

        if bytes_read == 0 {
            break;
        }

        let parts: Vec<&str> = request.split_whitespace().collect();

        let response = match parts.as_slice() {
            ["PING"] => "PONG".to_string(),

            ["SET", key, value] => {
                let mut cache = cache.lock().await;
                cache.set(*key, *value);
                "OK".to_string()
            }

            ["GET", key] => {
                let cache = cache.lock().await;
                match cache.get(key) {
                    Some(value) => format!("VALUE {value}"),
                    None => "NOT_FOUND".to_string(),
                }
            }

            ["DELETE", key] => {
                let mut cache = cache.lock().await;
                if cache.delete(key) {
                    "OK".to_string()
                } else {
                    "NOT_FOUND".to_string()
                }
            }

            ["EXISTS", key] => {
                let cache = cache.lock().await;
                cache.exists(key).to_string()
            }

            _ => "ERROR unknown command or invalid arguments".to_string(),
        };

        writer.write_all(format!("{response}\n").as_bytes()).await?;
    }

    Ok(())
}

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::Cache;

pub async fn run(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(address).await?;
    println!("RustCache listening on {}", listener.local_addr()?);

    serve(listener, std::future::pending()).await
}

pub async fn serve<F>(listener: TcpListener, shutdown: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: std::future::Future<Output = ()> + Send,
{
    let cache = Arc::new(Mutex::new(Cache::new()));
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                println!("Dkv shutting down");
                break;
            }

            accepted = listener.accept() => {
                let (stream, _address) = accepted?;
                let cache = Arc::clone(&cache);

                tokio::spawn(async move {
                    if let Err(error) = handle_connection(stream, cache).await {
                        eprintln!("Client error: {error}");
                    }
                });
            }
        }
    }

    Ok(())
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
                cache.lock().await.set(*key, *value);
                "OK".to_string()
            }

            ["SET", key, value, ttl] => match ttl.parse::<u64>() {
                Ok(seconds) => {
                    cache.lock().await.set_with_ttl(
                        *key,
                        *value,
                        std::time::Duration::from_secs(seconds),
                    );
                    "OK".to_string()
                }
                Err(_) => "ERROR invalid TTL".to_string(),
            },

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

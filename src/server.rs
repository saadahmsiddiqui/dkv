use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

use crate::Cache;
use crate::persistence;

pub async fn run(address: &str) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(address).await?;
    let persistence = Arc::new(persistence::Persistence::new("dkv.log"));
    let cache = persistence.load_cache()?;
    let cache = Arc::new(RwLock::new(cache));

    println!("dkv listening on {}", listener.local_addr()?);

    serve(listener, cache, std::future::pending(), Some(persistence)).await
}

pub async fn serve<F>(
    listener: TcpListener,
    cache: Arc<RwLock<Cache>>,
    shutdown: F,
    persistence: Option<Arc<persistence::Persistence>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: std::future::Future<Output = ()> + Send,
{
    serve_inner(listener, shutdown, cache, persistence).await
}

async fn serve_inner<F>(
    listener: TcpListener,
    shutdown: F,
    cache: Arc<RwLock<Cache>>,
    persistence: Option<Arc<persistence::Persistence>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: std::future::Future<Output = ()> + Send,
{
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                println!("dkv shutting down");
                break;
            }

            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let cache = Arc::clone(&cache);
                let persistence = persistence.clone();

                tokio::spawn(async move {
                    if let Err(error) =
                        handle_connection(stream, cache, persistence).await
                    {
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
    cache: Arc<RwLock<Cache>>,
    persistence: Option<Arc<persistence::Persistence>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut request = String::new();
    let p = persistence.as_ref();

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
                let _ = p.unwrap().append_set(key, value);
                cache.write().await.set(*key, *value);
                "OK".to_string()
            }

            ["SET", key, value, ttl] => match ttl.parse::<u64>() {
                Ok(seconds) => {
                    let _ = p.unwrap().append_set(key, value);
                    cache.write().await.set_with_ttl(
                        *key,
                        *value,
                        std::time::Duration::from_secs(seconds),
                    );
                    "OK".to_string()
                }
                Err(_) => "ERROR invalid TTL".to_string(),
            },

            ["GET", key] => {
                let cache = cache.read().await;
                match cache.get(key) {
                    Some(value) => format!("VALUE {value}"),
                    None => "NOT_FOUND".to_string(),
                }
            }

            ["DELETE", key] => {
                let _ = p.unwrap().append_delete(key);
                let mut cache = cache.write().await;
                if cache.delete(key) {
                    "OK".to_string()
                } else {
                    "NOT_FOUND".to_string()
                }
            }

            ["EXISTS", key] => {
                let cache = cache.read().await;
                cache.exists(key).to_string()
            }

            _ => "ERROR unknown command or invalid arguments".to_string(),
        };

        writer.write_all(format!("{response}\n").as_bytes()).await?;
    }

    Ok(())
}

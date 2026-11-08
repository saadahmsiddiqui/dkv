use std::sync::Arc;

use dkv::Cache;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, oneshot};

#[tokio::test]
async fn server_integration_test() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");

    let address = listener.local_addr().expect("get test listener address");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let cache = Arc::new(RwLock::new(Cache::new()));
    let persistence = Arc::new(dkv::persistence::Persistence::new("test.log"));

    let server = tokio::spawn(async move {
        dkv::server::serve(
            listener,
            cache,
            async move {
                let _ = shutdown_rx.await;
            },
            Some(persistence),
        )
        .await
        .expect("server should run successfully");
    });

    let stream = TcpStream::connect(address)
        .await
        .expect("connect to server");

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut response = String::new();

    writer.write_all(b"PING\n").await.expect("send PING");
    reader
        .read_line(&mut response)
        .await
        .expect("read PING response");
    assert_eq!(response.trim(), "PONG");
    response.clear();

    writer.write_all(b"SET name dkv\n").await.expect("send SET");
    reader
        .read_line(&mut response)
        .await
        .expect("read SET response");
    assert_eq!(response.trim(), "OK");
    response.clear();

    writer.write_all(b"GET name\n").await.expect("send GET");
    reader
        .read_line(&mut response)
        .await
        .expect("read GET response");

    assert_eq!(response.trim(), "VALUE dkv");
    response.clear();

    writer
        .write_all(b"SET ttl dkv 500\n")
        .await
        .expect("send SET TTL");
    reader
        .read_line(&mut response)
        .await
        .expect("read SET response");
    assert_eq!(response.trim(), "OK");
    response.clear();

    drop(writer);
    let _ = shutdown_tx.send(());
    server.await.expect("server task should finish");
}

#[tokio::test]
async fn server_handles_concurrent_clients_correctly() {
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");

    let address = listener.local_addr().expect("get local address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let cache = Arc::new(RwLock::new(Cache::new()));
    let persistence = Arc::new(dkv::persistence::Persistence::new("test.log"));

    let server = tokio::spawn(async move {
        dkv::server::serve(
            listener,
            cache,
            async move {
                let _ = shutdown_rx.await;
            },
            Some(persistence.clone()),
        )
        .await
        .expect("server should run");
    });

    let mut clients = Vec::new();

    for i in 0..20 {
        let client_address = address;

        clients.push(tokio::spawn(async move {
            let stream = TcpStream::connect(client_address)
                .await
                .expect("connect to server");

            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            let key = format!("key{i}");
            let value = format!("value{i}");

            writer
                .write_all(format!("SET {key} {value}\n").as_bytes())
                .await
                .expect("send SET");

            let mut response = String::new();
            reader.read_line(&mut response).await.expect("read SET");
            assert_eq!(response.trim(), "OK");

            writer
                .write_all(format!("GET {key}\n").as_bytes())
                .await
                .expect("send GET");

            response.clear();
            reader.read_line(&mut response).await.expect("read GET");
            assert_eq!(response.trim(), format!("VALUE {value}"));
        }));
    }

    for client in clients {
        client.await.expect("client task should finish");
    }

    let _ = shutdown_tx.send(());
    server.await.expect("server task should finish");
}

#[tokio::test]
async fn concurrent_clients_can_write_to_the_same_key() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;
    use tokio::sync::oneshot;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

    let address = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let cache = Arc::new(RwLock::new(Cache::new()));

    let persistence = Arc::new(dkv::persistence::Persistence::new("test.log"));

    let server = tokio::spawn(async move {
        dkv::server::serve(
            listener,
            cache,
            async {
                let _ = shutdown_rx.await;
            },
            Some(persistence.clone()),
        )
        .await
        .unwrap();
    });

    let mut clients = Vec::new();

    for i in 0..20 {
        let address = address.to_string();
        clients.push(tokio::spawn(async move {
            let stream = TcpStream::connect(address).await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);

            let command = format!("SET shared value{}\n", i);
            write_half.write_all(command.as_bytes()).await.unwrap();

            let mut response = String::new();
            reader.read_line(&mut response).await.unwrap();

            assert_eq!(response.trim(), "OK");
        }));
    }

    for client in clients {
        client.await.unwrap();
    }

    // All writes have completed. The final value can be from any client.
    let stream = TcpStream::connect(address).await.unwrap();
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    write_half.write_all(b"GET shared\n").await.unwrap();

    let mut response = String::new();
    reader.read_line(&mut response).await.unwrap();

    let value = response.trim().strip_prefix("VALUE ").unwrap();

    assert!(
        (0..20).any(|i| value == format!("value{}", i)),
        "Unexpected final value: {value}"
    );

    let _ = shutdown_tx.send(());
    server.await.unwrap();
}

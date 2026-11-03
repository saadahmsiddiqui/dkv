use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

#[tokio::test]
async fn server_integration_test() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener");

    let address = listener.local_addr().expect("get test listener address");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        dkv::server::serve(listener, async move {
            let _ = shutdown_rx.await;
        })
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

    let server = tokio::spawn(async move {
        dkv::server::serve(listener, async move {
            let _ = shutdown_rx.await;
        })
        .await
        .expect("server should run");
    });

    let mut clients = Vec::new();

    for i in 0..20 {
        let client_address = address.clone();

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

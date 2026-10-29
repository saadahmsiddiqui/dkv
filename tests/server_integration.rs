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

    writer.write_all(b"SET ttl dkv 500\n").await.expect("send SET TTL");
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

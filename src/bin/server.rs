use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    println!("RustCache listening on 127.0.0.1:6379");

    loop {
        let (stream, address) = listener.accept().await?;
        println!("Client connected: {address}");

        handle_connection(stream).await?;
    }
}

async fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut req_str = String::new();
    let bytes_read = reader.read_to_string(&mut req_str).await?;

    if bytes_read == 0 {
        return Ok(());
    }

    let response = match req_str.trim() {
        "PING" => "PONG\n",
        _ => "ERROR unknown command\n",
    };

    writer.write_all(response.as_bytes()).await?;

    Ok(())
}

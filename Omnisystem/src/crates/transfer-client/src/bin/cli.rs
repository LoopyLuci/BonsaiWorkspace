//! transfer-client CLI: demonstrates the real length-delimited framing over
//! an actual TCP loopback pair (the "relay" stream shape), and shows the
//! client's honest failure mode when no transport is configured.

use tokio::net::{TcpListener, TcpStream};
use transfer_client::{PeerStream, TransferClientConfig, TransferDaemonClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. With no relay/fallback configured, connect() must honestly fail
    //    rather than pretend to succeed.
    let client = TransferDaemonClient::new(TransferClientConfig {
        relay_addr: None,
        relay_token: None,
        fallback_url: None,
        stream_timeout_ms: 5_000,
    });
    match client.connect("peer-x").await {
        Err(e) => println!("connect() with no transport configured correctly failed: {e}"),
        Ok(_) => println!("unexpected success"),
    }

    // 2. Exercise the real length-delimited FrameCodec end to end over an
    //    actual TCP loopback socket, wrapped exactly like a relay session.
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let (read_half, write_half) = socket.into_split();
        let mut stream = PeerStream::new_relay("demo", "peer-server", Box::new(read_half), Box::new(write_half));

        let msg = stream.recv().await.unwrap();
        println!("server received: {}", String::from_utf8_lossy(&msg));
        stream.send(b"pong").await.unwrap();
    });

    let socket = TcpStream::connect(addr).await?;
    let (read_half, write_half) = socket.into_split();
    let mut client_stream = PeerStream::new_relay("demo", "peer-client", Box::new(read_half), Box::new(write_half));

    let reply = client_stream.exchange(b"ping").await?;
    println!("client received: {}", String::from_utf8_lossy(&reply));

    server.await?;
    Ok(())
}

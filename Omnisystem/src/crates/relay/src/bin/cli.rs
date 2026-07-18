//! Relay CLI: starts a real blind relay server on an OS-assigned port,
//! then connects two raw TCP "peers" through it end to end -- mining the
//! proof-of-work, registering, and exchanging a frame in each direction --
//! to demonstrate the real protocol working.

use relay::{RegisterRequest, RelayServer, RelayToken};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    stream.write_u32(data.len() as u32).await?;
    stream.write_all(data).await
}

async fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let len = stream.read_u32().await? as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn register(stream: &mut TcpStream, token: RelayToken) -> std::io::Result<()> {
    println!("mining proof-of-work...");
    let req = RegisterRequest::mine(token);
    let bytes = serde_json::to_vec(&req).expect("serialize register request");
    write_frame(stream, &bytes).await?;
    let ack = read_frame(stream).await?;
    assert_eq!(ack, b"OK");
    println!("registered (pow_nonce={})", req.pow_nonce);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = RelayServer::bind("127.0.0.1:0").await?;
    let addr = server.local_addr()?;
    println!("relay server listening on {addr}");
    tokio::spawn(server.run());

    let token = RelayToken::random();
    println!("session token: {}", token.to_hex());

    let mut peer_a = TcpStream::connect(addr).await?;
    register(&mut peer_a, token.clone()).await?;

    let mut peer_b = TcpStream::connect(addr).await?;
    register(&mut peer_b, token).await?;

    let ready = read_frame(&mut peer_a).await?;
    println!("peer A received: {:?}", String::from_utf8_lossy(&ready));

    write_frame(&mut peer_a, b"hello from peer A").await?;
    let msg = read_frame(&mut peer_b).await?;
    println!("peer B received: {}", String::from_utf8_lossy(&msg));

    write_frame(&mut peer_b, b"hello back from peer B").await?;
    let msg = read_frame(&mut peer_a).await?;
    println!("peer A received: {}", String::from_utf8_lossy(&msg));

    Ok(())
}

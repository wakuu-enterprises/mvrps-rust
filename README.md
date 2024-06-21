# Muvor Protocol Secure (MVRPS)

## Description

A secure custom protocol implementation for Muvor Protocol Secure (MVRPS) using TLS.

## Installation

```bash
cargo build
```

## Implementation

### Client

```bash
use mvrps::client::MVRPSClient;

#[tokio::main]
async fn main() {
    let mut client = MVRPSClient::new("127.0.0.1:8443", "client-key.pem", "client-cert.pem", "ca-cert.pem").await.unwrap();
    let response = client.send_request("CREATE", "/", "Hello, secure server!").await.unwrap();
    println!("Response: {}", response);
}
```

### Server

```bash
use mvrps::server::MVRPSServer;

#[tokio::main]
async fn main() {
    let server = MVRPSServer::new("127.0.0.1:8443", "server-key.pem", "server-cert.pem").await.unwrap();
    server.run().await.unwrap();
}
```
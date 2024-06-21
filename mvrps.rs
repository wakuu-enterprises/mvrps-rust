use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{rustls, TlsAcceptor, server::TlsStream};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use rustls::{ServerConfig, NoClientAuth, Certificate, PrivateKey};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::path::Path;
use std::collections::HashMap;
use mvrp_protocol::{MVRPRequest, MVRPResponse};

pub struct MVRPSServer {
    addr: String,
    tls_acceptor: TlsAcceptor,
}

impl MVRPSServer {
    pub async fn new(addr: &str, key_file: &str, cert_file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let certs = load_certs(cert_file)?;
        let key = load_private_key(key_file)?;

        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        let tls_acceptor = TlsAcceptor::from(Arc::new(config));

        Ok(MVRPSServer {
            addr: addr.to_string(),
            tls_acceptor,
        })
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("MVRPS server listening on {}", self.addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let acceptor = self.tls_acceptor.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(acceptor, stream).await {
                    eprintln!("Failed to process connection: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(acceptor: TlsAcceptor, stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = acceptor.accept(stream).await?;
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;

    let request = String::from_utf8_lossy(&buffer[..n]);
    let lines: Vec<&str> = request.split("\r\n").collect();
    if lines.len() < 1 {
        return Err("Malformed request".into());
    }

    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Malformed request line".into());
    }

    let method = parts[0];
    let url = parts[1];
    let headers = parse_headers(&lines[1..]);
    let body = lines.last().unwrap_or(&"").to_string();

    println!("Received {} request for {} with body: {}", method, url, body);
    handle_request(&mut stream, method, url, headers, body).await
}

async fn handle_request(stream: &mut TlsStream<TcpStream>, method: &str, url: &str, headers: HashMap<String, String>, body: String) -> Result<(), Box<dyn std::error::Error>> {
    let (status_line, response_body) = match method {
        "OPTIONS" => ("MVRP/1.0 204 No Content", ""),
        "CREATE" => ("MVRP/1.0 201 Created", "Resource created\n"),
        "READ" => ("MVRP/1.0 200 OK", "Resource read\n"),
        "EMIT" => ("MVRP/1.0 200 OK", "Event emitted\n"),
        "BURN" => ("MVRP/1.0 200 OK", "Resource burned\n"),
        _ => ("MVRP/1.0 405 Method Not Allowed", "Method not allowed\n"),
    };

    let response = format!(
        "{}\r\nContent-Type: text/plain\r\n\r\n{}",
        status_line, response_body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

fn parse_headers(lines: &[&str]) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let parts: Vec<&str> = line.splitn(2, ": ").collect();
        if parts.len() == 2 {
            headers.insert(parts[0].to_string(), parts[1].to_string());
        }
    }
    headers
}

fn load_certs(path: &str) -> Result<Vec<Certificate>, Box<dyn std::error::Error>> {
    let certfile = File::open(Path::new(path))?;
    let mut reader = BufReader::new(certfile);
    let certs = rustls_pemfile::certs(&mut reader)?
        .iter()
        .map(|v| Certificate(v.clone()))
        .collect();
    Ok(certs)
}

fn load_private_key(path: &str) -> Result<PrivateKey, Box<dyn std::error::Error>> {
    let keyfile = File::open(Path::new(path))?;
    let mut reader = BufReader::new(keyfile);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?
        .iter()
        .map(|v| PrivateKey(v.clone()))
        .collect::<Vec<_>>();
    if keys.len() != 1 {
        return Err("Expected a single private key".into());
    }
    Ok(keys[0].clone())
}

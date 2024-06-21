use tokio::net::TcpStream;
use tokio_rustls::{rustls, TlsConnector};
use tokio_rustls::webpki::DNSNameRef;
use std::sync::Arc;
use std::fs::File;
use std::io::BufReader;
use rustls::{ClientConfig, Certificate, PrivateKey, RootCertStore};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;

pub struct MVRPSClient {
    addr: String,
    tls_connector: TlsConnector,
}

impl MVRPSClient {
    pub async fn new(addr: &str, key_file: &str, cert_file: &str, ca_file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let key = load_private_key(key_file)?;
        let cert = load_cert(cert_file)?;
        let ca_cert = load_ca_cert(ca_file)?;

        let mut root_store = RootCertStore::empty();
        root_store.add(&ca_cert)?;

        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_single_cert(vec![cert], key)?;

        let tls_connector = TlsConnector::from(Arc::new(config));

        Ok(MVRPSClient {
            addr: addr.to_string(),
            tls_connector,
        })
    }

    pub async fn send_request(&mut self, method: &str, url: &str, body: &str) -> Result<String, Box<dyn std::error::Error>> {
        let domain = DNSNameRef::try_from_ascii_str("localhost")?;
        let stream = TcpStream::connect(&self.addr).await?;
        let mut stream = self.tls_connector.connect(domain, stream).await?;

        let request = format!(
            "{} {} MVRP/1.0\r\nContent-Length: {}\r\n\r\n{}",
            method, url, body.len(), body
        );
        stream.write_all(request.as_bytes()).await?;

        let mut buffer = vec![0; 1024];
        let n = stream.read(&mut buffer).await?;
        let response = String::from_utf8_lossy(&buffer[..n]).to_string();

        Ok(response)
    }
}

fn load_cert(path: &str) -> Result<Certificate, Box<dyn std::error::Error>> {
    let certfile = File::open(Path::new(path))?;
    let mut reader = BufReader::new(certfile);
    let certs = rustls_pemfile::certs(&mut reader)?
        .iter()
        .map(|v| Certificate(v.clone()))
        .collect::<Vec<_>>();
    if certs.len() != 1 {
        return Err("Expected a single certificate".into());
    }
    Ok(certs[0].clone())
}

fn load_ca_cert(path: &str) -> Result<Certificate, Box<dyn std::error::Error>> {
    let certfile = File::open(Path::new(path))?;
    let mut reader = BufReader::new(certfile);
    let certs = rustls_pemfile::certs(&mut reader)?
        .iter()
        .map(|v| Certificate(v.clone()))
        .collect::<Vec<_>>();
    if certs.len() != 1 {
        return Err("Expected a single CA certificate".into());
    }
    Ok(certs[0].clone())
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

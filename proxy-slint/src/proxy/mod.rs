use std::net::SocketAddr;
use slint::Weak;
use crate::MainWindow;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::error::Error;

mod system_proxy;
pub use system_proxy::{get_system_proxy, set_system_proxy, clear_system_proxy};

pub struct ProxyController {
    window: Option<Weak<MainWindow>>,
    listener: Option<TcpListener>,
}

impl ProxyController {
    pub fn new() -> Self {
        Self {
            window: None,
            listener: None,
        }
    }

    pub fn set_window(&mut self, window: Weak<MainWindow>) {
        self.window = Some(window);
    }

    pub async fn start(&mut self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
        if self.listener.is_some() {
            return Ok(());
        }

        let listener = TcpListener::bind(addr).await?;
        let listener_handle = listener.into_std()?;
        let listener_handle2 = listener_handle.try_clone()?;
        let listener = TcpListener::from_std(listener_handle)?;
        
        tokio::spawn(async move {
            while let Ok((inbound, _)) = listener.accept().await {
                tokio::spawn(handle_connection(inbound));
            }
        });

        self.listener = Some(TcpListener::from_std(listener_handle2)?);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        self.listener = None;
        Ok(())
    }
}

async fn handle_connection(mut inbound: tokio::net::TcpStream) {
    let mut buffer = [0; 4096];
    
    if let Ok(n) = inbound.read(&mut buffer).await {
        if n == 0 {
            return;
        }
        
        println!("Received request: {}", String::from_utf8_lossy(&buffer[..n]));
        
        if let Ok(request) = String::from_utf8(buffer[..n].to_vec()) {
            if request.starts_with("CONNECT") {
                handle_connect(inbound, &request).await;
            } else {
                handle_http(inbound, &request, &buffer[..n]).await;
            }
        }
    }
}

async fn handle_connect(mut client: tokio::net::TcpStream, request: &str) {
    if let Some(host_port) = extract_host(request) {
        println!("CONNECT to: {}", host_port);
        
        if let Ok(mut server) = tokio::net::TcpStream::connect(&host_port).await {
            let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
            if let Err(e) = client.write_all(response.as_bytes()).await {
                println!("Failed to write response: {}", e);
                return;
            }
            
            if let Err(e) = tokio::io::copy_bidirectional(&mut client, &mut server).await {
                println!("Failed to tunnel: {}", e);
            }
        } else {
            println!("Failed to connect to target server");
        }
    }
}

async fn handle_http(mut client: tokio::net::TcpStream, request: &str, original_data: &[u8]) {
    if let Some(host_port) = extract_host(request) {
        println!("HTTP request to: {}", host_port);
        if let Ok(mut server) = tokio::net::TcpStream::connect(&host_port).await {
            if let Err(e) = server.write_all(original_data).await {
                println!("Failed to write to server: {}", e);
                return;
            }
            
            let mut buffer = [0; 4096];
            while let Ok(n) = server.read(&mut buffer).await {
                if n == 0 { break; }
                if let Err(e) = client.write_all(&buffer[..n]).await {
                    println!("Failed to write to client: {}", e);
                    break;
                }
            }
        }
    }
}

fn extract_host(request: &str) -> Option<String> {
    for line in request.lines() {
        if line.to_lowercase().starts_with("host: ") {
            let host = line[6..].trim();
            if host.contains(":") {
                return Some(host.to_string());
            }
            if request.starts_with("CONNECT") {
                return Some(format!("{}:443", host));
            } else {
                return Some(format!("{}:80", host));
            }
        }
    }
    None
} 
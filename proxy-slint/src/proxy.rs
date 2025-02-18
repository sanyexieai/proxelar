use std::net::SocketAddr;
use proxyapi::proxy::Proxy;
use proxyapi::ca::Ssl;
use std::sync::mpsc::SyncSender;
use slint::{Weak, ModelRc, VecModel, Model};
use std::sync::Arc;
use std::error::Error;
use serde::{Serialize, Deserialize};

slint::slint! {
    import { MainWindow, RequestRecord } from "ui/main.slint";
}

// 使用 slint! 宏生成的类型
use crate::MainWindow;
use crate::RequestRecord;  // 从 crate 根导入

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub bypass: String,
}

pub struct ProxyServer {
    config: ProxyConfig,
}

impl ProxyServer {
    pub fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    pub fn start(&self) -> Result<(), Box<dyn Error>> {
        // 实现代理服务器启动逻辑
        Ok(())
    }

    pub fn stop(&self) -> Result<(), Box<dyn Error>> {
        // 实现代理服务器停止逻辑
        Ok(())
    }

    pub fn export_certificate(&self) -> Result<(), Box<dyn Error>> {
        // 实现证书导出逻辑
        Ok(())
    }
}

pub struct ProxyController {
    server: Option<(tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<()>)>,
    window: Option<Weak<MainWindow>>,
    ca: Option<Ssl>,
}

impl ProxyController {
    pub fn new() -> Self {
        Self { 
            server: None,
            window: None,
            ca: Some(Ssl::default()),
        }
    }

    pub fn set_window(&mut self, window: Weak<MainWindow>) {
        self.window = Some(window);
    }

    pub async fn start(&mut self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting proxy server on {}", addr);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let server = Proxy::new(addr, Some(tx.clone()));
        
        let (close_tx, close_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            println!("Proxy server task started");
            if let Err(e) = server.start(async {
                let _ = close_rx.await;
            }).await {
                eprintln!("Error in proxy server: {}", e);
            }
        });
        
        self.server = Some((close_tx, handle));
        
        let window = self.window.clone();
        tokio::spawn(async move {
            println!("Request handler task started");
            for exchange in rx.iter() {
                let (request, _response) = exchange.to_parts();
                if let Some(req) = request {
                    println!("Processing request: {} {}", req.method(), req.uri());
                    if let Some(window) = window.as_ref().and_then(|w| w.upgrade()) {
                        let current_requests = window.get_requests();
                        println!("Current request count: {}", current_requests.row_count());
                        let mut model = VecModel::default();
                        for i in 0..current_requests.row_count() {
                            model.push(current_requests.row_data(i).unwrap());
                        }
                        model.push(RequestRecord {
                            method: req.method().to_string().into(),
                            url: req.uri().to_string().into(),
                            status: 200,
                        });
                        println!("New request count: {}", model.row_count());
                        window.set_requests(ModelRc::new(model));
                    } else {
                        println!("Window not available for updating requests");
                    }
                }
            }
            println!("Request handler task ended");
        });
        
        println!("Proxy server started successfully");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some((tx, handle)) = self.server.take() {
            let _ = tx.send(());
            let _ = handle.await;
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.server.is_some()
    }
} 
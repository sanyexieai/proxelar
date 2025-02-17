use std::net::SocketAddr;
use proxyapi::proxy::Proxy;
use proxyapi::ca::Ssl;
use std::sync::mpsc::SyncSender;
use slint::{Weak, ModelRc, VecModel};
use std::sync::Arc;

slint::slint! {
    import { MainWindow, RequestRecord } from "ui/main.slint";
}

// 使用 slint! 宏生成的类型
use crate::MainWindow;
use crate::RequestRecord;  // 从 crate 根导入

pub struct ProxyController {
    server: Option<(tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<()>)>,
    window: Option<Weak<MainWindow>>,
}

impl ProxyController {
    pub fn new() -> Self {
        Self { 
            server: None,
            window: None,
        }
    }

    pub fn set_window(&mut self, window: Weak<MainWindow>) {
        self.window = Some(window);
    }

    pub async fn start(&mut self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let ca = Ssl::default();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let server = Proxy::new(addr, Some(tx.clone()));
        
        let (close_tx, close_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            if let Err(e) = server.start(async {
                let _ = close_rx.await;
            }).await {
                eprintln!("Error: {}", e);
            }
        });
        
        self.server = Some((close_tx, handle));
        
        let window = self.window.clone();
        tokio::spawn(async move {
            for exchange in rx.iter() {
                let (request, _response) = exchange.to_parts();
                if let Some(req) = request {
                    println!("Received request: {} {}", req.method(), req.uri());
                    if let Some(window) = window.as_ref().and_then(|w| w.upgrade()) {
                        let mut model = VecModel::default();
                        model.push(RequestRecord {
                            method: req.method().to_string().into(),
                            url: req.uri().to_string().into(),
                            status: 200,
                        });
                        window.set_requests(ModelRc::new(model));
                    } else {
                        println!("Window not available for updating requests");
                    }
                }
            }
        });
        
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(server) = self.server.take() {
            // 服务器会在 drop 时自动停止
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.server.is_some()
    }
} 
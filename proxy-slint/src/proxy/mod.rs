use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{mpsc::SyncSender, Arc},
};

use proxyapi::{Proxy, proxy_handler::ProxyHandler};
use slint::Weak;
use crate::MainWindow;
use std::sync::atomic::{AtomicBool, Ordering};

mod system_proxy;
pub use system_proxy::{get_system_proxy, set_system_proxy, clear_system_proxy};

pub struct ProxyController {
    window: Option<Weak<MainWindow>>,
    tx: Option<SyncSender<ProxyHandler>>,
    running: Arc<AtomicBool>,
    addr: Option<SocketAddr>,
}

impl ProxyController {
    pub fn new(proxy_handler: SyncSender<ProxyHandler>) -> Self {
        Self {
            window: None,
            tx: Some(proxy_handler),
            running: Arc::new(AtomicBool::new(false)),
            addr: None,
        }
    }

    pub fn set_window(&mut self, window: Weak<MainWindow>) {
        self.window = Some(window);
    }

    pub async fn start(&mut self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        self.addr = Some(addr);
        let proxy = proxyapi::proxy::Proxy::new(addr, self.tx.clone());
        proxy.start(std::future::pending()).await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(false, Ordering::SeqCst);
        
        if let Some(addr) = self.addr {
            let _ = tokio::net::TcpStream::connect(addr).await;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        
        Ok(())
    }
}
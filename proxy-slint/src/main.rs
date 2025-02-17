slint::slint! {
    import { MainWindow } from "ui/main.slint";
}

mod proxy;
use proxy::ProxyController;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use slint::ComponentHandle;
use once_cell::sync::Lazy;
use std::sync::Mutex;

static PROXY_HOST: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("127.0.0.1".to_string()));
static PROXY_PORT: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("8100".to_string()));

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    let proxy_controller = std::sync::Arc::new(tokio::sync::Mutex::new(ProxyController::new()));
    
    // 设置 window
    {
        let mut controller = proxy_controller.lock().await;
        controller.set_window(main_window.as_weak());
    }
    
    // 设置代理启动事件
    let proxy_weak = proxy_controller.clone();
    let window_weak = main_window.as_weak();
    main_window.on_start_proxy(move |host: slint::SharedString, port: i32| {
        println!("Start proxy triggered with host: {}, port: {}", host, port);
        
        let proxy = proxy_weak.clone();
        let window = window_weak.clone();
        
        tokio::spawn(async move {
            let addr = match IpAddr::from_str(&host.to_string()) {
                Ok(ip) => ip,
                Err(e) => {
                    println!("Invalid IP address: {}", e);
                    IpAddr::from_str("127.0.0.1").unwrap()
                }
            };
            
            let socket_addr = SocketAddr::new(addr, port as u16);
            println!("Attempting to start proxy on {}", socket_addr);
            
            let mut proxy = proxy.lock().await;
            match proxy.start(socket_addr).await {
                Ok(()) => {
                    println!("Proxy started successfully");
                    if let Some(window) = window.upgrade() {
                        window.set_proxy_running(true);
                    }
                },
                Err(e) => {
                    println!("Failed to start proxy: {}", e);
                    if let Some(window) = window.upgrade() {
                        window.set_proxy_running(false);
                    }
                }
            }
        });
    });

    // 添加停止代理处理
    let proxy_weak = proxy_controller.clone();
    let window_weak = main_window.as_weak();
    main_window.on_stop_proxy(move || {
        println!("Stop proxy triggered");
        let proxy = proxy_weak.clone();
        let window = window_weak.clone();
        
        tokio::spawn(async move {
            let mut proxy = proxy.lock().await;
            if let Err(e) = proxy.stop().await {
                println!("Failed to stop proxy: {}", e);
            }
            if let Some(window) = window.upgrade() {
                window.set_proxy_running(false);
            }
        });
    });

    // 修改证书安装处理
    let proxy_weak = proxy_controller.clone();
    let window_weak = main_window.as_weak();
    main_window.on_install_certificate(move || {
        println!("Installing certificate...");
        let proxy = proxy_weak.clone();
        let window = window_weak.clone();
        
        tokio::spawn(async move {
            // 1. 先停止代理
            let mut proxy = proxy.lock().await;
            let _ = proxy.stop().await;
            if let Some(window) = window.upgrade() {
                window.set_proxy_running(false);
            }

            // 2. 安装证书
            match proxyapi::ca::Ssl::install_certificate().await {
                Ok(()) => {
                    println!("Certificate installed successfully");
                    #[cfg(target_os = "windows")]
                    println!("If automatic installation failed, please manually install the certificate from: %APPDATA%\\proxelar\\ca.crt");
                    #[cfg(target_os = "macos")]
                    println!("If automatic installation failed, please manually install the certificate from: ~/Library/Application Support/proxelar/ca.crt");
                    #[cfg(target_os = "linux")]
                    println!("If automatic installation failed, please manually install the certificate from: ~/.local/share/proxelar/ca.crt");
                    println!("Please restart your browser after installing the certificate");
                }
                Err(e) => {
                    println!("Failed to install certificate: {}", e);
                }
            }
        });
    });

    println!("Application started");
    main_window.run()
} 
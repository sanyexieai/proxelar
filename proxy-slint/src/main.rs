slint::slint! {
    import { MainWindow } from "ui/main.slint";
}

mod proxy;
use proxy::{ProxyController, get_system_proxy, set_system_proxy, clear_system_proxy};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use slint::ComponentHandle;
use once_cell::sync::Lazy;
use std::sync::Mutex;

static PROXY_HOST: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("127.0.0.1".to_string()));
static PROXY_PORT: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("8100".to_string()));

// 添加新的静态变量来存储原始代理设置
static ORIGINAL_PROXY: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

fn is_port_available(port: u16) -> bool {
    // 同时检查 TCP 和 UDP
    let tcp_available = std::net::TcpListener::bind(("127.0.0.1", port)).is_ok();
    let udp_available = std::net::UdpSocket::bind(("127.0.0.1", port)).is_ok();
    
    // 如果创建成功，立即释放
    if tcp_available {
        if let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", port)) {
            drop(listener);
        }
    }
    if udp_available {
        if let Ok(socket) = std::net::UdpSocket::bind(("127.0.0.1", port)) {
            drop(socket);
        }
    }
    
    tcp_available && udp_available
}

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
        
        // 立即将按钮状态设置为启动中
        if let Some(window) = window.upgrade() {
            window.set_proxy_running(true);
        }
        
        tokio::spawn(async move {
            // 确保之前的代理已经完全停止
            {
                let mut proxy = proxy.lock().await;
                if let Err(e) = proxy.stop().await {
                    println!("Failed to stop previous proxy instance: {}", e);
                }
                
                // 添加小延迟确保端口完全释放
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            
            // 检查端口是否可用，最多尝试3次
            let mut port_available = false;
            for attempt in 1..=3 {
                if is_port_available(port as u16) {
                    port_available = true;
                    break;
                }
                println!("Port {} is still in use, attempt {}/3", port, attempt);
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            
            if !port_available {
                println!("Port {} is still in use after multiple attempts", port);
                if let Some(window) = window.upgrade() {
                    window.set_proxy_running(false);
                }
                return;
            }
            
            // 保存当前系统代理设置
            if let Ok(current_proxy) = get_system_proxy() {
                let mut original_proxy = ORIGINAL_PROXY.lock().unwrap();
                *original_proxy = Some(current_proxy);
            }

            let addr = match IpAddr::from_str(&host.to_string()) {
                Ok(ip) => ip,
                Err(e) => {
                    println!("Invalid IP address: {}", e);
                    IpAddr::from_str("127.0.0.1").unwrap()
                }
            };
            
            let socket_addr = SocketAddr::new(addr, port as u16);
            println!("Attempting to start proxy on {}", socket_addr);
            
            // 设置系统代理
            if let Err(e) = set_system_proxy(&format!("{}:{}", host, port)) {
                println!("Failed to set system proxy: {}", e);
                if let Some(window) = window.upgrade() {
                    window.set_proxy_running(false);
                }
                return;
            }
            
            let mut proxy = proxy.lock().await;
            match proxy.start(socket_addr).await {
                Ok(_) => {
                    println!("Proxy started successfully");
                    // 代理启动成功，保持按钮状态为 true
                },
                Err(e) => {
                    println!("Failed to start proxy: {}", e);
                    if let Some(window) = window.upgrade() {
                        window.set_proxy_running(false);
                    }
                    // 如果启动失败，清除代理设置
                    if let Err(e) = clear_system_proxy() {
                        println!("Failed to clear system proxy: {}", e);
                    }
                }
            }
        });
    });

    // 修改停止代理处理
    let proxy_weak = proxy_controller.clone();
    let window_weak = main_window.as_weak();
    main_window.on_stop_proxy(move || {
        println!("Stop proxy triggered");
        let proxy = proxy_weak.clone();
        let window = window_weak.clone();
        
        // 立即更新UI状态
        if let Some(window) = window.upgrade() {
            window.set_proxy_running(false);
        }
        
        // 在 spawn 之前获取代理设置
        let proxy_setting = ORIGINAL_PROXY.lock().unwrap().clone();
        
        tokio::spawn(async move {
            // 先停止代理服务
            let mut proxy = proxy.lock().await;
            if let Err(e) = proxy.stop().await {
                println!("Failed to stop proxy server: {}", e);
                // 即使代理服务停止失败，也继续尝试清理系统代理设置
            }

            // 然后恢复系统代理设置
            if let Some(proxy_setting) = proxy_setting {
                if let Err(e) = set_system_proxy(&proxy_setting) {
                    println!("Failed to restore original proxy settings: {}", e);
                    // 如果恢复失败，尝试完全清除代理设置
                    if let Err(e) = clear_system_proxy() {
                        println!("Failed to clear system proxy as fallback: {}", e);
                    }
                }
            } else {
                if let Err(e) = clear_system_proxy() {
                    println!("Failed to clear system proxy: {}", e);
                }
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
            // // 1. 先停止代理
            // let mut proxy = proxy.lock().await;
            // let _ = proxy.stop().await;
            // if let Some(window) = window.upgrade() {
            //     window.set_proxy_running(false);
            // }

            // // 2. 安装证书
            // match proxyapi::ca::Ssl::install_certificate().await {
            //     Ok(()) => {
            //         println!("Certificate installed successfully");
            //         #[cfg(target_os = "windows")]
            //         println!("If automatic installation failed, please manually install the certificate from: %APPDATA%\\proxelar\\ca.crt");
            //         #[cfg(target_os = "macos")]
            //         println!("If automatic installation failed, please manually install the certificate from: ~/Library/Application Support/proxelar/ca.crt");
            //         #[cfg(target_os = "linux")]
            //         println!("If automatic installation failed, please manually install the certificate from: ~/.local/share/proxelar/ca.crt");
            //         println!("Please restart your browser after installing the certificate");
            //     }
            //     Err(e) => {
            //         println!("Failed to install certificate: {}", e);
            //     }
            // }
        });
    });

    println!("Application started");
    main_window.run()
} 
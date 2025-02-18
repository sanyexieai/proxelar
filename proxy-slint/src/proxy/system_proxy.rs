use std::error::Error;

#[cfg(target_os = "windows")]
pub fn get_system_proxy() -> Result<String, Box<dyn Error>> {
    // Windows 实现
    let output = std::process::Command::new("reg")
        .args(&["query", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings", "/v", "ProxyServer"])
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    if output_str.contains("ProxyServer") {
        let proxy = output_str.lines()
            .find(|line| line.contains("ProxyServer"))
            .and_then(|line| line.split_whitespace().last())
            .unwrap_or("");
        Ok(proxy.to_string())
    } else {
        Ok("".to_string())
    }
}

#[cfg(target_os = "windows")]
pub fn set_system_proxy(proxy: &str) -> Result<(), Box<dyn Error>> {
    // 启用代理
    std::process::Command::new("reg")
        .args(&["add", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings", "/v", "ProxyEnable", "/t", "REG_DWORD", "/d", "1", "/f"])
        .output()?;
    
    // 设置代理服务器
    std::process::Command::new("reg")
        .args(&["add", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings", "/v", "ProxyServer", "/t", "REG_SZ", "/d", proxy, "/f"])
        .output()?;
    
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn clear_system_proxy() -> Result<(), Box<dyn Error>> {
    // 禁用代理
    std::process::Command::new("reg")
        .args(&["add", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings", "/v", "ProxyEnable", "/t", "REG_DWORD", "/d", "0", "/f"])
        .output()?;
    
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn get_system_proxy() -> Result<String, Box<dyn Error>> {
    let output = std::process::Command::new("networksetup")
        .args(&["-getwebproxy", "Wi-Fi"])
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    if output_str.contains("Enabled: Yes") {
        let server = output_str.lines()
            .find(|line| line.contains("Server:"))
            .and_then(|line| line.split_whitespace().last())
            .unwrap_or("");
        let port = output_str.lines()
            .find(|line| line.contains("Port:"))
            .and_then(|line| line.split_whitespace().last())
            .unwrap_or("");
        Ok(format!("{}:{}", server, port))
    } else {
        Ok("".to_string())
    }
}

#[cfg(target_os = "macos")]
pub fn set_system_proxy(proxy: &str) -> Result<(), Box<dyn Error>> {
    let parts: Vec<&str> = proxy.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid proxy format".into());
    }
    
    std::process::Command::new("networksetup")
        .args(&["-setwebproxy", "Wi-Fi", parts[0], parts[1]])
        .output()?;
    
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn clear_system_proxy() -> Result<(), Box<dyn Error>> {
    std::process::Command::new("networksetup")
        .args(&["-setwebproxystate", "Wi-Fi", "off"])
        .output()?;
    
    Ok(())
}

// Linux 实现可以根据具体发行版添加
#[cfg(target_os = "linux")]
pub fn get_system_proxy() -> Result<String, Box<dyn Error>> {
    // 实现 Linux 的代理获取逻辑
    Ok("".to_string())
}

#[cfg(target_os = "linux")]
pub fn set_system_proxy(proxy: &str) -> Result<(), Box<dyn Error>> {
    // 实现 Linux 的代理设置逻辑
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn clear_system_proxy() -> Result<(), Box<dyn Error>> {
    // 实现 Linux 的代理清除逻辑
    Ok(())
} 
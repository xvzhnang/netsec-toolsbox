use tauri::State;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::process::{Child, Command, Stdio};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub struct MkDocsServerState {
    pub child: Mutex<Option<Child>>,
    pub port: Mutex<u16>,
    pub is_running: Mutex<bool>,
}

impl Default for MkDocsServerState {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            port: Mutex::new(8008),
            is_running: Mutex::new(false),
        }
    }
}

fn find_mkdocs_config(base_dir: &Path) -> Option<PathBuf> {
    // Prefer lowercase `wiki/mkdocs.yml`
    let candidate1 = base_dir.join("wiki").join("mkdocs.yml");
    if candidate1.exists() {
        return Some(candidate1);
    }
    // Fallback to uppercase `Wiki/mkdocs.yml`
    let candidate2 = base_dir.join("Wiki").join("mkdocs.yml");
    if candidate2.exists() {
        return Some(candidate2);
    }
    None
}

fn is_port_open(addr: &str) -> bool {
    // Try connect with short timeout
    if let Ok(mut addrs_iter) = addr.to_socket_addrs() {
        if let Some(sock_addr) = addrs_iter.next() {
            return TcpStream::connect_timeout(&sock_addr, Duration::from_millis(200)).is_ok();
        }
    }
    false
}

#[tauri::command]
pub async fn start_mkdocs_server(state: State<'_, MkDocsServerState>) -> Result<u16, String> {
    let base_dir = crate::utils::get_app_base_dir();
    let config_path = find_mkdocs_config(&base_dir)
        .ok_or_else(|| {
            format!(
                "未找到 mkdocs.yml 配置文件。\n请创建如下结构：\n\
toolbox/\n  ├─ wiki/ 或 Wiki/\n  │   ├─ mkdocs.yml\n  │   └─ docs/\n\n\
并在 mkdocs.yml 中配置 Material 主题与导航。"
            )
        })?;

    let mut is_running_guard = state.is_running.lock().map_err(|e| e.to_string())?;
    let mut port_guard = state.port.lock().map_err(|e| e.to_string())?;
    let mut child_guard = state.child.lock().map_err(|e| e.to_string())?;

    let port = 8008;
    let addr = format!("127.0.0.1:{}", port);

    // If already running, just return
    if *is_running_guard {
        return Ok(*port_guard);
    }

    // If port already open (existing mkdocs or other), treat as running
    if is_port_open(&addr) {
        *is_running_guard = true;
        *port_guard = port;
        return Ok(port);
    }

    // Spawn mkdocs serve
    let config_str = config_path.to_string_lossy().to_string();
    let mut cmd = Command::new("mkdocs");
    cmd.arg("serve")
        .arg("-f")
        .arg(&config_str)
        .arg("-a")
        .arg(&addr)
        .current_dir(config_path.parent().unwrap_or(&base_dir))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    match cmd.spawn() {
        Ok(child) => {
            *child_guard = Some(child);
            *is_running_guard = true;
            *port_guard = port;
            log::info!("MkDocs 本地服务已启动: http://{}", addr);
            Ok(port)
        }
        Err(e) => {
            Err(format!(
                "启动 MkDocs 失败：{}\n请确保已安装 MkDocs 与 Material 主题，并将 mkdocs.exe 加入 PATH。",
                e
            ))
        }
    }
}

#[tauri::command]
pub async fn stop_mkdocs_server(state: State<'_, MkDocsServerState>) -> Result<(), String> {
    let mut is_running_guard = state.is_running.lock().map_err(|e| e.to_string())?;
    let mut child_guard = state.child.lock().map_err(|e| e.to_string())?;

    if let Some(child) = child_guard.as_mut() {
        match child.kill() {
            Ok(_) => {
                log::info!("MkDocs 本地服务已停止");
            }
            Err(e) => {
                log::warn!("停止 MkDocs 进程失败: {}", e);
            }
        }
        *child_guard = None;
    }
    *is_running_guard = false;
    Ok(())
}

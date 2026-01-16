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

fn ensure_default_scaffold(base_dir: &Path) -> std::io::Result<PathBuf> {
    let wiki_dir = base_dir.join("wiki");
    let docs_dir = wiki_dir.join("docs");
    std::fs::create_dir_all(&docs_dir)?;

    let mkdocs_yml = wiki_dir.join("mkdocs.yml");
    if !mkdocs_yml.exists() {
        let yml = r#"site_name: Toolbox Wiki
theme:
  name: material
  language: zh
  features:
    - navigation.instant
    - navigation.sections
    - navigation.expand
    - navigation.top
    - content.code.copy
    - search.highlight
markdown_extensions:
  - admonition
  - codehilite
  - toc:
      permalink: true
  - pymdownx.superfences
  - pymdownx.details
nav:
  - 首页: index.md
"#;
        std::fs::write(&mkdocs_yml, yml)?;
    }

    let index_md = docs_dir.join("index.md");
    if !index_md.exists() {
        let md = r#"# 欢迎来到 Toolbox Wiki

这是自动初始化的首页。将你的 Markdown 文档放入 wiki/docs/，保存后通过右上角 Wiki 打开即可查看。
"#;
        std::fs::write(index_md, md)?;
    }

    Ok(mkdocs_yml)
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

pub fn init_mkdocs(state: &MkDocsServerState) -> Result<u16, String> {
    let base_dir = crate::utils::get_app_base_dir();
    let config_path = match find_mkdocs_config(&base_dir) {
        Some(p) => p,
        None => {
            match ensure_default_scaffold(&base_dir) {
                Ok(p) => {
                    log::info!("已自动创建 MkDocs 基础结构: {}", p.display());
                    p
                },
                Err(e) => {
                    return Err(format!("自动创建 MkDocs 结构失败: {}", e));
                }
            }
        }
    };

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

    // Spawn mkdocs serve using project-embedded Python (python313)
    let config_str = config_path.to_string_lossy().to_string();
    let python_exe = base_dir.join("python313").join("python.exe");
    if !python_exe.exists() {
        return Err(format!(
            "未找到项目内置 Python 解释器: {}\n请确保存在 python313/python.exe。",
            python_exe.display()
        ));
    }
    let mut cmd = Command::new(python_exe);
    cmd.arg("-m")
        .arg("mkdocs")
        .arg("serve")
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
                "启动 MkDocs 失败：{}\n请在项目根的 python313 环境安装依赖：\n  python313/python.exe -m pip install mkdocs mkdocs-material",
                e
            ))
        }
    }
}

#[tauri::command]
pub async fn start_mkdocs_server(state: State<'_, MkDocsServerState>) -> Result<u16, String> {
    // 异步命令中调用同步初始化逻辑
    // 由于这可能涉及文件IO和进程启动，如果是长时间操作应该放在 spawn_blocking 中
    // 但 mkdocs serve 启动通常很快，且 spawn 本身是非阻塞的
    init_mkdocs(&state)
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

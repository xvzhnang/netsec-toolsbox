use tauri::{Emitter, State, Manager};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::process::{Child, Command, Stdio};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{CloseHandle, HANDLE};

#[cfg(target_os = "windows")]
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

pub struct MkDocsServerState {
    pub child: Mutex<Option<Child>>,
    pub port: Mutex<u16>,
    pub is_running: Mutex<bool>,
    #[cfg(target_os = "windows")]
    pub job: Mutex<Option<JobObject>>,
}

impl Default for MkDocsServerState {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            port: Mutex::new(8000),
            is_running: Mutex::new(false),
            #[cfg(target_os = "windows")]
            job: Mutex::new(None),
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct JobObject {
    handle: HANDLE,
}

#[cfg(target_os = "windows")]
unsafe impl Send for JobObject {}

#[cfg(target_os = "windows")]
unsafe impl Sync for JobObject {}

#[cfg(target_os = "windows")]
impl Drop for JobObject {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
fn create_job_object_kill_on_close() -> Result<JobObject, String> {
    unsafe {
        let handle = CreateJobObjectW(None, None).map_err(|e| format!("CreateJobObjectW 失败: {}", e))?;

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(|e| format!("SetInformationJobObject 失败: {}", e))?;

        Ok(JobObject { handle })
    }
}

#[cfg(target_os = "windows")]
fn assign_child_to_job(job: &JobObject, child: &Child) -> Result<(), String> {
    unsafe {
        let process_handle = HANDLE(child.as_raw_handle() as *mut c_void);
        AssignProcessToJobObject(job.handle, process_handle)
            .map_err(|e| format!("AssignProcessToJobObject 失败: {}", e))?;
    }
    Ok(())
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

fn pick_free_port(start_port: u16) -> u16 {
    let mut port = start_port;
    for _ in 0..200 {
        let addr = format!("127.0.0.1:{}", port);
        if !is_port_open(&addr) {
            return port;
        }
        port = port.saturating_add(1);
    }
    start_port
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
    #[cfg(target_os = "windows")]
    let mut job_guard = state.job.lock().map_err(|e| e.to_string())?;

    // If already running, just return
    if *is_running_guard {
        if child_guard.is_some() {
            return Ok(*port_guard);
        }
        *is_running_guard = false;
    }
    *child_guard = None;
    #[cfg(target_os = "windows")]
    {
        *job_guard = None;
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

    let port = pick_free_port(*port_guard);
    let addr = format!("127.0.0.1:{}", port);

    let mut cmd = Command::new(python_exe);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
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
            #[cfg(target_os = "windows")]
            {
                let job = create_job_object_kill_on_close()?;
                assign_child_to_job(&job, &child)?;
                *job_guard = Some(job);
            }
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
pub async fn start_mkdocs_server(
    app: tauri::AppHandle,
    _state: State<'_, MkDocsServerState>,
) -> Result<u16, String> {
    // 异步命令中调用同步初始化逻辑
    // 由于这可能涉及文件IO和进程启动，如果是长时间操作应该放在 spawn_blocking 中
    let app_clone = app.clone();
    let port = tauri::async_runtime::spawn_blocking(move || {
        let state = app_clone.state::<MkDocsServerState>();
        init_mkdocs(&state)
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {}", e))??;

    let _ = app.emit("module_status_changed", ());
    Ok(port)
}

#[tauri::command]
pub async fn stop_mkdocs_server(
    app: tauri::AppHandle,
    state: State<'_, MkDocsServerState>,
) -> Result<(), String> {
    stop_mkdocs(&state)?;
    let _ = app.emit("module_status_changed", ());
    Ok(())
}

pub fn stop_mkdocs(state: &MkDocsServerState) -> Result<(), String> {
    let mut is_running_guard = state.is_running.lock().map_err(|e| e.to_string())?;
    let mut child_guard = state.child.lock().map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    let mut job_guard = state.job.lock().map_err(|e| e.to_string())?;

    if let Some(child) = child_guard.as_mut() {
        let _ = child.kill();
        let mut attempts = 0;
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => break,
                Ok(None) => {
                    attempts += 1;
                    if attempts >= 10 {
                        let _ = child.kill();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => break,
            }
        }
        let _ = child.wait();
        *child_guard = None;
        #[cfg(target_os = "windows")]
        {
            *job_guard = None;
        }
        log::info!("MkDocs 本地服务已停止");
    }

    *is_running_guard = false;
    Ok(())
}

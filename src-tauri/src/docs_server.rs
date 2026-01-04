use actix_cors::Cors;
use actix_files as fs;
use actix_web::{App, HttpServer};
use std::sync::Mutex;
use tauri::State;
use std::path::Path;

// 全局状态，用于控制服务器的启动和停止（虽然这里是后台运行，但可以用来记录端口等信息）
pub struct DocsServerState {
    pub port: Mutex<u16>,
    pub is_running: Mutex<bool>,
}

impl Default for DocsServerState {
    fn default() -> Self {
        Self {
            port: Mutex::new(3000), // 默认端口，实际启动时可能会变
            is_running: Mutex::new(false),
        }
    }
}

// 递归复制目录
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

// 启动本地 Docsify 服务器
#[tauri::command]
pub async fn start_docs_server(
    state: State<'_, DocsServerState>,
) -> Result<u16, String> {
    let mut is_running = state.is_running.lock().map_err(|e| e.to_string())?;
    let mut port_guard = state.port.lock().map_err(|e| e.to_string())?;

    // 如果已经在运行，直接返回当前端口
    if *is_running {
        return Ok(*port_guard);
    }

    // 获取文档根目录
    // 优先使用根目录下的 Wiki 目录
    let mut docs_dir = crate::utils::get_app_base_dir().join("Wiki");
    
    // 如果 Wiki 目录不存在，尝试使用 public/docs (兼容旧结构)
    if !docs_dir.exists() {
        docs_dir = crate::utils::get_app_base_dir().join("public").join("docs");
    }
    
    // 如果 public/docs 不存在，尝试直接找 docs（可能是打包后的结构）
    if !docs_dir.exists() {
        docs_dir = crate::utils::get_app_base_dir().join("docs");
    }

    // 如果还是不存在，尝试创建 Wiki 目录并初始化（复制 public/docs 内容）
    if !docs_dir.exists() {
        let wiki_path = crate::utils::get_app_base_dir().join("Wiki");
        if let Err(e) = std::fs::create_dir_all(&wiki_path) {
             log::warn!("无法创建 Wiki 目录: {}", e);
        } else {
            // 尝试复制 public/docs 下的资源文件到 Wiki
            let public_docs = crate::utils::get_app_base_dir().join("public").join("docs");
            if public_docs.exists() {
                 if let Err(e) = copy_dir_all(&public_docs, &wiki_path) {
                      log::warn!("复制初始化文件到 Wiki 失败: {}", e);
                 } else {
                      log::info!("已初始化 Wiki 目录: {}", wiki_path.display());
                      docs_dir = wiki_path;
                 }
            }
        }
    }

    // 如果最终还是不存在，报错
    if !docs_dir.exists() {
        return Err(format!("文档目录不存在，请确保根目录下存在 Wiki 目录: {}", docs_dir.display()));
    }

    let docs_dir_path = docs_dir.to_string_lossy().to_string();
    log::info!("启动文档服务器，根目录: {}", docs_dir_path);

    // 动态寻找可用端口 (从 3000 开始)
    let mut port = 3000;
    loop {
        if is_port_available(port) {
            break;
        }
        port += 1;
        if port > 4000 {
            return Err("无法找到可用端口".to_string());
        }
    }

    *port_guard = port;
    let server_port = port;
    
    // 在新线程中启动 Actix Web 服务器
    std::thread::spawn(move || {
        let sys = actix_web::rt::System::new();
        sys.block_on(async move {
            HttpServer::new(move || {
                App::new()
                    .wrap(Cors::permissive()) // 允许跨域
                    .service(fs::Files::new("/", &docs_dir_path).index_file("index.html"))
            })
            .bind(("127.0.0.1", server_port))
            .expect("无法绑定端口")
            .run()
            .await
            .expect("服务器运行出错");
        });
    });

    *is_running = true;
    log::info!("文档服务器已启动: http://localhost:{}", port);

    Ok(port)
}

// 检查端口是否可用
fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

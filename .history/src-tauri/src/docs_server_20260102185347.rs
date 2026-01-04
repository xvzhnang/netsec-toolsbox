use actix_cors::Cors;
use actix_files as fs;
use actix_web::{App, HttpServer};
use std::sync::Mutex;
use tauri::State;

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
    // 这里假设文档在项目根目录的 public/docs 下（开发环境）
    // 或者资源目录下的 docs（生产环境）
    // 简单起见，我们先定位到 public/docs，后续可以优化路径获取逻辑
    let mut docs_dir = crate::utils::get_app_base_dir().join("public").join("docs");
    
    // 如果 public/docs 不存在，尝试直接找 docs（可能是打包后的结构）
    if !docs_dir.exists() {
        docs_dir = crate::utils::get_app_base_dir().join("docs");
    }

    // 如果还是不存在，报错
    if !docs_dir.exists() {
        return Err(format!("文档目录不存在: {}", docs_dir.display()));
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

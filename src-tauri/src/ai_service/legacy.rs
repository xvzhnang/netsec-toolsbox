use crate::utils::get_app_base_dir;
use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::State;

/// AI 服务状态管理
#[derive(Default)]
pub struct AIServiceState {
    process: Arc<Mutex<Option<Child>>>,
}

/// 获取 Python 可执行文件路径
fn get_python_path() -> std::path::PathBuf {
    let base_dir = get_app_base_dir();
    let python_path = base_dir.join("python313").join("python.exe");

    log::debug!(
        "get_python_path: 项目根目录: {}, Python 路径: {}",
        base_dir.display(),
        python_path.display()
    );

    python_path
}

/// 获取 AI Gateway 服务脚本路径
fn get_ai_service_path() -> std::path::PathBuf {
    let base_dir = get_app_base_dir();
    let service_path = base_dir.join("ai_service").join("main_gateway.py");

    log::debug!(
        "get_ai_service_path: 项目根目录: {}, 服务脚本路径: {}",
        base_dir.display(),
        service_path.display()
    );

    service_path
}

/// 启动 AI Gateway 服务
#[tauri::command]
pub fn start_ai_service(state: State<AIServiceState>) -> Result<String, String> {
    let mut process_guard =
        crate::utils::lock_or_recover(state.process.as_ref(), "AIServiceState.process");

    // 检查服务是否已经在运行
    if let Some(ref mut child) = process_guard.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                // 进程已结束，可以重新启动
                log::info!("AI 服务进程已结束，准备重新启动");
            }
            Ok(None) => {
                // 进程仍在运行
                return Ok("AI 服务已在运行".to_string());
            }
            Err(e) => {
                log::warn!("检查 AI 服务状态失败: {}, 尝试重新启动", e);
            }
        }
    }

    let python_path = get_python_path();
    let service_path = get_ai_service_path();
    let base_dir = get_app_base_dir();

    // 检查 Python 可执行文件是否存在
    if !python_path.exists() {
        return Err(format!(
            "Python 可执行文件不存在: {}",
            python_path.display()
        ));
    }

    // 检查服务脚本是否存在
    if !service_path.exists() {
        return Err(format!(
            "AI Gateway 服务脚本不存在: {}",
            service_path.display()
        ));
    }

    log::info!(
        "启动 AI Gateway 服务: {} {}",
        python_path.display(),
        service_path.display()
    );

    // 构建配置文件路径（ai_service/config/models.json）
    let config_path = base_dir
        .join("ai_service")
        .join("config")
        .join("models.json");

    // 启动 Python 服务
    let mut cmd = Command::new(&python_path);
    cmd.arg(&service_path).arg("--port").arg("8765");

    // 如果配置文件存在，传递配置文件路径
    if config_path.exists() {
        cmd.arg("--config").arg(config_path.to_str().unwrap_or(""));
    }

    // 使用 stderr 捕获错误，stdout 设为 null 避免冲突
    let mut child = cmd
        .stdout(Stdio::null()) // 丢弃 stdout，避免冲突
        .stderr(Stdio::piped()) // 保留 stderr 用于错误捕获
        .spawn()
        .map_err(|e| format!("启动 AI Gateway 服务失败: {}", e))?;

    // 启动后台线程持续读取 stderr 并记录日志
    if let Some(stderr) = child.stderr.take() {
        let stderr_reader = BufReader::new(stderr);
        thread::spawn(move || {
            log::info!("[AI Gateway] 开始读取 stderr 输出...");
            for line in stderr_reader.lines() {
                match line {
                    Ok(line) => {
                        // 所有日志都输出，确保能看到调试信息
                        if line.contains("[FATAL]")
                            || line.contains("[EXIT]")
                            || line.contains("[UNHANDLED]")
                        {
                            log::error!("[AI Gateway] {}", line);
                        } else if line.contains("[ERROR]") {
                            log::error!("[AI Gateway] {}", line);
                        } else if line.contains("[WARN]") {
                            log::warn!("[AI Gateway] {}", line);
                        } else if line.contains("[INIT]")
                            || line.contains("[READY]")
                            || line.contains("[SERVER]")
                            || line.contains("[MAIN]")
                        {
                            log::info!("[AI Gateway] {}", line);
                        } else if line.contains("[REQUEST-")
                            || line.contains("[HANDLER]")
                            || line.contains("[STEP-")
                        {
                            // DEBUG 级别的日志也输出为 INFO，确保能看到
                            log::info!("[AI Gateway] {}", line);
                        } else if line.trim().is_empty() {
                            // 空行跳过
                            continue;
                        } else {
                            // 默认作为 info 输出
                            log::info!("[AI Gateway] {}", line);
                        }
                    }
                    Err(e) => {
                        log::warn!("[AI Gateway] 读取 stderr 失败: {}", e);
                        break;
                    }
                }
            }
            log::warn!("[AI Gateway] stderr 读取线程结束（可能是进程退出）");
        });
    } else {
        log::warn!("[AI Gateway] 无法获取 stderr 句柄");
    }

    // 等待服务启动（给 Python 服务一些时间初始化）
    // 先等待 2 秒，然后检查进程是否还在运行
    std::thread::sleep(std::time::Duration::from_millis(2000));

    // 检查进程是否已经退出
    match child.try_wait() {
        Ok(Some(status)) => {
            // 进程已退出
            // 注意：stderr 已经在后台线程中被读取，这里不需要再次读取
            log::error!("AI Gateway 服务进程立即退出，退出状态: {:?}", status);
            log::error!("请查看上方的 [AI Gateway] 日志以获取详细错误信息");
            return Err(format!(
                "AI Gateway 服务进程立即退出，退出状态: {:?}",
                status
            ));
        }
        Ok(None) => {
            // 进程仍在运行，再等待一段时间让 HTTP 服务器完全启动
            log::info!("AI Gateway 服务进程已启动，等待 HTTP 服务器就绪...");
            std::thread::sleep(std::time::Duration::from_millis(3000));

            // 再次检查进程是否还在运行
            match child.try_wait() {
                Ok(Some(status)) => {
                    log::error!("AI Gateway 服务进程在启动后退出，退出状态: {:?}", status);
                    return Err(format!(
                        "AI Gateway 服务进程在启动后退出，退出状态: {:?}",
                        status
                    ));
                }
                Ok(None) => {
                    log::info!("AI Gateway 服务已启动并运行中");
                }
                Err(e) => {
                    log::warn!("检查 AI Gateway 服务状态失败: {}", e);
                }
            }
        }
        Err(e) => {
            log::warn!("检查 AI Gateway 服务状态失败: {}", e);
        }
    }

    *process_guard = Some(child);

    Ok("AI Gateway 服务已启动".to_string())
}

/// 停止 AI Gateway 服务
#[tauri::command]
pub fn stop_ai_service(state: State<AIServiceState>) -> Result<String, String> {
    let mut process_guard =
        crate::utils::lock_or_recover(state.process.as_ref(), "AIServiceState.process");

    if let Some(mut child) = process_guard.take() {
        log::info!("停止 AI Gateway 服务");

        #[cfg(target_os = "windows")]
        {
            // Windows 上使用 taskkill 强制终止进程树
            if let Err(e) = child.kill() {
                log::warn!("终止 AI Gateway 服务进程失败: {}", e);
            }

            // 尝试使用 taskkill 确保进程被终止
            if let Ok(output) = Command::new("taskkill")
                .args(&["/F", "/T", "/PID", &child.id().to_string()])
                .output()
            {
                if output.status.success() {
                    log::info!("AI Gateway 服务进程已终止");
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Err(e) = child.kill() {
                return Err(format!("终止 AI Gateway 服务失败: {}", e));
            }
        }

        Ok("AI Gateway 服务已停止".to_string())
    } else {
        Ok("AI Gateway 服务未运行".to_string())
    }
}

/// 检查 AI Gateway 服务状态
#[tauri::command]
pub fn check_ai_service_status(state: State<AIServiceState>) -> Result<bool, String> {
    let mut process_guard =
        crate::utils::lock_or_recover(state.process.as_ref(), "AIServiceState.process");

    if let Some(child) = process_guard.as_mut() {
        match child.try_wait() {
            Ok(Some(status)) => {
                log::info!("AI Gateway 服务进程已结束，退出状态: {:?}", status);
                Ok(false)
            }
            Ok(None) => {
                Ok(true) // 进程仍在运行
            }
            Err(e) => {
                log::warn!("检查 AI Gateway 服务状态失败: {}", e);
                Ok(false)
            }
        }
    } else {
        Ok(false) // 进程不存在
    }
}

/// 获取 models.json 配置文件路径
fn get_models_config_path() -> std::path::PathBuf {
    let base_dir = get_app_base_dir();
    let config_path = base_dir
        .join("ai_service")
        .join("config")
        .join("models.json");

    log::debug!(
        "get_models_config_path: 项目根目录: {}, 配置文件路径: {}",
        base_dir.display(),
        config_path.display()
    );

    config_path
}

/// 读取 models.json 配置文件
#[tauri::command]
pub fn read_models_config() -> Result<String, String> {
    let config_path = get_models_config_path();

    if config_path.exists() {
        fs::read_to_string(&config_path).map_err(|e| format!("读取 models.json 失败: {}", e))
    } else {
        // 如果文件不存在，返回空 JSON
        Ok(r#"{"models": []}"#.to_string())
    }
}

/// 写入 models.json 配置文件
#[tauri::command]
pub fn write_models_config(content: String) -> Result<(), String> {
    let config_path = get_models_config_path();

    // 确保目录存在
    if let Some(parent) = config_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(format!("创建配置目录失败: {}", e));
        }
    }

    // 写入文件
    fs::write(&config_path, content).map_err(|e| format!("写入 models.json 失败: {}", e))?;

    log::info!("models.json 配置文件已更新: {}", config_path.display());
    Ok(())
}

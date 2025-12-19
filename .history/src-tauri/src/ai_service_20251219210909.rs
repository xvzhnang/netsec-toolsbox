use std::path::PathBuf;
use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader};
use tauri::State;
use crate::utils::get_config_dir;

/// AI 服务进程状态
#[derive(Default)]
pub struct AIServiceState {
    process: Arc<Mutex<Option<Child>>>,
}

/// 获取项目根目录（使用与 wiki 相同的逻辑）
fn get_app_base_dir() -> PathBuf {
    let exe_path = std::env::current_exe()
        .expect("failed to get executable path");
    
    let mut current = exe_path.parent()
        .expect("failed to get executable directory");
    
    // 向上查找，直到找到 src-tauri 目录
    loop {
        let src_tauri_path = current.join("src-tauri");
        if src_tauri_path.exists() && src_tauri_path.is_dir() {
            return current.to_path_buf();
        }
        
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            // 如果找不到 src-tauri 目录，使用可执行文件所在目录
            return exe_path.parent()
                .expect("failed to get executable directory")
                .to_path_buf();
        }
    }
}

/// 获取 Python 可执行文件路径
fn get_python_path() -> Result<PathBuf, String> {
    let project_root = get_app_base_dir();
    let python_path = project_root.join("python313").join("python.exe");
    
    log::debug!("查找 Python 路径: {}", python_path.display());
    
    if !python_path.exists() {
        return Err(format!("Python 可执行文件不存在: {}", python_path.display()));
    }
    
    Ok(python_path)
}

/// 获取 AI 服务脚本路径
fn get_ai_service_path() -> Result<PathBuf, String> {
    let project_root = get_app_base_dir();
    let service_path = project_root.join("ai_service").join("main.py");
    
    log::debug!("查找 AI 服务脚本: {}", service_path.display());
    
    if !service_path.exists() {
        return Err(format!("AI 服务脚本不存在: {}", service_path.display()));
    }
    
    Ok(service_path)
}

/// 启动 AI 服务
#[tauri::command]
pub fn start_ai_service(
    state: State<'_, AIServiceState>,
) -> Result<String, String> {
    let mut process_guard = state.process.lock()
        .map_err(|e| format!("锁定状态失败: {}", e))?;
    
    // 检查是否已经运行
    if let Some(ref mut child) = *process_guard {
        // 检查进程是否还在运行
        match child.try_wait() {
            Ok(Some(_)) => {
                // 进程已结束，可以重新启动
            }
            Ok(None) => {
                return Err("AI 服务已在运行".to_string());
            }
            Err(e) => {
                return Err(format!("检查进程状态失败: {}", e));
            }
        }
    }
    
    // 获取 Python 和服务脚本路径
    let python_path = get_python_path()?;
    let service_path = get_ai_service_path()?;
    
    log::info!("启动 AI 服务: Python={}, Script={}", python_path.display(), service_path.display());
    
    // 设置环境变量：配置目录和 Wiki 目录
    let config_dir = get_config_dir();
    let wiki_dir = get_app_base_dir().join("wiki");
    log::debug!("设置配置目录环境变量: {}", config_dir.display());
    log::debug!("设置 Wiki 目录环境变量: {}", wiki_dir.display());
    
    // 启动 Python 服务
    let mut cmd = Command::new(&python_path);
    cmd.arg(&service_path)
        .arg("--port")
        .arg("8765")
        .arg("--host")
        .arg("127.0.0.1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(service_path.parent().unwrap())
        .env("NETSEC_TOOLBOX_CONFIG_DIR", &config_dir)
        .env("NETSEC_TOOLBOX_WIKI_DIR", &wiki_dir);
    
    let mut child = cmd.spawn()
        .map_err(|e| {
            let error_msg = format!("启动 AI 服务失败: {} (Python: {}, Script: {})", 
                e, python_path.display(), service_path.display());
            log::error!("{}", error_msg);
            error_msg
        })?;
    
    // 启动后台线程读取 stderr，以便捕获错误信息
    let stderr = child.stderr.take();
    if let Some(stderr) = stderr {
        let stderr_reader = BufReader::new(stderr);
        std::thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    log::error!("[AI Service] {}", line);
                }
            }
        });
    }
    
    // 启动后台线程读取 stdout
    let stdout = child.stdout.take();
    if let Some(stdout) = stdout {
        let stdout_reader = BufReader::new(stdout);
        std::thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    log::info!("[AI Service] {}", line);
                }
            }
        });
    }
    
    *process_guard = Some(child);
    
    log::info!("AI 服务进程已启动");
    Ok("AI 服务已启动".to_string())
}

/// 停止 AI 服务
#[tauri::command]
pub fn stop_ai_service(
    state: State<'_, AIServiceState>,
) -> Result<String, String> {
    let mut process_guard = state.process.lock()
        .map_err(|e| format!("锁定状态失败: {}", e))?;
    
    if let Some(mut child) = process_guard.take() {
        // 尝试优雅关闭
        #[cfg(windows)]
        {
            // Windows 系统：终止进程
            if let Err(e) = child.kill() {
                return Err(format!("停止 AI 服务失败: {}", e));
            }
        }
        
        #[cfg(not(windows))]
        {
            // Unix 系统：发送 SIGTERM
            if let Err(e) = child.kill() {
                return Err(format!("停止 AI 服务失败: {}", e));
            }
        }
        
        // 等待进程结束
        let _ = child.wait();
        
        Ok("AI 服务已停止".to_string())
    } else {
        Err("AI 服务未运行".to_string())
    }
}

/// 检查 AI 服务状态
#[tauri::command]
pub fn check_ai_service_status(
    state: State<'_, AIServiceState>,
) -> Result<bool, String> {
    let mut process_guard = state.process.lock()
        .map_err(|e| format!("锁定状态失败: {}", e))?;
    
    if let Some(child) = process_guard.as_mut() {
        match child.try_wait() {
            Ok(Some(status)) => {
                // 进程已结束，记录退出状态
                log::warn!("AI 服务进程已结束，退出状态: {:?}", status);
                Ok(false) // 进程已结束
            }
            Ok(None) => Ok(true),     // 进程正在运行
            Err(e) => Err(format!("检查进程状态失败: {}", e)),
        }
    } else {
        Ok(false) // 未启动
    }
}


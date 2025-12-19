use std::path::PathBuf;
use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use tauri::State;
use crate::utils::get_config_dir;

/// AI 服务进程状态
#[derive(Default)]
pub struct AIServiceState {
    process: Arc<Mutex<Option<Child>>>,
}

/// 获取 Python 可执行文件路径
fn get_python_path() -> Result<PathBuf, String> {
    // 获取项目根目录（假设可执行文件在 src-tauri/target/... 下）
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("无法获取可执行文件路径: {}", e))?;
    
    // 从可执行文件路径推导项目根目录
    // 例如：target/release/netsec-toolbox.exe -> 项目根目录
    let mut project_root = exe_path.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| "无法确定项目根目录".to_string())?
        .to_path_buf();
    
    // 如果是开发模式，路径可能不同
    if project_root.ends_with("target") {
        project_root = project_root.parent()
            .ok_or_else(|| "无法确定项目根目录".to_string())?
            .to_path_buf();
    }
    
    // 构建 Python 路径
    let python_path = project_root.join("python313").join("python.exe");
    
    if !python_path.exists() {
        return Err(format!("Python 可执行文件不存在: {}", python_path.display()));
    }
    
    Ok(python_path)
}

/// 获取 AI 服务脚本路径
fn get_ai_service_path() -> Result<PathBuf, String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("无法获取可执行文件路径: {}", e))?;
    
    let mut project_root = exe_path.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or_else(|| "无法确定项目根目录".to_string())?
        .to_path_buf();
    
    if project_root.ends_with("target") {
        project_root = project_root.parent()
            .ok_or_else(|| "无法确定项目根目录".to_string())?
            .to_path_buf();
    }
    
    let service_path = project_root.join("ai_service").join("main.py");
    
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
    
    // 设置环境变量：配置目录
    let config_dir = get_config_dir();
    std::env::set_var("NETSEC_TOOLBOX_CONFIG_DIR", &config_dir);
    
    // 启动 Python 服务
    let mut cmd = Command::new(&python_path);
    cmd.arg(&service_path)
        .arg("--port")
        .arg("8765")
        .arg("--host")
        .arg("127.0.0.1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(service_path.parent().unwrap());
    
    let child = cmd.spawn()
        .map_err(|e| format!("启动 AI 服务失败: {}", e))?;
    
    *process_guard = Some(child);
    
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
    
    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(Some(_)) => Ok(false), // 进程已结束
            Ok(None) => Ok(true),     // 进程正在运行
            Err(e) => Err(format!("检查进程状态失败: {}", e)),
        }
    } else {
        Ok(false) // 未启动
    }
}


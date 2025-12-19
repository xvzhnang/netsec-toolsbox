use std::process::{Command, Child, Stdio};
use std::sync::{Arc, Mutex};
use tauri::State;
use crate::utils::get_app_base_dir;

/// AI 服务状态管理
#[derive(Default)]
pub struct AIServiceState {
    process: Arc<Mutex<Option<Child>>>,
}

/// 获取 Python 可执行文件路径
fn get_python_path() -> std::path::PathBuf {
    let base_dir = get_app_base_dir();
    let python_path = base_dir.join("python313").join("python.exe");
    
    log::debug!("get_python_path: 项目根目录: {}, Python 路径: {}", 
                base_dir.display(), python_path.display());
    
    python_path
}

/// 获取 AI Gateway 服务脚本路径
fn get_ai_service_path() -> std::path::PathBuf {
    let base_dir = get_app_base_dir();
    let service_path = base_dir.join("ai_service").join("main_gateway.py");
    
    log::debug!("get_ai_service_path: 项目根目录: {}, 服务脚本路径: {}", 
                base_dir.display(), service_path.display());
    
    service_path
}

/// 启动 AI Gateway 服务
#[tauri::command]
pub fn start_ai_service(state: State<AIServiceState>) -> Result<String, String> {
    let mut process_guard = state.process.lock().unwrap();
    
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
    
    log::info!("启动 AI Gateway 服务: {} {}", 
               python_path.display(), service_path.display());
    
    // 启动 Python 服务
    let child = Command::new(&python_path)
        .arg(&service_path)
        .arg("--port")
        .arg("8765")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 AI Gateway 服务失败: {}", e))?;
    
    *process_guard = Some(child);
    
    log::info!("AI Gateway 服务已启动");
    Ok("AI Gateway 服务已启动".to_string())
}

/// 停止 AI Gateway 服务
#[tauri::command]
pub fn stop_ai_service(state: State<AIServiceState>) -> Result<String, String> {
    let mut process_guard = state.process.lock().unwrap();
    
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
    let process_guard = state.process.lock().unwrap();
    
    if let Some(child) = process_guard.as_ref() {
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


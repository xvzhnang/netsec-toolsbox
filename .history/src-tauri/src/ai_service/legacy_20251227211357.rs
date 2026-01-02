use crate::utils::get_app_base_dir;
use std::fs;

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

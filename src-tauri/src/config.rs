use std::fs;
use crate::utils;

/// 通用的读取配置文件命令
#[tauri::command]
pub fn read_config_file(filename: String) -> Result<String, String> {
  let config_path = utils::get_config_dir().join(&filename);
  if config_path.exists() {
    fs::read_to_string(&config_path)
      .map_err(|e| format!("Failed to read config file {}: {}", filename, e))
  } else {
    // 返回空JSON，前端会使用默认值
    Ok("{}".to_string())
  }
}

/// 通用的写入配置文件命令
#[tauri::command]
pub fn write_config_file(filename: String, content: String) -> Result<(), String> {
  let config_path = utils::get_config_dir().join(&filename);
  fs::write(&config_path, content)
    .map_err(|e| format!("Failed to write config file {}: {}", filename, e))
}

/// 检查配置文件是否存在
#[tauri::command]
pub fn config_file_exists(filename: String) -> Result<bool, String> {
  let config_path = utils::get_config_dir().join(&filename);
  Ok(config_path.exists())
}

/// 兼容旧版本的命令（保留以向后兼容）
#[tauri::command]
pub fn read_categories_config(_app: tauri::AppHandle) -> Result<String, String> {
  read_config_file("categories.json".to_string())
}

#[tauri::command]
pub fn write_categories_config(_app: tauri::AppHandle, content: String) -> Result<(), String> {
  write_config_file("categories.json".to_string(), content)
}

/// 获取配置文件路径（用于显示给用户）
#[tauri::command]
pub fn get_config_file_path(filename: String) -> Result<String, String> {
  let config_path = utils::get_config_dir().join(&filename);
  config_path
    .to_str()
    .ok_or("无法转换路径为字符串".to_string())
    .map(|s| s.to_string())
}


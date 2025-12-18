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

/// 读取图标文件并返回 base64 数据 URL
/// 支持相对路径（如 "icons/xxx.png"）和绝对路径
#[tauri::command]
pub fn read_icon_file(icon_path: String) -> Result<String, String> {
  use base64::{Engine as _, engine::general_purpose};
  
  log::info!("read_icon_file: 开始读取图标文件: {}", icon_path);
  
  // 如果是相对路径（如 "icons/xxx.png" 或 ".config/icons/xxx.png"），从配置目录读取
  let icon_file_path = if icon_path.starts_with("icons/") {
    let config_dir = utils::get_config_dir();
    let full_path = config_dir.join(&icon_path);
    log::info!("read_icon_file: 相对路径 (icons/)，完整路径: {}", full_path.display());
    full_path
  } else if icon_path.starts_with(".config/icons/") {
    // 支持 .config/icons/ 格式（与 icons/ 等价）
    let config_dir = utils::get_config_dir();
    // 移除 .config/ 前缀，因为 config_dir 已经是 .config 目录
    let relative_path = icon_path.strip_prefix(".config/").unwrap_or(&icon_path);
    let full_path = config_dir.join(relative_path);
    log::info!("read_icon_file: 相对路径 (.config/icons/)，完整路径: {}", full_path.display());
    full_path
  } else {
    // 绝对路径
    let full_path = std::path::PathBuf::from(&icon_path);
    log::info!("read_icon_file: 绝对路径: {}", full_path.display());
    full_path
  };
  
  if !icon_file_path.exists() {
    let error_msg = format!("图标文件不存在: {} (完整路径: {})", icon_path, icon_file_path.display());
    log::error!("{}", error_msg);
    return Err(error_msg);
  }
  
  log::info!("read_icon_file: 文件存在，开始读取: {}", icon_file_path.display());
  
  // 读取文件
  let file_data = fs::read(&icon_file_path)
    .map_err(|e| {
      let error_msg = format!("读取图标文件失败: {} (路径: {})", e, icon_file_path.display());
      log::error!("{}", error_msg);
      error_msg
    })?;
  
  log::info!("read_icon_file: 文件读取成功，大小: {} 字节", file_data.len());
  
  // 转换为 base64 数据 URL
  let base64_str = general_purpose::STANDARD.encode(&file_data);
  
  // 根据文件扩展名确定 MIME 类型
  let mime_type = if icon_path.ends_with(".png") {
    "image/png"
  } else if icon_path.ends_with(".jpg") || icon_path.ends_with(".jpeg") {
    "image/jpeg"
  } else if icon_path.ends_with(".gif") {
    "image/gif"
  } else if icon_path.ends_with(".svg") {
    "image/svg+xml"
  } else {
    "image/png" // 默认使用 PNG
  };
  
  let result = format!("data:{};base64,{}", mime_type, base64_str);
  log::info!("read_icon_file: 转换完成，base64 长度: {}", result.len());
  
  Ok(result)
}


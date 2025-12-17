use std::fs;
use std::path::Path;
use base64::{Engine as _, engine::general_purpose};
use crate::types::{UploadFileParams, ResolveFilePathParams};
use crate::utils::get_uploads_dir;

/// 打开文件对话框的参数结构体
#[derive(Debug, serde::Deserialize)]
pub struct OpenFileDialogParams {
  #[serde(alias = "filters", default)]
  pub filters: Option<Vec<FileFilter>>,
  #[serde(alias = "defaultPath", alias = "default_path", default)]
  pub default_path: Option<String>,
}

/// 文件过滤器
#[derive(Debug, serde::Deserialize)]
pub struct FileFilter {
  pub name: String,
  pub extensions: Vec<String>,
}

/// 打开文件选择对话框（后端实现，作为前端 API 不可用时的降级方案）
#[tauri::command]
pub fn open_file_dialog(params: OpenFileDialogParams) -> Result<Option<String>, String> {
  #[cfg(target_os = "windows")]
  {
    use std::process::Command;
    
    // 在 Windows 上，使用 PowerShell 打开文件对话框
    // 使用 System.Windows.Forms.OpenFileDialog
    let mut filter_parts = Vec::new();
    if let Some(ref filters) = params.filters {
      for filter in filters {
        let ext_list: Vec<String> = filter.extensions.iter()
          .map(|e| format!("*.{}", e))
          .collect();
        filter_parts.push(format!("{}|{}", filter.name, ext_list.join(";")));
      }
    }
    let filter_str = if !filter_parts.is_empty() {
      format!("$dialog.Filter = \"{}\"", filter_parts.join("|").replace('"', "\""))
    } else {
      String::new()
    };
    
    let default_path_str = if let Some(ref path) = params.default_path {
      format!("$dialog.InitialDirectory = \"{}\"", path.replace('\\', "\\\\").replace('"', "\""))
    } else {
      String::new()
    };
    
    // 使用 PowerShell 的 [System.Windows.Forms.OpenFileDialog]
    let script = format!(r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = "选择文件"
$dialog.Multiselect = $false
{}
{}
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
  Write-Output $dialog.FileName
}}
"#, filter_str, default_path_str);
    
    let output = Command::new("powershell")
      .arg("-NoProfile")
      .arg("-WindowStyle")
      .arg("Hidden")
      .arg("-Command")
      .arg(&script)
      .output()
      .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
    
    if output.status.success() {
      let stdout = String::from_utf8_lossy(&output.stdout);
      let file_path = stdout.trim();
      if file_path.is_empty() {
        Ok(None) // 用户取消
      } else {
        // 解析路径并返回绝对路径
        resolve_file_path(ResolveFilePathParams {
          file_path: file_path.to_string(),
        }).map(|p| Some(p))
      }
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      if stderr.trim().is_empty() {
        Ok(None) // 用户取消
      } else {
        Err(format!("打开文件对话框失败: {}", stderr))
      }
    }
  }
  
  #[cfg(not(target_os = "windows"))]
  {
    // 在非 Windows 系统上，可以使用 zenity (Linux) 或 osascript (macOS)
    // 这里先返回错误，提示使用前端 API
    Err("文件对话框功能需要前端 Tauri dialog API 支持，或安装 @tauri-apps/plugin-dialog 插件".to_string())
  }
}

/// 上传文件并保存到 uploads 目录
/// 返回保存后的文件路径
#[tauri::command]
pub fn upload_file(params: UploadFileParams) -> Result<String, String> {
  let file_name = params.file_name;
  let file_data = params.file_data;
  let tool_id = params.tool_id;
  
  // 解码 base64 数据
  let file_bytes = general_purpose::STANDARD.decode(
    file_data.strip_prefix("data:").and_then(|s| s.split(',').nth(1))
      .unwrap_or(&file_data)
  ).map_err(|e| format!("Base64 解码失败: {}", e))?;
  
  // 确定保存目录
  let uploads_dir = if let Some(id) = tool_id {
    // 如果有工具ID，创建子目录
    get_uploads_dir().join(id)
  } else {
    get_uploads_dir()
  };
  
  // 确保目录存在
  std::fs::create_dir_all(&uploads_dir)
    .map_err(|e| format!("创建上传目录失败: {}", e))?;
  
  // 生成安全的文件名（防止路径遍历攻击）
  let safe_file_name = Path::new(&file_name)
    .file_name()
    .and_then(|n| n.to_str())
    .ok_or("无效的文件名")?;
  
  // 如果文件已存在，添加时间戳后缀
  let mut final_path = uploads_dir.join(safe_file_name);
  if final_path.exists() {
    let stem = final_path.file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or("file");
    let extension = final_path.extension()
      .and_then(|s| s.to_str())
      .unwrap_or("");
    let timestamp = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs();
    let new_name = if extension.is_empty() {
      format!("{}_{}", stem, timestamp)
    } else {
      format!("{}_{}.{}", stem, timestamp, extension)
    };
    final_path = uploads_dir.join(new_name);
  }
  
  // 保存文件
  fs::write(&final_path, &file_bytes)
    .map_err(|e| format!("保存文件失败: {}", e))?;
  
  // 确保返回绝对路径
  let abs_path = final_path.canonicalize()
    .unwrap_or(final_path);
  
  // 返回保存后的文件路径（绝对路径）
  Ok(abs_path.to_string_lossy().to_string())
}

/// 解析文件路径为绝对路径
/// 支持相对路径和绝对路径，总是返回规范化的绝对路径
/// 如果是相对路径且在当前目录找不到，会在 PATH 环境变量中查找
#[tauri::command]
pub fn resolve_file_path(params: ResolveFilePathParams) -> Result<String, String> {
  let file_path = params.file_path;
  let path = Path::new(&file_path);
  
  log::info!("解析文件路径: 原始='{}'", file_path);
  
  // 如果已经是绝对路径，直接规范化
  let abs_path = if path.is_absolute() {
    path.canonicalize()
      .map_err(|e| format!("无法解析路径 {}: {}", file_path, e))?
  } else {
    // 如果是相对路径，尝试从当前工作目录解析
    let current_dir = std::env::current_dir()
      .map_err(|e| format!("获取当前目录失败: {}", e))?;
    
    let joined_path = current_dir.join(path);
    
    // 如果文件不存在，尝试在 PATH 环境变量中查找
    if !joined_path.exists() {
      // 尝试在 PATH 中查找可执行文件
      if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        #[cfg(target_os = "windows")]
        {
          // 在 Windows 上，尝试在 PATH 中查找
          if let Ok(path_env) = std::env::var("PATH") {
            for path_dir in path_env.split(';') {
              let test_path = Path::new(path_dir).join(file_name);
              if test_path.exists() {
                let canonical = test_path.canonicalize()
                  .map_err(|e| format!("无法解析路径 {}: {}", file_path, e))?;
                let final_path_str = canonical.to_string_lossy().to_string();
                let final_path_str = if final_path_str.starts_with("\\\\?\\") {
                  &final_path_str[4..]
                } else {
                  &final_path_str
                }.to_string();
                log::info!("解析文件路径: 在 PATH 中找到='{}'", final_path_str);
                return Ok(final_path_str);
              }
            }
          }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
          // 在 Unix 系统上，尝试在 PATH 中查找
          if let Ok(path_env) = std::env::var("PATH") {
            for path_dir in path_env.split(':') {
              let test_path = Path::new(path_dir).join(file_name);
              if test_path.exists() {
                let canonical = test_path.canonicalize()
                  .map_err(|e| format!("无法解析路径 {}: {}", file_path, e))?;
                let final_path_str = canonical.to_string_lossy().to_string();
                log::info!("解析文件路径: 在 PATH 中找到='{}'", final_path_str);
                return Ok(final_path_str);
              }
            }
          }
        }
      }
      
      // 如果仍然找不到，返回错误
      return Err(format!("无法解析路径 {}: 文件不存在", file_path));
    }
    
    joined_path.canonicalize()
      .map_err(|e| format!("无法解析路径 {}: {}", file_path, e))?
  };
  
  // 移除 Windows 长路径前缀 (\\?\)，返回标准路径格式
  let mut final_path_str = abs_path.to_string_lossy().to_string();
  if final_path_str.starts_with("\\\\?\\") {
    final_path_str = final_path_str[4..].to_string();
  }
  
  log::info!("解析文件路径: 绝对='{}'", final_path_str);
  Ok(final_path_str)
}


// Wiki 相关的 Tauri 命令
use crate::wiki::server::WikiServer;
use std::sync::Mutex;

// 全局 Wiki 服务器实例
static WIKI_SERVER: Mutex<Option<WikiServer>> = Mutex::new(None);

/// 启动 Wiki 服务器
#[tauri::command]
pub fn start_wiki_server() -> Result<String, String> {
  let mut server_guard = WIKI_SERVER.lock().unwrap();
  
  if server_guard.is_none() {
    let mut server = WikiServer::new();
    server.start()?;
    *server_guard = Some(server);
    Ok("Wiki 服务器已启动".to_string())
  } else {
    Ok("Wiki 服务器已在运行".to_string())
  }
}

/// 停止 Wiki 服务器
#[tauri::command]
pub fn stop_wiki_server() -> Result<String, String> {
  let mut server_guard = WIKI_SERVER.lock().unwrap();
  
  if let Some(mut server) = server_guard.take() {
    server.stop();
    Ok("Wiki 服务器已停止".to_string())
  } else {
    Ok("Wiki 服务器未运行".to_string())
  }
}

/// 获取 Wiki 文件列表
#[tauri::command]
pub fn get_wiki_files() -> Result<Vec<crate::wiki::types::WikiFileInfo>, String> {
  let server_guard = WIKI_SERVER.lock().unwrap();
  
  if let Some(server) = server_guard.as_ref() {
    server.list_files()
  } else {
    // 如果服务器未启动，创建临时实例获取文件列表
    let server = WikiServer::new();
    server.list_files()
  }
}

/// 渲染 Markdown 文件
#[tauri::command]
pub fn render_wiki_file(file_path: String) -> Result<crate::wiki::types::RenderResult, String> {
  let server_guard = WIKI_SERVER.lock().unwrap();
  
  if let Some(server) = server_guard.as_ref() {
    server.render_file(&file_path)
  } else {
    // 如果服务器未启动，创建临时实例渲染文件
    let server = WikiServer::new();
    server.render_file(&file_path)
  }
}

/// 搜索 Wiki
#[tauri::command]
pub fn search_wiki(query: String) -> Result<Vec<crate::wiki::types::SearchResult>, String> {
  let server_guard = WIKI_SERVER.lock().unwrap();
  
  if let Some(server) = server_guard.as_ref() {
    server.search(&query)
  } else {
    // 如果服务器未启动，创建临时实例搜索
    let server = WikiServer::new();
    server.search(&query)
  }
}

/// 获取 Wiki 目录路径
#[tauri::command]
pub fn get_wiki_dir() -> Result<String, String> {
  let server = WikiServer::new();
  Ok(server.get_wiki_dir().to_string_lossy().to_string())
}

/// 根据工具 ID 或名称查找对应的 Wiki 文件
#[tauri::command]
pub fn find_wiki_for_tool(tool_id: String, tool_name: Option<String>) -> Result<Option<String>, String> {
  let server = WikiServer::new();
  let files = server.list_files()?;
  
  // 尝试多种匹配方式：
  // 1. 精确匹配工具 ID
  // 2. 匹配工具名称（不区分大小写）
  // 3. 匹配文件名包含工具 ID 或名称
  
  let tool_name_lower = tool_name.as_ref()
    .map(|n| n.to_lowercase())
    .unwrap_or_default();
  let tool_id_lower = tool_id.to_lowercase();
  
  // 递归搜索文件
  fn search_files(files: &[crate::wiki::types::WikiFileInfo], 
                  tool_id: &str, 
                  tool_name: &str) -> Option<String> {
    for file in files {
      if file.is_dir {
        if let Some(children) = &file.children {
          if let Some(found) = search_files(children, tool_id, tool_name) {
            return Some(found);
          }
        }
      } else {
        // 检查文件名或路径是否匹配
        let name_lower = file.name.to_lowercase();
        let path_lower = file.path.to_lowercase();
        let title_lower = file.title.to_lowercase();
        
        // 移除 .md 扩展名进行比较
        let name_without_ext = name_lower.trim_end_matches(".md");
        let path_without_ext = path_lower.trim_end_matches(".md");
        
        // 匹配逻辑：
        // 1. 文件名完全匹配工具 ID 或名称
        // 2. 文件名包含工具 ID 或名称
        // 3. 路径包含工具 ID 或名称
        // 4. 标题包含工具 ID 或名称
        if name_without_ext == tool_id || 
           name_without_ext == tool_name ||
           name_without_ext.contains(&tool_id) ||
           name_without_ext.contains(&tool_name) ||
           path_without_ext.contains(&tool_id) ||
           path_without_ext.contains(&tool_name) ||
           title_lower.contains(&tool_id) ||
           title_lower.contains(&tool_name) {
          return Some(file.path.clone());
        }
      }
    }
    None
  }
  
  Ok(search_files(&files, &tool_id_lower, &tool_name_lower))
}


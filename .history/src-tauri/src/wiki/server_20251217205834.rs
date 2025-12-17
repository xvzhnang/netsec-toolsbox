use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::wiki::parser::MarkdownParser;
use crate::wiki::types::*;
use crate::utils::get_config_dir;

/// Wiki 服务器状态
pub struct WikiServer {
  wiki_dir: PathBuf,
  parser: MarkdownParser,
  port: u16,
  server_handle: Option<thread::JoinHandle<()>>,
  is_running: Arc<Mutex<bool>>,
}

impl WikiServer {
  pub fn new() -> Self {
    let config_dir = get_config_dir();
    let wiki_dir = config_dir.join("wiki");
    
    // 确保 Wiki 目录存在
    if !wiki_dir.exists() {
      if let Err(e) = fs::create_dir_all(&wiki_dir) {
        log::warn!("创建 Wiki 目录失败: {}", e);
      }
    }
    
    Self {
      wiki_dir,
      parser: MarkdownParser::new(),
      port: 8777,
      server_handle: None,
      is_running: Arc::new(Mutex::new(false)),
    }
  }

  /// 启动 Wiki 服务器
  pub fn start(&mut self) -> Result<(), String> {
    let mut is_running = self.is_running.lock().unwrap();
    if *is_running {
      return Ok(()); // 已经运行
    }
    *is_running = true;
    drop(is_running);
    
    let wiki_dir = self.wiki_dir.clone();
    let port = self.port;
    let is_running_clone = Arc::clone(&self.is_running);
    
    // 使用 axum HTTP 服务器
    let handle = thread::spawn(move || {
      start_simple_server(wiki_dir, port, is_running_clone);
    });
    
    self.server_handle = Some(handle);
    
    // 等待服务器启动
    thread::sleep(Duration::from_millis(500));
    
    log::info!("Wiki 服务器已启动在端口 {}", self.port);
    Ok(())
  }

  /// 停止 Wiki 服务器
  pub fn stop(&mut self) {
    let mut is_running = self.is_running.lock().unwrap();
    *is_running = false;
    drop(is_running);
    
    if let Some(handle) = self.server_handle.take() {
      handle.join().ok();
    }
    
    log::info!("Wiki 服务器已停止");
  }

  /// 获取 Wiki 目录
  pub fn get_wiki_dir(&self) -> &Path {
    &self.wiki_dir
  }

  /// 获取文件列表
  pub fn list_files(&self) -> Result<Vec<WikiFileInfo>, String> {
    list_wiki_files(&self.wiki_dir, &self.wiki_dir)
  }

  /// 渲染 Markdown 文件
  pub fn render_file(&self, file_path: &str) -> Result<RenderResult, String> {
    let full_path = self.wiki_dir.join(file_path);
    
    if !full_path.exists() {
      return Err(format!("文件不存在: {}", file_path));
    }
    
    self.parser.render_file(&full_path)
  }

  /// 搜索 Wiki
  pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, String> {
    search_wiki_files(&self.wiki_dir, query)
  }
}

/// 启动简单的 HTTP 服务器
/// 注意：这是一个简化实现，实际应该使用更完善的 HTTP 库
fn start_simple_server(
  wiki_dir: PathBuf,
  port: u16,
  is_running: Arc<Mutex<bool>>,
) {
  // 这里使用简化实现
  // 实际应该使用 axum 或 warp 等库
  log::info!("Wiki HTTP 服务器启动在端口 {} (简化实现)", port);
  
  // 保持运行
  while *is_running.lock().unwrap() {
    thread::sleep(Duration::from_secs(1));
  }
}

/// 列出 Wiki 文件（公开函数，供 http_server 使用）
pub fn list_wiki_files(root: &Path, current: &Path) -> Result<Vec<WikiFileInfo>, String> {
  let mut files = Vec::new();
  
  if !current.exists() {
    return Ok(files);
  }
  
  let entries = fs::read_dir(current)
    .map_err(|e| format!("读取目录失败: {}", e))?;
  
  for entry in entries {
    let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
    let path = entry.path();
    let metadata = entry.metadata()
      .map_err(|e| format!("获取文件信息失败: {}", e))?;
    
    if metadata.is_dir() {
      // 跳过隐藏目录和特殊目录
      if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') {
          continue;
        }
      }
      
      let children = list_wiki_files(root, &path)?;
      let relative_path = path.strip_prefix(root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
      
      files.push(WikiFileInfo {
        path: relative_path,
        name: path.file_name()
          .and_then(|n| n.to_str())
          .unwrap_or("")
          .to_string(),
        title: path.file_name()
          .and_then(|n| n.to_str())
          .unwrap_or("")
          .to_string(),
        is_dir: true,
        children: Some(children),
      });
    } else if metadata.is_file() {
      // 只处理 .md 文件
      if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext.to_lowercase() == "md" {
          let relative_path = path.strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
          
          // 尝试从文件第一行提取标题
          let title = extract_file_title(&path).unwrap_or_else(|| {
            path.file_stem()
              .and_then(|n| n.to_str())
              .unwrap_or("")
              .to_string()
          });
          
          files.push(WikiFileInfo {
            path: relative_path,
            name: path.file_name()
              .and_then(|n| n.to_str())
              .unwrap_or("")
              .to_string(),
            title,
            is_dir: false,
            children: None,
          });
        }
      }
    }
  }
  
  // 排序：目录在前，然后按名称排序
  files.sort_by(|a, b| {
    match (a.is_dir, b.is_dir) {
      (true, false) => std::cmp::Ordering::Less,
      (false, true) => std::cmp::Ordering::Greater,
      _ => a.name.cmp(&b.name),
    }
  });
  
  Ok(files)
}

/// 从文件第一行提取标题
fn extract_file_title(file_path: &Path) -> Option<String> {
  if let Ok(content) = fs::read_to_string(file_path) {
    for line in content.lines() {
      let trimmed = line.trim();
      if trimmed.starts_with("# ") {
        return Some(trimmed[2..].trim().to_string());
      }
    }
  }
  None
}

/// 搜索 Wiki 文件
fn search_wiki_files(root: &Path, query: &str) -> Result<Vec<SearchResult>, String> {
  let mut results = Vec::new();
  let query_lower = query.to_lowercase();
  
  search_directory(root, root, &query_lower, &mut results)?;
  
  Ok(results)
}

/// 递归搜索目录
fn search_directory(
  root: &Path,
  current: &Path,
  query: &str,
  results: &mut Vec<SearchResult>,
) -> Result<(), String> {
  if !current.exists() {
    return Ok(());
  }
  
  let entries = fs::read_dir(current)
    .map_err(|e| format!("读取目录失败: {}", e))?;
  
  for entry in entries {
    let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
    let path = entry.path();
    let metadata = entry.metadata()
      .map_err(|e| format!("获取文件信息失败: {}", e))?;
    
    if metadata.is_dir() {
      // 递归搜索子目录
      search_directory(root, &path, query, results)?;
    } else if metadata.is_file() {
      // 只搜索 .md 文件
      if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext.to_lowercase() == "md" {
          if let Ok(content) = fs::read_to_string(&path) {
            let matches = search_in_content(&content, query);
            if !matches.is_empty() {
              let relative_path = path.strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
              
              let title = extract_file_title(&path).unwrap_or_else(|| {
                path.file_stem()
                  .and_then(|n| n.to_str())
                  .unwrap_or("")
                  .to_string()
              });
              
              results.push(SearchResult {
                file_path: relative_path,
                title,
                matches,
              });
            }
          }
        }
      }
    }
  }
  
  Ok(())
}

/// 在内容中搜索
fn search_in_content(content: &str, query: &str) -> Vec<SearchMatch> {
  let mut matches = Vec::new();
  let content_lower = content.to_lowercase();
  
  for (line_num, line) in content.lines().enumerate() {
    if line.to_lowercase().contains(query) {
      // 提取匹配的上下文（前后各 50 个字符）
      let start = line.to_lowercase().find(query).unwrap_or(0);
      let end = start + query.len();
      let context_start = start.saturating_sub(50);
      let context_end = (end + 50).min(line.len());
      let text = line[context_start..context_end].trim().to_string();
      
      matches.push(SearchMatch {
        line: (line_num + 1) as u32,
        text,
      });
    }
  }
  
  matches
}


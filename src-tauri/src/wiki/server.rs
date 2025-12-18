use std::fs;
use std::path::{Path, PathBuf};
use crate::wiki::types::*;
use crate::utils::{get_wiki_dir, get_docs_dir};

/// Wiki 服务器（简化版，只负责文件操作，不启动 HTTP 服务器）
pub struct WikiServer {
  wiki_dir: PathBuf,
}

impl WikiServer {
  pub fn new() -> Self {
    let wiki_dir = get_wiki_dir();
    let docs_dir = get_docs_dir();
    
    // 确保 Wiki 目录和子目录存在
    if !wiki_dir.exists() {
      if let Err(e) = fs::create_dir_all(&wiki_dir) {
        log::warn!("创建 Wiki 目录失败: {}", e);
      }
    }
    if !docs_dir.exists() {
      if let Err(e) = fs::create_dir_all(&docs_dir) {
        log::warn!("创建 Wiki 目录失败: {}", e);
      }
    }
    
    // 确保子目录存在
    let _ = fs::create_dir_all(&docs_dir.join("tools"));
    let _ = fs::create_dir_all(&docs_dir.join("notes"));
    let _ = fs::create_dir_all(&docs_dir.join("labs"));
    
    Self {
      wiki_dir: docs_dir, // 使用 docs_dir 作为基础目录
    }
  }

  /// 获取 Wiki 目录
  pub fn get_wiki_dir(&self) -> &Path {
    &self.wiki_dir
  }

  /// 获取文件列表
  pub fn list_files(&self) -> Result<Vec<WikiFileInfo>, String> {
    list_wiki_files(&self.wiki_dir, &self.wiki_dir)
  }

  /// 搜索 Wiki
  pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, String> {
    search_wiki_files(&self.wiki_dir, query)
  }
}

/// 列出 Wiki 文件
pub fn list_wiki_files(root: &Path, current: &Path) -> Result<Vec<WikiFileInfo>, String> {
  let mut files = Vec::new();
  
  if !current.exists() {
    return Ok(files);
  }
  
  let entries = fs::read_dir(current)
    .map_err(|e| format!("读取目录失败: {}", e))?;
  
  let mut dirs = Vec::new();
  let mut md_files = Vec::new();
  
  for entry in entries {
    let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
    let path = entry.path();
    let metadata = entry.metadata()
      .map_err(|e| format!("获取文件元数据失败: {}", e))?;
    
    if metadata.is_dir() {
      // 跳过隐藏目录和特殊目录
      if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') || name == "node_modules" || name == "target" {
          continue;
        }
        dirs.push(path);
      }
    } else if metadata.is_file() {
      // 只处理 Markdown 文件
      if let Some(ext) = path.extension() {
        if ext == "md" || ext == "markdown" {
          md_files.push(path);
        }
      }
    }
  }
  
  // 先添加目录
  dirs.sort();
  for dir_path in dirs {
    if let Some(name) = dir_path.file_name().and_then(|n| n.to_str()) {
      let relative_path = dir_path.strip_prefix(root)
        .unwrap_or(&dir_path)
        .to_string_lossy()
        .to_string();
      
      let children = list_wiki_files(root, &dir_path)?;
      
      files.push(WikiFileInfo {
        name: name.to_string(),
        path: relative_path,
        title: name.to_string(),
        is_dir: true,
        children: if children.is_empty() { None } else { Some(children) },
      });
    }
  }
  
  // 再添加文件
  md_files.sort();
  for file_path in md_files {
    if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
      let relative_path = file_path.strip_prefix(root)
        .unwrap_or(&file_path)
        .to_string_lossy()
        .to_string();
      
      // 从文件内容提取标题
      let title = extract_title_from_file(&file_path).unwrap_or_else(|| {
        name.trim_end_matches(".md")
          .trim_end_matches(".markdown")
          .to_string()
      });
      
      files.push(WikiFileInfo {
        name: name.to_string(),
        path: relative_path,
        title,
        is_dir: false,
        children: None,
      });
    }
  }
  
  Ok(files)
}

/// 从 Markdown 文件提取标题
fn extract_title_from_file(file_path: &Path) -> Option<String> {
  if let Ok(content) = fs::read_to_string(file_path) {
    for line in content.lines() {
      let trimmed = line.trim();
      if trimmed.starts_with("# ") {
        return Some(trimmed[2..].trim().to_string());
      } else if trimmed.starts_with("## ") {
        return Some(trimmed[3..].trim().to_string());
      }
    }
  }
  None
}

/// 搜索 Wiki 文件
pub fn search_wiki_files(root: &Path, query: &str) -> Result<Vec<SearchResult>, String> {
  let query_lower = query.to_lowercase();
  let mut results = Vec::new();
  
  fn search_recursive(
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
        .map_err(|e| format!("获取文件元数据失败: {}", e))?;
      
      if metadata.is_dir() {
        // 递归搜索子目录
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
          if !name.starts_with('.') && name != "node_modules" && name != "target" {
            search_recursive(root, &path, query, results)?;
          }
        }
      } else if metadata.is_file() {
        // 搜索 Markdown 文件
        if let Some(ext) = path.extension() {
          if ext == "md" || ext == "markdown" {
            if let Ok(content) = fs::read_to_string(&path) {
              let content_lower = content.to_lowercase();
              
              // 检查文件名或内容是否包含查询
              let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
              
              if file_name.contains(&query) || content_lower.contains(&query) {
                let relative_path = path.strip_prefix(root)
                  .unwrap_or(&path)
                  .to_string_lossy()
                  .to_string();
                
                let title = extract_title_from_file(&path)
                  .unwrap_or_else(|| {
                    file_name.trim_end_matches(".md")
                      .trim_end_matches(".markdown")
                      .to_string()
                  });
                
                results.push(SearchResult {
                  file_path: relative_path,
                  title,
                });
              }
            }
          }
        }
      }
    }
    
    Ok(())
  }
  
  search_recursive(root, root, &query_lower, &mut results)?;
  Ok(results)
}

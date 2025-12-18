use serde::{Deserialize, Serialize};

/// Wiki 文件信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WikiFileInfo {
  pub path: String,           // 相对路径，如 "tools/tool1.md"
  pub name: String,           // 文件名，如 "tool1.md"
  pub title: String,          // 文件标题（从 Markdown 第一行提取）
  pub is_dir: bool,          // 是否为目录
  pub children: Option<Vec<WikiFileInfo>>, // 子文件/目录（仅目录有）
}


/// 搜索结果
#[derive(Debug, Serialize)]
pub struct SearchResult {
  pub file_path: String,      // 文件路径
  pub title: String,          // 文件标题
}


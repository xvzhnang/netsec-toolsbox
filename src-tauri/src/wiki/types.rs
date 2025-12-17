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

/// Wiki 目录树
#[derive(Debug, Serialize, Deserialize)]
pub struct WikiTree {
  pub files: Vec<WikiFileInfo>,
}

/// 渲染 Markdown 的请求参数
#[derive(Debug, Deserialize)]
pub struct RenderMarkdownParams {
  #[serde(alias = "filePath", alias = "file_path")]
  pub file_path: String,      // 相对路径，如 "tools/tool1.md"
}

/// 渲染结果
#[derive(Debug, Serialize)]
pub struct RenderResult {
  pub html: String,           // 渲染后的 HTML
  pub toc: Option<Vec<TocItem>>, // 目录结构
  pub title: String,          // 页面标题
}

/// 目录项
#[derive(Debug, Serialize, Clone)]
pub struct TocItem {
  pub id: String,            // 锚点 ID
  pub text: String,          // 标题文本
  pub level: u32,            // 标题级别（1-6）
  pub children: Vec<TocItem>, // 子标题
}

/// 搜索参数
#[derive(Debug, Deserialize)]
pub struct SearchWikiParams {
  pub q: String,             // 搜索关键词
}

/// 搜索结果
#[derive(Debug, Serialize)]
pub struct SearchResult {
  pub file_path: String,      // 文件路径
  pub title: String,          // 文件标题
  pub matches: Vec<SearchMatch>, // 匹配项
}

/// 搜索匹配项
#[derive(Debug, Serialize)]
pub struct SearchMatch {
  pub line: u32,             // 行号
  pub text: String,          // 匹配的文本片段
}

/// 主题信息
#[derive(Debug, Serialize, Clone)]
pub struct ThemeInfo {
  pub name: String,          // 主题名称（文件名，不含扩展名）
  pub display_name: String,  // 显示名称
  pub description: Option<String>, // 主题描述
  pub file_path: String,     // 主题文件路径（相对路径）
}

/// 主题列表
#[derive(Debug, Serialize)]
pub struct ThemeList {
  pub themes: Vec<ThemeInfo>,
  pub current: Option<String>, // 当前使用的主题名称
}


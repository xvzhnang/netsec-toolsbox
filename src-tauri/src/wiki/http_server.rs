// HTTP 服务器实现（使用 axum）
use axum::{
  extract::{Path, Query},
  http::StatusCode,
  response::{Html, IntoResponse, Response},
  routing::get,
  Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use crate::wiki::parser::MarkdownParser;
use crate::wiki::types::*;

/// 启动 HTTP 服务器
pub async fn start_http_server(
  wiki_dir: PathBuf,
  port: u16,
  _is_running: Arc<tokio::sync::Mutex<bool>>,
) {
  let parser = Arc::new(MarkdownParser::new());
  let wiki_dir_arc = Arc::new(wiki_dir);
  
  // 创建路由
  let app = Router::new()
    .route("/", get(index_handler))
    .route("/api/files", get(list_files_handler))
    .route("/api/render", get(render_handler))
    .route("/api/tree", get(tree_handler))
    .route("/api/search", get(search_handler))
    .route("/api/themes", get(themes_handler))
    .route("/file/*path", get(file_handler))
    .nest_service("/static", ServeDir::new(&*wiki_dir_arc))
    .layer(CorsLayer::permissive())
    .with_state(AppState {
      wiki_dir: wiki_dir_arc.clone(),
      parser: parser.clone(),
      current_theme: Arc::new(TokioMutex::new(None)),
    });
  
  let addr = SocketAddr::from(([127, 0, 0, 1], port));
  log::info!("Wiki HTTP 服务器启动在 http://{}", addr);
  
  let listener = tokio::net::TcpListener::bind(&addr).await
    .expect("无法绑定端口");
  
  axum::serve(listener, app)
    .await
    .expect("服务器启动失败");
}

/// 应用状态
#[derive(Clone)]
struct AppState {
  wiki_dir: Arc<PathBuf>,
  parser: Arc<MarkdownParser>,
  current_theme: Arc<TokioMutex<Option<String>>>, // 当前选择的主题
}

/// 首页处理器
async fn index_handler(
  Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
  // 获取主题参数并传递给前端
  let theme_name = params.get("theme").cloned().unwrap_or_else(|| "default".to_string());
  let mut html = include_str!("../../static/wiki_index.html").to_string();
  
  // 在 HTML 中注入主题信息
  html = html.replace(
    "</head>",
    &format!(
      r#"<script>
        window.WIKI_THEME = "{}";
        window.WIKI_AVAILABLE_THEMES = {:?};
      </script></head>"#,
      theme_name,
      get_available_themes()
    )
  );
  
  Html(html)
}

/// 文件列表处理器
async fn list_files_handler(
  axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
  match crate::wiki::server::list_wiki_files(&state.wiki_dir, &state.wiki_dir) {
    Ok(files) => (StatusCode::OK, axum::Json(files)).into_response(),
    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
  }
}

/// 渲染处理器
async fn render_handler(
  axum::extract::State(state): axum::extract::State<AppState>,
  Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
  if let Some(file_path) = params.get("path") {
    let full_path = state.wiki_dir.join(file_path);
    match state.parser.render_file(&full_path) {
      Ok(mut result) => {
        // 如果请求了主题，包装 HTML
        if let Some(theme) = params.get("theme") {
          let wrapped_html = wrap_wiki_html_with_theme(&result.html, &result.title, &result.toc, Some(theme));
          result.html = wrapped_html;
        }
        (StatusCode::OK, axum::Json(result)).into_response()
      }
      Err(e) => (StatusCode::NOT_FOUND, e).into_response(),
    }
  } else {
    (StatusCode::BAD_REQUEST, "缺少 path 参数").into_response()
  }
}

/// 目录树处理器
async fn tree_handler(
  axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
  match crate::wiki::server::list_wiki_files(&state.wiki_dir, &state.wiki_dir) {
    Ok(files) => {
      let tree = WikiTree { files };
      (StatusCode::OK, axum::Json(tree)).into_response()
    }
    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
  }
}

/// 搜索处理器
async fn search_handler(
  axum::extract::State(state): axum::extract::State<AppState>,
  Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
  if let Some(query) = params.get("q") {
    match crate::wiki::server::search_wiki_files(&state.wiki_dir, query) {
      Ok(results) => (StatusCode::OK, axum::Json(results)).into_response(),
      Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
  } else {
    (StatusCode::BAD_REQUEST, "缺少 q 参数").into_response()
  }
}

/// 主题列表处理器
async fn themes_handler() -> impl IntoResponse {
  let themes = get_available_themes();
  (StatusCode::OK, axum::Json(themes)).into_response()
}

/// 获取可用主题列表（内部函数，供 http_server 使用）
fn get_available_themes() -> Vec<String> {
  use crate::utils::get_wiki_dir;
  let wiki_dir = get_wiki_dir();
  let theme_dir = wiki_dir.join("theme");
  
  if !theme_dir.exists() {
    return vec!["default".to_string()];
  }
  
  let mut themes = Vec::new();
  if let Ok(entries) = std::fs::read_dir(&theme_dir) {
    for entry in entries {
      if let Ok(entry) = entry {
        let path = entry.path();
        if path.is_file() {
          if let Some(ext) = path.extension() {
            if ext == "css" {
              if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                themes.push(name.to_string());
              }
            }
          }
        }
      }
    }
  }
  
  themes.sort();
  if themes.is_empty() {
    themes.push("default".to_string());
  }
  
  themes
}

/// 加载自定义主题（从 wiki/theme 目录加载）
fn load_custom_theme(theme_name: Option<&str>) -> Result<String, std::io::Error> {
  use crate::utils::get_wiki_dir;
  let wiki_dir = get_wiki_dir();
  let theme_dir = wiki_dir.join("theme");
  
  // 如果没有指定主题，尝试加载 default.css
  let theme_file = if let Some(name) = theme_name {
    if name.ends_with(".css") {
      name.to_string()
    } else {
      format!("{}.css", name)
    }
  } else {
    // 尝试从配置文件读取主题设置，如果没有则使用 default
    if let Ok(config) = std::fs::read_to_string(wiki_dir.join("theme_config.json")) {
      if let Ok(json) = serde_json::from_str::<serde_json::Value>(&config) {
        if let Some(theme) = json.get("theme").and_then(|t| t.as_str()) {
          format!("{}.css", theme)
        } else {
          "default.css".to_string()
        }
      } else {
        "default.css".to_string()
      }
    } else {
      "default.css".to_string()
    }
  };
  
  let theme_path = theme_dir.join(&theme_file);
  if theme_path.exists() {
    std::fs::read_to_string(theme_path)
  } else {
    // 如果指定的主题不存在，尝试加载 default.css
    let default_path = theme_dir.join("default.css");
    if default_path.exists() {
      std::fs::read_to_string(default_path)
    } else {
      Ok(String::new())
    }
  }
}

/// 包装 Wiki HTML（添加样式和导航）- 旧版本，已废弃
#[allow(dead_code)]
fn wrap_wiki_html_old(html: &str, title: &str, toc: &Option<Vec<TocItem>>) -> String {
  wrap_wiki_html_with_theme(html, title, toc, None)
}

/// 包装 Wiki HTML（添加样式和导航）
fn wrap_wiki_html_with_theme(html: &str, title: &str, toc: &Option<Vec<TocItem>>, theme_name: Option<&str>) -> String {
  let toc_html = if let Some(toc) = toc {
    render_toc(toc)
  } else {
    String::new()
  };
  
  // 加载自定义主题
  let theme_css = load_custom_theme(theme_name).unwrap_or_default();
  
  // 获取可用主题列表用于主题选择器
  let available_themes = get_available_themes();
  let theme_selector = if available_themes.len() > 1 {
    let mut options = String::new();
    for theme in &available_themes {
      let selected = if theme_name == Some(theme.as_str()) || (theme_name.is_none() && theme == "default") {
        " selected"
      } else {
        ""
      };
      options.push_str(&format!("<option value=\"{}\"{}>{}</option>", theme, selected, theme));
    }
    format!(
      r#"<div class="wiki-theme-selector">
        <label for="theme-select">主题：</label>
        <select id="theme-select" onchange="changeTheme(this.value)">
          {}
        </select>
      </div>"#,
      options
    )
  } else {
    String::new()
  };
  
  // 获取文件树用于侧边栏导航
  let file_tree_html = get_file_tree_html();
  
  format!(
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{}</title>
  <style>{}</style>
  {}
</head>
<body>
  <div class="wiki-container">
    <aside class="wiki-sidebar">
      <div class="wiki-sidebar-header">
        <h2><a href="/" style="color: inherit; text-decoration: none;">Wiki</a></h2>
        {}
        <button class="wiki-search-btn" onclick="toggleSearch()">🔍 搜索</button>
      </div>
      <div id="wiki-search" class="wiki-search" style="display: none;">
        <input type="text" id="search-input" placeholder="搜索 Wiki..." onkeyup="performSearch(event)">
        <div id="search-results"></div>
      </div>
      <div class="wiki-file-tree">
        <h3>文件导航</h3>
        {}
      </div>
      <div class="wiki-toc-section">
        <h3>页面目录</h3>
        {}
      </div>
    </aside>
    <main class="wiki-content">
      <article class="markdown-body">
        {}
      </article>
    </main>
  </div>
  <script>
    function toggleSearch() {{
      const search = document.getElementById('wiki-search');
      search.style.display = search.style.display === 'none' ? 'block' : 'none';
      if (search.style.display === 'block') {{
        document.getElementById('search-input').focus();
      }}
    }}
    
    async function performSearch(event) {{
      if (event.key === 'Enter' || event.keyCode === 13) {{
        const query = event.target.value;
        if (!query.trim()) {{
          document.getElementById('search-results').innerHTML = '';
          return;
        }}
        try {{
          const response = await fetch(`/api/search?q=${{encodeURIComponent(query)}}`);
          const results = await response.json();
          let html = '<ul class="search-results-list">';
          for (const result of results) {{
            html += `<li><a href="/file/${{result.file_path}}">${{result.title}}</a></li>`;
          }}
          html += '</ul>';
          document.getElementById('search-results').innerHTML = html;
        }} catch (error) {{
          document.getElementById('search-results').innerHTML = '<p>搜索失败</p>';
        }}
      }}
    }}
    
    // 为所有标题添加锚点
    document.querySelectorAll('h1, h2, h3, h4, h5, h6').forEach((heading, index) => {{
      const id = heading.textContent?.toLowerCase().replace(/[^a-z0-9]+/g, '-') || `heading-${{index}}`;
      heading.id = id;
    }});
    
    // 主题切换功能
    function changeTheme(themeName) {{
      const url = new URL(window.location.href);
      if (themeName && themeName !== 'default') {{
        url.searchParams.set('theme', themeName);
      }} else {{
        url.searchParams.delete('theme');
      }}
      window.location.href = url.toString();
    }}
    
    // 加载主题列表
    async function loadThemes() {{
      try {{
        const response = await fetch('/api/themes');
        const themes = await response.json();
        const themeSelect = document.getElementById('theme-select');
        if (themeSelect) {{
          themeSelect.innerHTML = '';
          const urlParams = new URLSearchParams(window.location.search);
          const currentTheme = urlParams.get('theme') || 'default';
          for (const theme of themes) {{
            const option = document.createElement('option');
            option.value = theme;
            option.textContent = theme === 'default' ? '默认主题' : theme.replace(/_/g, ' ').replace(/-/g, ' ');
            if (theme === currentTheme) {{
              option.selected = true;
            }}
            themeSelect.appendChild(option);
          }}
        }}
      }} catch (error) {{
        console.error('加载主题列表失败:', error);
      }}
    }}
    
    // 页面加载时加载主题列表和恢复主题选择
    document.addEventListener('DOMContentLoaded', function() {{
      loadThemes();
      
      // 从 localStorage 恢复主题选择
      const savedTheme = localStorage.getItem('wiki-theme');
      if (savedTheme) {{
        const urlParams = new URLSearchParams(window.location.search);
        if (!urlParams.has('theme')) {{
          changeTheme(savedTheme);
        }}
      }}
      
      // 保存主题选择
      const themeSelect = document.getElementById('theme-select');
      if (themeSelect) {{
        themeSelect.addEventListener('change', function() {{
          localStorage.setItem('wiki-theme', this.value);
        }});
      }}
    }});
  </script>
</body>
</html>"#,
    title,
    get_wiki_styles(),
    if theme_css.is_empty() { String::new() } else { format!("<style>{}</style>", theme_css) },
    theme_selector,
    file_tree_html,
    toc_html,
    html
  )
}

/// 获取 Wiki 样式 CSS
fn get_wiki_styles() -> &'static str {
  include_str!("../../static/wiki_styles.css")
}

/// 获取文件树 HTML（用于侧边栏导航）
fn get_file_tree_html() -> String {
  use crate::utils::get_wiki_dir;
  let wiki_dir = get_wiki_dir();
  match crate::wiki::server::list_wiki_files(&wiki_dir, &wiki_dir) {
    Ok(files) => {
      let mut html = String::from("<nav class=\"wiki-file-tree\"><ul>");
      render_file_tree_items(&mut html, &files, 0);
      html.push_str("</ul></nav>");
      html
    }
    Err(_) => String::from("<nav class=\"wiki-file-tree\"><p>无法加载文件列表</p></nav>"),
  }
}

/// 递归渲染文件树项
fn render_file_tree_items(html: &mut String, files: &[crate::wiki::types::WikiFileInfo], level: usize) {
  for file in files {
    if file.is_dir {
      html.push_str(&format!(
        "<li class=\"wiki-tree-dir\" style=\"padding-left: {}px;\"><span>📁 {}</span>",
        level * 16,
        file.name
      ));
      if let Some(children) = &file.children {
        html.push_str("<ul>");
        render_file_tree_items(html, children, level + 1);
        html.push_str("</ul>");
      }
      html.push_str("</li>");
    } else {
      html.push_str(&format!(
        "<li class=\"wiki-tree-file\" style=\"padding-left: {}px;\"><a href=\"/file/{}\">{}</a></li>",
        level * 16,
        file.path,
        file.title
      ));
    }
  }
}

/// 渲染目录
fn render_toc(toc: &[TocItem]) -> String {
  if toc.is_empty() {
    return String::from("<nav class=\"wiki-toc\"><p>暂无目录</p></nav>");
  }
  
  let mut html = String::from("<nav class=\"wiki-toc\"><ul>");
  render_toc_items(&mut html, toc);
  html.push_str("</ul></nav>");
  html
}

/// 递归渲染目录项
fn render_toc_items(html: &mut String, items: &[TocItem]) {
  for item in items {
    html.push_str(&format!(
      "<li><a href=\"#{}\">{}</a>",
      item.id, item.text
    ));
    if !item.children.is_empty() {
      html.push_str("<ul>");
      render_toc_items(html, &item.children);
      html.push_str("</ul>");
    }
    html.push_str("</li>");
  }
}

/// 文件处理器（直接返回文件内容）
async fn file_handler(
  axum::extract::State(state): axum::extract::State<AppState>,
  Path(path): Path<String>,
  Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
  let full_path = state.wiki_dir.join(&path);
  
  if !full_path.exists() {
    return (StatusCode::NOT_FOUND, "文件不存在").into_response();
  }
  
  // 如果是 Markdown 文件，渲染为 HTML
  if path.ends_with(".md") {
    match state.parser.render_file(&full_path) {
      Ok(result) => {
        // 从查询参数获取主题名称
        let theme_name = params.get("theme");
        let html = wrap_wiki_html_with_theme(&result.html, &result.title, &result.toc, theme_name.map(|s| s.as_str()));
        Html(html).into_response()
      }
      Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
  } else {
    // 其他文件直接返回
    match std::fs::read(&full_path) {
      Ok(content) => {
        let content_type = if path.ends_with(".css") {
          "text/css"
        } else if path.ends_with(".js") {
          "application/javascript"
        } else if path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg") {
          "image/*"
        } else {
          "text/plain"
        };
        let body: axum::body::Body = content.into();
        Response::builder()
          .status(StatusCode::OK)
          .header("Content-Type", content_type)
          .body(body)
          .unwrap()
          .into_response()
      }
      Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "读取文件失败").into_response(),
    }
  }
}



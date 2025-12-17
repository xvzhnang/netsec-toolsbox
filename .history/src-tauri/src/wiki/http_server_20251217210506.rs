// HTTP 服务器实现（使用 axum）
use axum::{
  extract::{Path, Query},
  http::StatusCode,
  response::{Html, IntoResponse, Response},
  routing::{get, post},
  Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use crate::wiki::parser::MarkdownParser;
use crate::wiki::server::WikiServer;
use crate::wiki::types::*;

/// 启动 HTTP 服务器
pub async fn start_http_server(
  wiki_dir: PathBuf,
  port: u16,
  is_running: Arc<Mutex<bool>>,
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
    .route("/file/*path", get(file_handler))
    .nest_service("/static", ServeDir::new(&*wiki_dir_arc))
    .layer(CorsLayer::permissive())
    .with_state(AppState {
      wiki_dir: wiki_dir_arc.clone(),
      parser: parser.clone(),
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
}

/// 首页处理器
async fn index_handler() -> impl IntoResponse {
  Html(include_str!("../../static/wiki_index.html"))
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
      Ok(result) => (StatusCode::OK, axum::Json(result)).into_response(),
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

/// 文件处理器（直接返回文件内容）
async fn file_handler(
  axum::extract::State(state): axum::extract::State<AppState>,
  Path(path): Path<String>,
) -> impl IntoResponse {
  let full_path = state.wiki_dir.join(&path);
  
  if !full_path.exists() {
    return (StatusCode::NOT_FOUND, "文件不存在").into_response();
  }
  
  // 如果是 Markdown 文件，渲染为 HTML
  if path.ends_with(".md") {
    match state.parser.render_file(&full_path) {
      Ok(result) => {
        let html = wrap_wiki_html(&result.html, &result.title, &result.toc);
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
        Response::builder()
          .status(StatusCode::OK)
          .header("Content-Type", content_type)
          .body(content.into())
          .unwrap()
          .into_response()
      }
      Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "读取文件失败").into_response(),
    }
  }
}

/// 包装 Wiki HTML（添加样式和导航）
fn wrap_wiki_html(html: &str, title: &str, toc: &Option<Vec<TocItem>>) -> String {
  let toc_html = if let Some(toc) = toc {
    render_toc(toc)
  } else {
    String::new()
  };
  
  format!(
    r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{}</title>
  <style>{}</style>
</head>
<body>
  <div class="wiki-container">
    <aside class="wiki-sidebar">
      {}
    </aside>
    <main class="wiki-content">
      <article class="markdown-body">
        {}
      </article>
    </main>
  </div>
</body>
</html>"#,
    title,
    get_wiki_styles(),
    toc_html,
    html
  )
}

/// 获取 Wiki 样式 CSS
fn get_wiki_styles() -> &'static str {
  include_str!("../../static/wiki_styles.css")
}

/// 渲染目录
fn render_toc(toc: &[TocItem]) -> String {
  let mut html = String::from("<nav class=\"wiki-toc\"><ul>");
  for item in toc {
    html.push_str(&format!(
      "<li><a href=\"#{}\">{}</a>",
      item.id, item.text
    ));
    if !item.children.is_empty() {
      html.push_str("<ul>");
      for child in &item.children {
        html.push_str(&format!(
          "<li><a href=\"#{}\">{}</a></li>",
          child.id, child.text
        ));
      }
      html.push_str("</ul>");
    }
    html.push_str("</li>");
  }
  html.push_str("</ul></nav>");
  html
}



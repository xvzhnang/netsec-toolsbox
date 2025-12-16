use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
struct CategoriesConfig {
  categories: Vec<CategoryConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CategoryConfig {
  id: String,
  name: String,
  label: Option<String>,
  description: Option<String>,
  icon: String,
  color: String,
  order: i32,
  enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CategoriesData {
  categories: Vec<CategoryPageData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CategoryPageData {
  id: String,
  name: String,
  label: Option<String>,
  description: Option<String>,
  sub_categories: Vec<SubCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SubCategory {
  id: String,
  name: String,
  description: Option<String>,
  tools: Vec<ToolItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolItem {
  id: String,
  name: String,
  description: Option<String>,
  icon_emoji: Option<String>,
  wiki_url: Option<String>,
  tool_type: Option<String>,
  exec_path: Option<String>,
  args: Option<Vec<String>>,
  working_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JarConfig {
  jar_path: String,
  java_path: Option<String>,
  jvm_args: Option<Vec<String>>,
  program_args: Option<Vec<String>>,
}

fn get_config_path(app: &tauri::AppHandle) -> PathBuf {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .expect("failed to get app data dir");
  std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
  app_data_dir.join("categories.json")
}

#[tauri::command]
fn read_categories_config(app: tauri::AppHandle) -> Result<String, String> {
  let config_path = get_config_path(&app);
  if config_path.exists() {
    fs::read_to_string(&config_path)
      .map_err(|e| format!("Failed to read config file: {}", e))
  } else {
    // 返回空JSON，前端会使用默认值
    Ok("{}".to_string())
  }
}

#[tauri::command]
fn write_categories_config(app: tauri::AppHandle, content: String) -> Result<(), String> {
  let config_path = get_config_path(&app);
  fs::write(&config_path, content)
    .map_err(|e| format!("Failed to write config file: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      read_categories_config,
      write_categories_config
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

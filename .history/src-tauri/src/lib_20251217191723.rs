// 模块声明
mod types;
mod utils;
mod config;
mod launcher;
mod icon_extractor;
mod file_ops;

// 重新导出公共类型和函数
pub use types::*;
pub use config::*;
pub use launcher::*;
pub use icon_extractor::*;
pub use file_ops::*;

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
      
      // 设置窗口默认全屏，但允许用户调整
      // 在 Tauri 2.x 中，主窗口通常名为 "main" 或使用默认窗口
      if let Some(window) = app.get_window("main") {
        let _ = window.set_fullscreen(true);
      } else if let Some(window) = app.get_webview_windows().values().next() {
        // 如果没有找到 "main" 窗口，尝试获取第一个窗口
        let _ = window.set_fullscreen(true);
      }
      
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      // 配置管理
      read_categories_config,
      write_categories_config,
      read_config_file,
      write_config_file,
      get_config_file_path,
      config_file_exists,
      // 工具启动
      launch_tool,
      // 图标提取
      extract_icon_from_file,
      fetch_favicon,
      // 文件操作
      upload_file,
      resolve_file_path,
      open_file_dialog
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

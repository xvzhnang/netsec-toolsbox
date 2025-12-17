// 模块声明
mod types;
mod utils;
mod config;
mod launcher;
mod icon_extractor;
mod file_ops;
mod wiki;

// Wiki 命令
mod wiki_commands {
  pub use crate::wiki::commands::*;
}

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
      
      // 窗口全屏设置已在 tauri.conf.json 中配置
      // fullscreen: true 和 resizable: true 允许用户自行调整窗口大小
      
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
      open_url_in_browser,
      // 图标提取
      extract_icon_from_file,
      fetch_favicon,
      // 文件操作
      upload_file,
      resolve_file_path,
      open_file_dialog,
      // Wiki 功能
      wiki_commands::start_wiki_server,
      wiki_commands::stop_wiki_server,
      wiki_commands::get_wiki_files,
      wiki_commands::render_wiki_file,
      wiki_commands::search_wiki,
      wiki_commands::get_wiki_dir,
      wiki_commands::find_wiki_for_tool,
      wiki_commands::get_wiki_themes,
      wiki_commands::set_wiki_theme,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

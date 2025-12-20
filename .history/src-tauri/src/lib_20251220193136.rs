// 模块声明
mod types;
mod utils;
mod config;
mod launcher;
mod icon_extractor;
mod file_ops;
mod wiki;
mod ai_service;

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
      read_icon_file,
      // 工具启动
      launch_tool,
      open_url_in_browser,
      // 图标提取
      extract_icon_from_file,
      fetch_favicon,
      save_icon_to_cache,
      // 文件操作
      upload_file,
      resolve_file_path,
      open_file_dialog,
      // Wiki 功能
        wiki_commands::get_wiki_files,
        wiki_commands::read_wiki_file,
        wiki_commands::search_wiki,
        wiki_commands::get_wiki_dir,
        wiki_commands::find_wiki_for_tool,
      // AI Gateway 服务（旧版，保持兼容）
        ai_service::legacy::start_ai_service,
        ai_service::legacy::stop_ai_service,
        ai_service::legacy::check_ai_service_status,
        ai_service::legacy::read_models_config,
        ai_service::legacy::write_models_config,
      // AI Gateway 连接池（新版）
        ai_service::init_gateway_pool,
        ai_service::start_gateway_pool,
        ai_service::stop_gateway_pool,
        ai_service::forward_ai_request,
        ai_service::get_gateway_pool_status,
    ])
    .manage(ai_service::legacy::AIServiceState::default())
    .manage(ai_service::AIServicePoolState::default())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

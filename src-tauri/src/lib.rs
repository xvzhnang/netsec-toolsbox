// 模块声明
mod ai_service;
mod config;
mod file_ops;
mod icon_extractor;
mod launcher;
mod service;
mod types;
mod utils;
mod wiki;

// Wiki 命令
mod wiki_commands {
    pub use crate::wiki::commands::*;
}

// 重新导出公共类型和函数
pub use config::*;
pub use file_ops::*;
pub use icon_extractor::*;
pub use launcher::*;
pub use types::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 创建 ServiceManager 并注册服务
    let service_manager = std::sync::Mutex::new(service::ServiceManager::new());

    // 注册 AI Gateway 服务
    {
        // 处理 Mutex 锁定失败的情况，避免应用启动时崩溃
        // 如果 Mutex 被污染（poisoned），尝试从错误中恢复
        let manager = match service_manager.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::error!("ServiceManager Mutex 被污染: {}", poisoned);
                // 尝试从被污染的 Mutex 中恢复数据
                // 注意：在应用启动阶段，Mutex 不应该被污染，但如果发生了，至少应用可以启动
                poisoned.into_inner()
            }
        };
        let ai_gateway_service = std::sync::Arc::new(std::sync::Mutex::new(
            ai_service::GatewayPoolService::new("ai-gateway".to_string(), "AI Gateway".to_string()),
        ));

        if let Err(e) = manager.register(ai_gateway_service) {
            log::warn!("注册 AI Gateway 服务失败: {}", e);
        } else {
            log::info!("AI Gateway 服务已注册到 ServiceManager");
        }

        // 启动监控线程
        manager.start_monitoring();
    }

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
            ai_service::diagnose_worker,
            // 统一服务管理（新架构）
            service::get_all_services,
            service::get_service_status,
            service::start_service,
            service::stop_service,
            service::restart_service,
            service::get_prometheus_metrics,
            service::get_service_metrics,
        ])
        .manage(ai_service::legacy::AIServiceState::default())
        .manage(ai_service::AIServicePoolState::default())
        .manage(service_manager)
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            log::error!("error while running tauri application: {}", e);
        });
}

// 模块声明
mod ai_history;
mod ai_service;
mod config;
mod file_ops;
mod icon_extractor;
mod launcher;
mod service;
mod types;
mod utils;
mod docs_server;
mod mkdocs_server;
mod splashscreen;

// 重新导出公共类型和函数
pub use config::*;
pub use file_ops::*;
pub use icon_extractor::*;
pub use launcher::*;
pub use types::*;

use tauri::{Emitter, Manager};

struct ServiceEventForwarder {
    app: tauri::AppHandle,
}

impl service::events::EventListener for ServiceEventForwarder {
    fn on_event(&self, event: &service::events::ServiceEvent) {
        if let Err(e) = self.app.emit("service_event", event) {
            log::warn!("[ServiceEventForwarder] emit 失败: {}", e);
        }
    }
}

#[derive(serde::Serialize)]
struct ModuleStatus {
    ai: bool,
    wiki: bool,
}

#[tauri::command]
async fn get_module_status(
    ai_state: tauri::State<'_, service::ServiceManager>,
    wiki_state: tauri::State<'_, mkdocs_server::MkDocsServerState>,
) -> Result<ModuleStatus, String> {
    let ai_status = ai_state.get_status("ai-gateway").map(|s| s.is_available).unwrap_or(false);
    let wiki_running = *wiki_state.is_running.lock().map_err(|e| e.to_string())?;
    Ok(ModuleStatus {
        ai: ai_status,
        wiki: wiki_running,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ✅ 工程原则：使用可超时锁（parking_lot）
    // 创建 ServiceManager 并注册服务
    let service_manager = service::ServiceManager::new();

    // ✅ 关键优化：延迟注册 AI Gateway 服务，避免在启动时阻塞
    // 将服务注册移到 setup 中异步执行，避免阻塞主线程

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(ai_service::AIServicePoolState::default())
        .manage(service_manager)
        .manage(docs_server::DocsServerState::default())
        .manage(mkdocs_server::MkDocsServerState::default())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let _ = splashscreen::show_splashscreen(&app.handle());

            // ✅ 关键优化：快速初始化，避免长时间持有锁
            let manager = app.state::<service::ServiceManager>();

            // 快速获取 event_bus 并订阅事件
            let event_bus = manager.event_bus();

            // 快速订阅事件（快速释放锁）
            {
                let mut bus_guard =
                    crate::utils::lock_or_recover(event_bus.as_ref(), "ServiceManager.event_bus");
                bus_guard.subscribe(Box::new(ServiceEventForwarder {
                    app: app.handle().clone(),
                }));
                drop(bus_guard); // 立即释放锁
            }

            // ✅ 关键修复：必须在启动前同步注册服务，避免竞态条件
            // 但 GatewayPoolService::new() 本身很快（只是创建结构体），不会阻塞
            // 真正的文件系统操作在 start_all() 时才执行，那时已经在后台线程中
            let manager_state = app.state::<service::ServiceManager>();
            {
                // GatewayPoolService::new() 只是创建结构体，get_global_pool() 使用 OnceLock 缓存
                // 首次调用时创建 GatewayPool，但 GatewayPool::new() 只是创建 Vec 和结构体，不涉及文件系统
                let ai_gateway_service = std::sync::Arc::new(std::sync::Mutex::new(
                    ai_service::GatewayPoolService::new(
                        "ai-gateway".to_string(),
                        "AI Gateway".to_string(),
                    ),
                ));

                if let Err(e) = manager_state.register(ai_gateway_service) {
                    log::warn!("注册 AI Gateway 服务失败: {}", e);
                } else {
                    log::info!("AI Gateway 服务已注册到 ServiceManager");
                }
            }

            // 窗口全屏设置已在 tauri.conf.json 中配置
            // fullscreen: true 和 resizable: true 允许用户自行调整窗口大小

            // ✅ 3️⃣ 关键保护：前端只在「一次性入口」启动 AI
            // 使用静态变量确保只启动一次，避免重复启动
            static AI_GATEWAY_STARTED: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);

            // 优化启动时间：在后台异步启动服务，不阻塞 UI 渲染
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                // 关键优化：减少延迟到 200ms，加快启动速度
                // 使用 catch_unwind 捕获 panic，避免启动线程崩溃导致应用无法启动
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // 短暂延迟，确保 UI 先渲染
                    std::thread::sleep(std::time::Duration::from_millis(200));

                    // ✅ 3️⃣ 关键保护：检查是否已经启动过，避免重复启动
                    if AI_GATEWAY_STARTED
                        .compare_exchange(
                            false,
                            true,
                            std::sync::atomic::Ordering::SeqCst,
                            std::sync::atomic::Ordering::SeqCst,
                        )
                        .is_err()
                    {
                        log::info!("[启动保护] AI Gateway 服务已在应用启动时启动过，跳过重复启动");
                        // 即使跳过启动，也要启动监控线程
                        let manager_state = app_handle.state::<service::ServiceManager>();
                        manager_state.start_monitoring();
                        return;
                    }

                    let manager_state = app_handle.state::<service::ServiceManager>();

                    // ✅ 1️⃣ 关键保护：ServiceManager 启动幂等性检查（已在 start_service 中实现）
                    if let Err(e) = manager_state.start_service("ai-gateway") {
                        log::warn!("[启动优化] 后台启动 AI Gateway 服务失败: {}", e);
                        // 启动失败时重置标志，允许下次重试
                        AI_GATEWAY_STARTED.store(false, std::sync::atomic::Ordering::SeqCst);
                    } else {
                        log::info!("[启动优化] AI Gateway 服务已在后台启动（一次性启动保护）");
                        // 发送模块状态更新事件
                        app_handle.emit("module_status_changed", ()).ok();
                    }

                    // ✅ 启动 Wiki 服务 (MkDocs)
                    let mkdocs_state = app_handle.state::<mkdocs_server::MkDocsServerState>();
                    if let Err(e) = mkdocs_server::init_mkdocs(&mkdocs_state) {
                        log::warn!("[启动优化] 后台启动 Wiki 服务失败: {}", e);
                    } else {
                        log::info!("[启动优化] Wiki 服务已在后台启动");
                        // 发送模块状态更新事件
                        app_handle.emit("module_status_changed", ()).ok();
                    }

                    // 启动监控线程
                    manager_state.start_monitoring();
                }));

                if let Err(panic_info) = result {
                    log::error!("[启动优化] 启动线程 panic: {:?}", panic_info);
                    // Panic 时重置标志，允许下次重试
                    AI_GATEWAY_STARTED.store(false, std::sync::atomic::Ordering::SeqCst);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Docsify Server
            docs_server::start_docs_server,
            // MkDocs Server
            mkdocs_server::start_mkdocs_server,
            mkdocs_server::stop_mkdocs_server,
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
            // AI 聊天历史（SQLite）
            ai_history::ai_history_load,
            ai_history::ai_history_get_session,
            ai_history::ai_history_upsert_session,
            ai_history::ai_history_add_message,
            ai_history::ai_history_update_message,
            ai_history::ai_history_delete_session,
            ai_history::ai_history_set_current_session,
            ai_history::ai_history_clear,
            // AI Gateway 配置（models.json）
            ai_service::legacy::read_models_config,
            ai_service::legacy::write_models_config,
            // AI Gateway 请求转发（连接池）
            ai_service::forward_ai_request,
            // 统一服务管理（新架构）
            service::get_all_services,
            service::get_service_status,
            service::start_service,
            service::stop_service,
            service::restart_service,
            service::get_prometheus_metrics,
            service::get_service_metrics,
            service::get_thread_heartbeat_status,
            // 模块状态
            get_module_status,
            splashscreen::close_splashscreen,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            log::error!("error while running tauri application: {}", e);
        });
}

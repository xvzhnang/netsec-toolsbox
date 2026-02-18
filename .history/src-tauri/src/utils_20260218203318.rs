use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Once;

// ✅ 工程原则：使用可超时锁（parking_lot）
pub mod channel_loop;
pub mod heartbeat;
pub mod lock;
pub mod mutex_compat;
pub mod net;
pub mod process;
pub use channel_loop::*;
pub use heartbeat::*;
pub use net::*;
pub use process::*;

// 缓存应用程序基础目录，避免重复计算和日志输出
static APP_BASE_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
static INIT_LOG: Once = Once::new();

// ✅ 工程原则：兼容旧代码的锁获取（带恢复机制）
// ⚠️ 注意：这是为了兼容现有代码，新代码应该使用 lock::try_lock_or_timeout
// 此函数仅用于 parking_lot::Mutex，对于 std::sync::Mutex 请使用 mutex_compat::lock_or_recover_std
pub fn lock_or_recover<'a, T: ?Sized>(
    mutex: &'a parking_lot::Mutex<T>,
    _name: &str,
) -> parking_lot::MutexGuard<'a, T> {
    // parking_lot::Mutex 不会 panic，所以这里直接 lock
    mutex.lock()
}

/// 兼容 std::sync::Mutex 的锁获取
pub fn lock_or_recover_std<'a, T: ?Sized>(
    mutex: &'a std::sync::Mutex<T>,
    name: &str,
) -> std::sync::MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::error!("{} Mutex 被污染，尝试恢复", name);
            poisoned.into_inner()
        }
    }
}

/// 获取应用程序基础目录（项目根目录，src-tauri 的父目录）
/// 用户可自定义的文件都放在这个目录下
/// 结果会被缓存，避免重复计算和日志输出
pub fn get_app_base_dir() -> PathBuf {
    APP_BASE_DIR
        .get_or_init(|| {
            // 只在第一次调用时打印日志
            INIT_LOG.call_once(|| {
                log::info!("[INIT] 初始化应用程序基础目录...");
            });

            // 获取可执行文件路径
            let exe_path = std::env::current_exe().unwrap_or_else(|e| {
                log::warn!("get_app_base_dir: 获取可执行文件路径失败: {}", e);
                std::env::current_dir().unwrap_or_else(|e| {
                    log::warn!("get_app_base_dir: 获取当前目录失败: {}", e);
                    PathBuf::from(".")
                })
            });

            log::debug!("get_app_base_dir: 可执行文件路径: {}", exe_path.display());

            // 从可执行文件路径向上查找 src-tauri 目录
            let mut current = exe_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| {
                    log::warn!(
                        "get_app_base_dir: 可执行文件路径没有 parent: {}",
                        exe_path.display()
                    );
                    exe_path.clone()
                });

            log::debug!(
                "get_app_base_dir: 开始从 {} 向上查找 src-tauri 目录",
                current.display()
            );

            // ✅ 关键优化：限制向上查找的深度，避免在深层目录结构中无限查找
            // 向上查找，直到找到 src-tauri 目录或到达根目录
            const MAX_DEPTH: usize = 10; // 限制最大查找深度，避免在深层目录中卡顿
            let mut depth = 0;
            let result =
                loop {
                    // 检查深度限制
                    if depth >= MAX_DEPTH {
                        log::warn!(
                            "get_app_base_dir: 达到最大查找深度 {}，使用可执行文件目录",
                            MAX_DEPTH
                        );
                        let fallback_dir = exe_path
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| {
                                log::warn!(
                                    "get_app_base_dir: 可执行文件路径没有 parent，降级到当前目录"
                                );
                                PathBuf::from(".")
                            });
                        break fallback_dir;
                    }

                    let src_tauri_path = current.join("src-tauri");
                    // ✅ 关键优化：使用 metadata() 一次性检查，避免多次文件系统调用
                    if let Ok(metadata) = src_tauri_path.metadata() {
                        if metadata.is_dir() {
                            // 找到 src-tauri 目录，返回其父目录（项目根目录）
                            log::info!(
                                "get_app_base_dir: 找到 src-tauri 目录: {}, 项目根目录: {}",
                                src_tauri_path.display(),
                                current.display()
                            );
                            break current.to_path_buf();
                        }
                    }

                    // 如果已经到达根目录，停止查找
                    if let Some(parent) = current.parent() {
                        current = parent.to_path_buf();
                        depth += 1;
                    } else {
                        // 如果找不到 src-tauri 目录（可能是发布版本），降级到可执行文件所在目录
                        let fallback_dir = exe_path
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| {
                                log::warn!(
                                    "get_app_base_dir: 可执行文件路径没有 parent，降级到当前目录"
                                );
                                PathBuf::from(".")
                            });
                        log::warn!(
                            "get_app_base_dir: 未找到 src-tauri 目录，使用可执行文件目录: {}",
                            fallback_dir.display()
                        );
                        break fallback_dir;
                    }
                };

            result
        })
        .clone()
}

/// 获取配置目录路径（在项目根目录下的 .config 文件夹）
/// 开发时：项目根目录/.config
/// 发布时：如果找不到 src-tauri，则使用可执行文件目录/.config
pub fn get_config_dir() -> PathBuf {
    let base_dir = get_app_base_dir();
    let config_dir = base_dir.join(".config");

    log::debug!(
        "get_config_dir: 基础目录: {}, 配置目录: {}",
        base_dir.display(),
        config_dir.display()
    );

    // ✅ 关键优化：延迟创建目录，避免在启动时阻塞
    // 使用 metadata() 快速检查目录是否存在，如果不存在则在后台线程中创建
    let config_dir_exists = std::fs::metadata(&config_dir).is_ok();
    if !config_dir_exists {
        // 在后台线程中创建目录，避免阻塞
        let config_dir_clone = config_dir.clone();
        std::thread::spawn(move || {
            if let Err(e) = std::fs::create_dir_all(&config_dir_clone) {
                log::error!("get_config_dir: 创建配置目录失败: {}", e);
            }
        });
    }

    config_dir
}

/// 获取图标缓存目录路径
pub fn get_icons_dir() -> PathBuf {
    let icons_dir = get_config_dir().join("icons");
    // ✅ 关键优化：延迟创建目录，避免在启动时阻塞
    let icons_dir_exists = std::fs::metadata(&icons_dir).is_ok();
    if !icons_dir_exists {
        let icons_dir_clone = icons_dir.clone();
        std::thread::spawn(move || {
            if let Err(e) = std::fs::create_dir_all(&icons_dir_clone) {
                log::error!("get_icons_dir: 创建图标目录失败: {}", e);
            }
        });
    }
    icons_dir
}

/// 获取上传文件目录路径
pub fn get_uploads_dir() -> PathBuf {
    let uploads_dir = get_config_dir().join("uploads");
    // ✅ 关键优化：延迟创建目录，避免在启动时阻塞
    let uploads_dir_exists = std::fs::metadata(&uploads_dir).is_ok();
    if !uploads_dir_exists {
        let uploads_dir_clone = uploads_dir.clone();
        std::thread::spawn(move || {
            if let Err(e) = std::fs::create_dir_all(&uploads_dir_clone) {
                log::error!("get_uploads_dir: 创建上传目录失败: {}", e);
            }
        });
    }
    uploads_dir
}

/// 生成文件路径的哈希值（用于缓存文件名）
pub fn hash_path(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..16]) // 使用前16字节，32个十六进制字符
}

/// 从文件路径提取所在目录
pub fn get_file_dir(file_path: &str) -> PathBuf {
    let path = std::path::Path::new(file_path);
    if let Some(parent) = path.parent() {
        parent.to_path_buf()
    } else {
        PathBuf::from(".")
    }
}

/// 获取 Wiki 目录路径（在项目根目录下）
/// 开发时：项目根目录/wiki
/// 发布时：如果找不到 src-tauri，则使用可执行文件目录/wiki
pub fn get_wiki_dir() -> PathBuf {
    // 使用项目根目录下的 wiki 文件夹
    let base_dir = get_app_base_dir();
    let wiki_dir = base_dir.join("wiki");

    log::debug!(
        "get_wiki_dir: 基础目录: {}, Wiki 目录: {}",
        base_dir.display(),
        wiki_dir.display()
    );

    // ✅ 关键优化：延迟创建目录，避免在启动时阻塞
    let wiki_dir_exists = std::fs::metadata(&wiki_dir).is_ok();
    if !wiki_dir_exists {
        let wiki_dir_clone = wiki_dir.clone();
        std::thread::spawn(move || {
            if let Err(e) = std::fs::create_dir_all(&wiki_dir_clone) {
                log::error!("get_wiki_dir: 创建 Wiki 目录失败: {}", e);
            }
        });
    }

    wiki_dir
}

/// 获取 Wiki 文档目录路径（wiki 根目录，包含 tools/, notes/, labs/ 等）
pub fn get_docs_dir() -> PathBuf {
    let wiki_dir = get_wiki_dir();

    // ✅ 关键优化：延迟创建目录，避免在启动时阻塞
    let wiki_dir_clone = wiki_dir.clone();
    std::thread::spawn(move || {
        if let Err(e) = std::fs::create_dir_all(wiki_dir_clone.join("tools")) {
            log::error!("get_docs_dir: 创建 tools 目录失败: {}", e);
        }
        if let Err(e) = std::fs::create_dir_all(wiki_dir_clone.join("notes")) {
            log::error!("get_docs_dir: 创建 notes 目录失败: {}", e);
        }
        if let Err(e) = std::fs::create_dir_all(wiki_dir_clone.join("labs")) {
            log::error!("get_docs_dir: 创建 labs 目录失败: {}", e);
        }
    });

    wiki_dir
}

/// 获取 Wiki 主题目录路径（在 wiki 目录下的 themes 文件夹）
pub fn get_theme_dir() -> PathBuf {
    let wiki_dir = get_wiki_dir();
    let theme_dir = wiki_dir.join("themes");

    // 确保主题目录存在
    if let Err(e) = std::fs::create_dir_all(&theme_dir) {
        log::error!("get_theme_dir: 创建 themes 目录失败: {}", e);
    }

    theme_dir
}

/// 计算从 `from` 目录到 `to` 目录的相对路径
pub fn get_relative_path(from: &std::path::Path, to: &std::path::Path) -> PathBuf {
    use std::path::Component;

    let mut from_iter = from.components();
    let mut to_iter = to.components();
    let mut from_next = from_iter.next();
    let mut to_next = to_iter.next();

    // 找到共同的前缀
    let mut common_components = PathBuf::new();
    while let (Some(f), Some(t)) = (from_next, to_next) {
        if f == t {
            common_components.push(f);
            from_next = from_iter.next();
            to_next = to_iter.next();
        } else {
            break;
        }
    }

    // 计算回退的步数
    let mut result = PathBuf::new();
    if from_next.is_some() {
        result.push("..");
        for _ in from_iter {
            result.push("..");
        }
    }

    // 添加剩余的路径
    if let Some(t) = to_next {
        if let Component::Normal(s) = t {
            result.push(s);
        }
    }
    for component in to_iter {
        if let Component::Normal(s) = component {
            result.push(s);
        }
    }

    result
}

use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, Once};

// 缓存应用程序基础目录，避免重复计算和日志输出
static APP_BASE_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
static INIT_LOG: Once = Once::new();

pub fn lock_or_recover<'a, T: ?Sized>(mutex: &'a Mutex<T>, name: &str) -> MutexGuard<'a, T> {
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

            // 向上查找，直到找到 src-tauri 目录或到达根目录
            let result = loop {
                let src_tauri_path = current.join("src-tauri");
                if src_tauri_path.exists() && src_tauri_path.is_dir() {
                    // 找到 src-tauri 目录，返回其父目录（项目根目录）
                    log::info!(
                        "get_app_base_dir: 找到 src-tauri 目录: {}, 项目根目录: {}",
                        src_tauri_path.display(),
                        current.display()
                    );
                    break current.to_path_buf();
                }

                // 如果已经到达根目录，停止查找
                if let Some(parent) = current.parent() {
                    current = parent.to_path_buf();
                } else {
                    // 如果找不到 src-tauri 目录（可能是发布版本），降级到可执行文件所在目录
                    let fallback_dir =
                        exe_path
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

    // 确保配置目录存在
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        log::error!("get_config_dir: 创建配置目录失败: {}", e);
    }

    config_dir
}

/// 获取图标缓存目录路径
pub fn get_icons_dir() -> PathBuf {
    let icons_dir = get_config_dir().join("icons");
    if let Err(e) = std::fs::create_dir_all(&icons_dir) {
        log::error!("get_icons_dir: 创建图标目录失败: {}", e);
    }
    icons_dir
}

/// 获取上传文件目录路径
pub fn get_uploads_dir() -> PathBuf {
    let uploads_dir = get_config_dir().join("uploads");
    if let Err(e) = std::fs::create_dir_all(&uploads_dir) {
        log::error!("get_uploads_dir: 创建上传目录失败: {}", e);
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

    // 确保 wiki 目录存在
    if let Err(e) = std::fs::create_dir_all(&wiki_dir) {
        log::error!("get_wiki_dir: 创建 Wiki 目录失败: {}", e);
    }

    wiki_dir
}

/// 获取 Wiki 文档目录路径（wiki 根目录，包含 tools/, notes/, labs/ 等）
pub fn get_docs_dir() -> PathBuf {
    let wiki_dir = get_wiki_dir();

    // 确保 Wiki 目录和子目录存在
    if let Err(e) = std::fs::create_dir_all(&wiki_dir.join("tools")) {
        log::error!("get_docs_dir: 创建 tools 目录失败: {}", e);
    }
    if let Err(e) = std::fs::create_dir_all(&wiki_dir.join("notes")) {
        log::error!("get_docs_dir: 创建 notes 目录失败: {}", e);
    }
    if let Err(e) = std::fs::create_dir_all(&wiki_dir.join("labs")) {
        log::error!("get_docs_dir: 创建 labs 目录失败: {}", e);
    }

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

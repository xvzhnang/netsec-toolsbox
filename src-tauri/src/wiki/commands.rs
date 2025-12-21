// Wiki 相关的 Tauri 命令
use crate::wiki::server::WikiServer;

/// 获取 Wiki 文件列表
#[tauri::command]
pub fn get_wiki_files() -> Result<Vec<crate::wiki::types::WikiFileInfo>, String> {
    // 直接创建实例获取文件列表，不依赖服务器
    let server = WikiServer::new();
    server.list_files()
}

/// 读取 Wiki 文件内容（不渲染，返回原始文本）
/// 支持读取 Markdown 文件和主题 CSS 文件
#[tauri::command]
pub fn read_wiki_file(file_path: String) -> Result<String, String> {
    use crate::utils::{get_docs_dir, get_theme_dir};
    use std::fs;

    // 如果是主题文件，从 themes 目录读取
    if file_path.starts_with("themes/") {
        let theme_dir = get_theme_dir();
        let theme_name = file_path.strip_prefix("themes/").unwrap_or(&file_path);
        let full_path = theme_dir.join(theme_name);

        if !full_path.exists() {
            return Err(format!("主题文件不存在: {}", file_path));
        }

        if !full_path.is_file() {
            return Err(format!("路径不是文件: {}", file_path));
        }

        return fs::read_to_string(&full_path).map_err(|e| format!("读取主题文件失败: {}", e));
    }

    // 否则从 docs 目录读取
    let docs_dir = get_docs_dir();
    let full_path = docs_dir.join(&file_path);

    if !full_path.exists() {
        return Err(format!("Wiki 文件不存在: {}", file_path));
    }

    if !full_path.is_file() {
        return Err(format!("路径不是文件: {}", file_path));
    }

    fs::read_to_string(&full_path).map_err(|e| format!("读取文件失败: {}", e))
}

/// 搜索 Wiki
#[tauri::command]
pub fn search_wiki(query: String) -> Result<Vec<crate::wiki::types::SearchResult>, String> {
    // 直接创建实例搜索，不依赖服务器
    let server = WikiServer::new();
    server.search(&query)
}

/// 获取 Wiki 目录路径
#[tauri::command]
pub fn get_wiki_dir() -> Result<String, String> {
    let server = WikiServer::new();
    Ok(server.get_wiki_dir().to_string_lossy().to_string())
}

/// 获取可用主题列表
#[tauri::command]
#[allow(dead_code)]
pub fn get_wiki_themes() -> Result<Vec<String>, String> {
    use crate::utils::get_theme_dir;
    let theme_dir = get_theme_dir();

    if !theme_dir.exists() {
        // 如果主题目录不存在，创建它
        if let Err(e) = std::fs::create_dir_all(&theme_dir) {
            return Err(format!("创建主题目录失败: {}", e));
        }

        // 不自动创建默认主题文件，让用户自己添加 Typora 主题
        return Ok(vec!["default".to_string()]);
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

    Ok(themes)
}

/// 根据工具 ID 或名称查找对应的 Wiki 文件
#[tauri::command]
pub fn find_wiki_for_tool(
    tool_id: String,
    tool_name: Option<String>,
) -> Result<Option<String>, String> {
    let server = WikiServer::new();
    let files = server.list_files()?;

    // 尝试多种匹配方式：
    // 1. 精确匹配工具 ID
    // 2. 匹配工具名称（不区分大小写）
    // 3. 匹配文件名包含工具 ID 或名称

    let tool_name_lower = tool_name
        .as_ref()
        .map(|n| n.to_lowercase())
        .unwrap_or_default();
    let tool_id_lower = tool_id.to_lowercase();

    // 递归搜索文件
    fn search_files(
        files: &[crate::wiki::types::WikiFileInfo],
        tool_id: &str,
        tool_name: &str,
    ) -> Option<String> {
        for file in files {
            if file.is_dir {
                if let Some(children) = &file.children {
                    if let Some(found) = search_files(children, tool_id, tool_name) {
                        return Some(found);
                    }
                }
            } else {
                // 检查文件名或路径是否匹配
                let name_lower = file.name.to_lowercase();
                let path_lower = file.path.to_lowercase();
                let title_lower = file.title.to_lowercase();

                // 移除 .md 扩展名进行比较
                let name_without_ext = name_lower.trim_end_matches(".md");
                let path_without_ext = path_lower.trim_end_matches(".md");

                // 匹配逻辑：
                // 1. 文件名完全匹配工具 ID 或名称
                // 2. 文件名包含工具 ID 或名称
                // 3. 路径包含工具 ID 或名称
                // 4. 标题包含工具 ID 或名称
                if name_without_ext == tool_id
                    || name_without_ext == tool_name
                    || name_without_ext.contains(tool_id)
                    || name_without_ext.contains(tool_name)
                    || path_without_ext.contains(tool_id)
                    || path_without_ext.contains(tool_name)
                    || title_lower.contains(tool_id)
                    || title_lower.contains(tool_name)
                {
                    return Some(file.path.clone());
                }
            }
        }
        None
    }

    Ok(search_files(&files, &tool_id_lower, &tool_name_lower))
}

/// 设置当前主题
#[tauri::command]
#[allow(dead_code)]
pub fn set_wiki_theme(theme_name: String) -> Result<String, String> {
    use crate::utils::get_theme_dir;
    use std::fs;

    let theme_dir = get_theme_dir();
    std::fs::create_dir_all(&theme_dir).map_err(|e| format!("创建主题目录失败: {}", e))?;

    let config_file = theme_dir.join("current_theme.txt");

    fs::write(&config_file, theme_name).map_err(|e| format!("保存主题配置失败: {}", e))?;

    Ok("主题已更新".to_string())
}

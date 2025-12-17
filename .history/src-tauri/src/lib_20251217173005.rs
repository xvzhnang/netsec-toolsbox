use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use sha2::{Sha256, Digest};
use regex::Regex;
use image::DynamicImage;
use base64::{Engine as _, engine::general_purpose};

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
  #[serde(alias = "iconUrl", alias = "icon_url")]
  icon_url: Option<String>,
  #[serde(alias = "wikiUrl", alias = "wiki_url")]
  wiki_url: Option<String>,
  #[serde(alias = "toolType", alias = "tool_type")]
  tool_type: Option<String>,
  #[serde(alias = "execPath", alias = "exec_path")]
  exec_path: Option<String>,
  args: Option<Vec<String>>,
  #[serde(alias = "workingDir", alias = "working_dir")]
  working_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JarConfig {
  #[serde(alias = "jarPath", alias = "jar_path")]
  jar_path: String,
  #[serde(alias = "javaPath", alias = "java_path")]
  java_path: Option<String>,
  #[serde(alias = "jvmArgs", alias = "jvm_args")]
  jvm_args: Option<Vec<String>>,
  #[serde(alias = "programArgs", alias = "program_args")]
  program_args: Option<Vec<String>>,
}

/// 启动工具的参数结构体（支持 camelCase 和 snake_case）
#[derive(Debug, Deserialize)]
struct LaunchToolParams {
  #[serde(alias = "toolType", alias = "tool_type")]
  tool_type: Option<String>,
  #[serde(alias = "execPath", alias = "exec_path")]
  exec_path: Option<String>,
  args: Option<Vec<String>>,
  #[serde(alias = "workingDir", alias = "working_dir")]
  working_dir: Option<String>,
  #[serde(alias = "jarConfig", alias = "jar_config")]
  jar_config: Option<JarConfig>,
}

/// 提取图标的参数结构体（支持 camelCase 和 snake_case）
#[derive(Debug, Deserialize)]
struct ExtractIconParams {
  #[serde(alias = "filePath", alias = "file_path")]
  file_path: String,
  #[serde(alias = "toolType", alias = "tool_type")]
  tool_type: Option<String>,
}

/// 获取 favicon 的参数结构体（支持 camelCase 和 snake_case）
#[derive(Debug, Deserialize)]
struct FetchFaviconParams {
  #[serde(alias = "urlStr", alias = "url_str")]
  url_str: String,
}

/// 上传文件的参数结构体（支持 camelCase 和 snake_case）
#[derive(Debug, Deserialize)]
struct UploadFileParams {
  #[serde(alias = "fileName", alias = "file_name")]
  file_name: String,
  #[serde(alias = "fileData", alias = "file_data")]
  file_data: String, // base64 编码的文件数据
  #[serde(alias = "toolId", alias = "tool_id")]
  tool_id: Option<String>, // 可选的工具ID，用于组织文件
}

/// 解析文件路径的参数结构体
#[derive(Debug, Deserialize)]
struct ResolveFilePathParams {
  #[serde(alias = "filePath", alias = "file_path")]
  file_path: String,
}

fn get_config_dir() -> PathBuf {
  // 使用当前工作目录下的 .config 文件夹
  let current_dir = std::env::current_dir()
    .expect("failed to get current directory");
  let config_dir = current_dir.join(".config");
  
  // 确保 .config 目录存在
  std::fs::create_dir_all(&config_dir)
    .expect("failed to create .config directory");
  
  config_dir
}

fn get_icons_dir() -> PathBuf {
  let icons_dir = get_config_dir().join("icons");
  std::fs::create_dir_all(&icons_dir)
    .expect("failed to create icons directory");
  icons_dir
}

fn get_uploads_dir() -> PathBuf {
  let uploads_dir = get_config_dir().join("uploads");
  std::fs::create_dir_all(&uploads_dir)
    .expect("failed to create uploads directory");
  uploads_dir
}

/// 生成文件路径的哈希值（用于缓存文件名）
fn hash_path(path: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(path.as_bytes());
  let hash = hasher.finalize();
  hex::encode(&hash[..16]) // 使用前16字节，32个十六进制字符
}

fn get_config_path(_app: &tauri::AppHandle, filename: &str) -> PathBuf {
  get_config_dir().join(filename)
}

// 通用的读取配置文件命令
#[tauri::command]
fn read_config_file(filename: String) -> Result<String, String> {
  let config_path = get_config_dir().join(&filename);
  if config_path.exists() {
    fs::read_to_string(&config_path)
      .map_err(|e| format!("Failed to read config file {}: {}", filename, e))
  } else {
    // 返回空JSON，前端会使用默认值
    Ok("{}".to_string())
  }
}

// 通用的写入配置文件命令
#[tauri::command]
fn write_config_file(filename: String, content: String) -> Result<(), String> {
  let config_path = get_config_dir().join(&filename);
  fs::write(&config_path, content)
    .map_err(|e| format!("Failed to write config file {}: {}", filename, e))
}

// 检查配置文件是否存在
#[tauri::command]
fn config_file_exists(filename: String) -> Result<bool, String> {
  let config_path = get_config_dir().join(&filename);
  Ok(config_path.exists())
}

// 兼容旧版本的命令（保留以向后兼容）
#[tauri::command]
fn read_categories_config(_app: tauri::AppHandle) -> Result<String, String> {
  read_config_file("categories.json".to_string())
}

#[tauri::command]
fn write_categories_config(_app: tauri::AppHandle, content: String) -> Result<(), String> {
  write_config_file("categories.json".to_string(), content)
}

/// 获取配置文件路径（用于显示给用户）
#[tauri::command]
fn get_config_file_path(filename: String) -> Result<String, String> {
  let config_path = get_config_dir().join(&filename);
  config_path
    .to_str()
    .ok_or("无法转换路径为字符串".to_string())
    .map(|s| s.to_string())
}

/// 从文件路径提取所在目录
fn get_file_dir(file_path: &str) -> PathBuf {
  let path = Path::new(file_path);
  if let Some(parent) = path.parent() {
    parent.to_path_buf()
  } else {
    PathBuf::from(".")
  }
}

/// 启动 GUI 工具（直接启动，不打开终端）
fn launch_gui_tool(exec_path: &str, args: Option<Vec<String>>, working_dir: Option<String>) -> Result<(), String> {
  let path = Path::new(exec_path);
  if !path.exists() {
    return Err(format!("文件不存在: {}", exec_path));
  }

  let mut cmd = Command::new(exec_path);
  
  // 设置工作目录
  if let Some(wd) = working_dir {
    cmd.current_dir(&wd);
  } else if let Some(parent) = path.parent() {
    cmd.current_dir(parent);
  }

  // 添加参数
  if let Some(ref args_vec) = args {
    cmd.args(args_vec);
  }

  // 启动进程（不等待）
  cmd.spawn()
    .map_err(|e| format!("启动工具失败: {}", e))?;

  Ok(())
}

/// 在 Windows 上打开 PowerShell 并执行命令
#[cfg(target_os = "windows")]
fn launch_in_terminal_windows(working_dir: &Path, command: &str) -> Result<(), String> {
  // 构建 PowerShell 命令
  // 使用 Start-Process 启动新的 PowerShell 窗口，并执行命令
  let ps_command = format!(
    "Start-Process powershell -ArgumentList '-NoExit', '-Command', 'Set-Location ''{}''; {}'",
    working_dir.to_string_lossy().replace('\'', "''"),
    command.replace('\'', "''")
  );

  Command::new("powershell")
    .args(&["-Command", &ps_command])
    .spawn()
    .map_err(|e| format!("启动终端失败: {}", e))?;

  Ok(())
}

/// 在 macOS 上打开终端并执行命令
#[cfg(target_os = "macos")]
fn launch_in_terminal_unix(working_dir: &Path, command: &str) -> Result<(), String> {
  // macOS 使用 AppleScript 打开 Terminal.app
  let working_dir_str = working_dir.to_string_lossy();
  let script = format!(
    "tell application \"Terminal\"\n  activate\n  do script \"cd '{}' && {}\"\nend tell",
    working_dir_str.replace('\'', "'\\''"),
    command.replace('\'', "'\\''").replace('"', "\\\"")
  );
  
  Command::new("osascript")
    .args(&["-e", &script])
    .spawn()
    .map_err(|e| format!("启动终端失败: {}", e))?;

  Ok(())
}

/// 在 Linux 上打开终端并执行命令
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn launch_in_terminal_unix(working_dir: &Path, command: &str) -> Result<(), String> {
  // 尝试使用常见的终端模拟器，按优先级排序
  let terminal_commands = vec![
    ("gnome-terminal", vec!["--working-directory", &working_dir.to_string_lossy(), "--", "bash", "-c", &format!("{}; exec bash", command)]),
    ("xterm", vec!["-e", "bash", "-c", &format!("cd '{}' && {}; exec bash", working_dir.to_string_lossy().replace('\'', "'\\''"), command.replace('\'', "'\\''"))]),
    ("konsole", vec!["--workdir", &working_dir.to_string_lossy(), "-e", "bash", "-c", &format!("{}; exec bash", command)]),
    ("x-terminal-emulator", vec!["-e", "bash", "-c", &format!("cd '{}' && {}; exec bash", working_dir.to_string_lossy().replace('\'', "'\\''"), command.replace('\'', "'\\''"))]),
  ];

  for (terminal, args) in terminal_commands {
    if let Ok(mut child) = Command::new(terminal)
      .args(&args)
      .spawn() {
      // 不等待子进程，让它独立运行
      let _ = child.wait();
      return Ok(());
    }
  }

  Err("无法找到可用的终端模拟器（请安装 gnome-terminal、xterm、konsole 或 x-terminal-emulator）".to_string())
}

/// 启动 CLI 工具（在对应目录打开终端执行）
fn launch_cli_tool(exec_path: &str, args: Option<Vec<String>>) -> Result<(), String> {
  let path = Path::new(exec_path);
  if !path.exists() {
    return Err(format!("文件不存在: {}", exec_path));
  }

  let working_dir = get_file_dir(exec_path);
  
  // 构建命令
  let mut command = exec_path.to_string();
  if let Some(ref args_vec) = args {
    for arg in args_vec {
      // 如果参数包含空格或特殊字符，用引号包裹
      if arg.contains(' ') || arg.contains('\'') || arg.contains('"') {
        command.push_str(&format!(" \"{}\"", arg.replace('"', "\\\"")));
      } else {
        command.push_str(&format!(" {}", arg));
      }
    }
  }

  // 在终端中执行
  #[cfg(target_os = "windows")]
  launch_in_terminal_windows(&working_dir, &command)?;

  #[cfg(not(target_os = "windows"))]
  launch_in_terminal_unix(&working_dir, &command)?;

  Ok(())
}

/// 启动 Python 脚本（在对应目录打开终端执行）
fn launch_python_tool(exec_path: &str, args: Option<Vec<String>>) -> Result<(), String> {
  let path = Path::new(exec_path);
  if !path.exists() {
    return Err(format!("文件不存在: {}", exec_path));
  }

  let working_dir = get_file_dir(exec_path);
  
  // 确定 Python 命令（Windows 使用 python，Linux/macOS 使用 python3）
  let python_cmd = if cfg!(target_os = "windows") {
    "python"
  } else {
    "python3"
  };

  // 构建命令
  let mut command = format!("{} \"{}\"", python_cmd, exec_path.replace('"', "\\\""));
  if let Some(ref args_vec) = args {
    for arg in args_vec {
      // 如果参数包含空格或特殊字符，用引号包裹
      if arg.contains(' ') || arg.contains('\'') || arg.contains('"') {
        command.push_str(&format!(" \"{}\"", arg.replace('"', "\\\"")));
      } else {
        command.push_str(&format!(" {}", arg));
      }
    }
  }

  // 在终端中执行
  #[cfg(target_os = "windows")]
  launch_in_terminal_windows(&working_dir, &command)?;

  #[cfg(not(target_os = "windows"))]
  launch_in_terminal_unix(&working_dir, &command)?;

  Ok(())
}

/// 启动 JAR 工具（在对应目录打开终端执行）
fn launch_jar_tool(jar_config: &JarConfig) -> Result<(), String> {
  let jar_path = Path::new(&jar_config.jar_path);
  if !jar_path.exists() {
    return Err(format!("JAR 文件不存在: {}", jar_config.jar_path));
  }

  let working_dir = get_file_dir(&jar_config.jar_path);
  
  // 确定 Java 命令
  let java_cmd = if let Some(ref java_path) = jar_config.java_path {
    java_path.clone()
  } else {
    "java".to_string()
  };

  // 构建 Java 命令
  let mut command = format!("{}", java_cmd);
  
  // 添加 JVM 参数
  if let Some(ref jvm_args) = jar_config.jvm_args {
    for arg in jvm_args {
      command.push_str(&format!(" {}", arg));
    }
  }
  
  // 添加 -jar 和 JAR 路径
  command.push_str(&format!(" -jar \"{}\"", jar_config.jar_path.replace('"', "\\\"")));
  
  // 添加程序参数
  if let Some(ref program_args) = jar_config.program_args {
    for arg in program_args {
      // 如果参数包含空格或特殊字符，用引号包裹
      if arg.contains(' ') || arg.contains('\'') || arg.contains('"') {
        command.push_str(&format!(" \"{}\"", arg.replace('"', "\\\"")));
      } else {
        command.push_str(&format!(" {}", arg));
      }
    }
  }

  // 在终端中执行
  #[cfg(target_os = "windows")]
  launch_in_terminal_windows(&working_dir, &command)?;

  #[cfg(not(target_os = "windows"))]
  launch_in_terminal_unix(&working_dir, &command)?;

  Ok(())
}

/// 启动 LNK 工具（Windows 快捷方式）
#[cfg(target_os = "windows")]
fn launch_lnk_tool(exec_path: &str) -> Result<(), String> {
  let path = Path::new(exec_path);
  if !path.exists() {
    return Err(format!("快捷方式文件不存在: {}", exec_path));
  }

  // Windows 上使用 start 命令打开快捷方式
  // start 命令会自动处理快捷方式并启动目标程序
  Command::new("cmd")
    .args(&["/C", "start", "", exec_path])
    .spawn()
    .map_err(|e| format!("启动快捷方式失败: {}", e))?;

  Ok(())
}

#[cfg(not(target_os = "windows"))]
fn launch_lnk_tool(_exec_path: &str) -> Result<(), String> {
  Err("LNK 工具仅在 Windows 系统上支持".to_string())
}

/// 打开 URL 在默认浏览器中（用于网页工具）
fn open_url_in_browser(url: &str) -> Result<(), String> {
  // 验证 URL 格式
  if !url.starts_with("http://") && !url.starts_with("https://") {
    return Err(format!("无效的 URL 格式: {}", url));
  }

  // 在默认浏览器中打开
  #[cfg(target_os = "windows")]
  {
    // Windows: 使用 start 命令打开默认浏览器
    Command::new("cmd")
      .args(&["/C", "start", "", url])
      .spawn()
      .map_err(|e| format!("打开浏览器失败: {}", e))?;
  }

  #[cfg(target_os = "macos")]
  {
    // macOS: 使用 open 命令
    Command::new("open")
      .arg(url)
      .spawn()
      .map_err(|e| format!("打开浏览器失败: {}", e))?;
  }

  #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
  {
    // Linux: 使用 xdg-open
    Command::new("xdg-open")
      .arg(url)
      .spawn()
      .map_err(|e| format!("打开浏览器失败: {}", e))?;
  }

  Ok(())
}

/// 启动 HTML 工具（本地 HTML 文件在浏览器中打开）
fn launch_html_tool(exec_path: &str) -> Result<(), String> {
  let path = Path::new(exec_path);
  if !path.exists() {
    return Err(format!("HTML 文件不存在: {}", exec_path));
  }

  // 将路径转换为绝对路径并规范化
  let abs_path = if path.is_absolute() {
    path.canonicalize()
      .map_err(|e| format!("无法解析路径: {}", e))?
  } else {
    std::env::current_dir()
      .map_err(|e| format!("获取当前目录失败: {}", e))?
      .join(path)
      .canonicalize()
      .map_err(|e| format!("无法解析路径: {}", e))?
  };

  // 转换为 file:// URL
  let file_url = if cfg!(target_os = "windows") {
    // Windows: file:///C:/path/to/file.html
    // 需要将反斜杠转换为正斜杠，并确保路径格式正确
    let path_str = abs_path.to_string_lossy().replace('\\', "/");
    format!("file:///{}", path_str)
  } else {
    // Unix-like: file:///path/to/file.html
    format!("file://{}", abs_path.to_string_lossy())
  };

  // 在浏览器中打开
  #[cfg(target_os = "windows")]
  {
    // Windows: 使用 start 命令打开默认浏览器
    Command::new("cmd")
      .args(&["/C", "start", "", &file_url])
      .spawn()
      .map_err(|e| format!("打开浏览器失败: {}", e))?;
  }

  #[cfg(target_os = "macos")]
  {
    // macOS: 使用 open 命令
    Command::new("open")
      .arg(&file_url)
      .spawn()
      .map_err(|e| format!("打开浏览器失败: {}", e))?;
  }

  #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
  {
    // Linux: 尝试使用 xdg-open
    Command::new("xdg-open")
      .arg(&file_url)
      .spawn()
      .map_err(|e| format!("打开浏览器失败: {}", e))?;
  }

  Ok(())
}

/// 启动工具的主函数
/// 参数支持 camelCase（前端）和 snake_case（Rust）两种命名方式
#[tauri::command]
fn launch_tool(params: LaunchToolParams) -> Result<(), String> {
  let tool_type = params.tool_type.as_deref().unwrap_or("GUI");
  let exec_path = params.exec_path;
  let args = params.args;
  let working_dir = params.working_dir;
  let jar_config = params.jar_config;
  
  // 调试日志（开发环境）
  #[cfg(debug_assertions)]
  {
    log::info!(
      "启动工具: type={:?} (len={}), exec_path={:?}, args={:?}, working_dir={:?}, jar_config={:?}", 
      tool_type, 
      tool_type.len(),
      exec_path,
      args,
      working_dir,
      jar_config
    );
  }
  
  match tool_type {
    "GUI" => {
      let exec_path = exec_path.ok_or("GUI 工具需要 exec_path")?;
      launch_gui_tool(&exec_path, args, working_dir)
    }
    "CLI" => {
      let exec_path = exec_path.ok_or("CLI 工具需要 exec_path")?;
      launch_cli_tool(&exec_path, args)
    }
    "Python" => {
      let exec_path = exec_path.ok_or("Python 工具需要 exec_path")?;
      launch_python_tool(&exec_path, args)
    }
    "JAR" => {
      let jar_config = jar_config.ok_or("JAR 工具需要 jar_config")?;
      launch_jar_tool(&jar_config)
    }
    "LNK" => {
      let exec_path = exec_path.ok_or("LNK 工具需要 exec_path")?;
      launch_lnk_tool(&exec_path)
    }
    "HTML" => {
      let exec_path = exec_path.ok_or("HTML 工具需要 exec_path")?;
      launch_html_tool(&exec_path)
    }
    "网页" => {
      let exec_path = exec_path.ok_or("网页工具需要 URL 地址")?;
      open_url_in_browser(&exec_path)
    }
    _ => Err(format!("不支持的工具类型: {}", tool_type)),
  }
}

/// 将图标转换为统一尺寸的 PNG base64
fn process_icon_to_base64(img: DynamicImage, size: u32) -> Result<String, String> {
  // 调整尺寸为正方形
  let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
  
  // 转换为 RGBA
  let rgba = resized.to_rgba8();
  
  // 编码为 PNG
  let mut png_data = Vec::new();
  {
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    let encoder = PngEncoder::new(&mut png_data);
    encoder.write_image(&rgba, size, size, image::ColorType::Rgba8.into())
      .map_err(|e| format!("PNG 编码失败: {}", e))?;
  }
  
  // 转换为 base64
  Ok(format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(&png_data)))
}

/// 从文件提取图标（EXE/LNK/HTML）
#[tauri::command]
fn extract_icon_from_file(params: ExtractIconParams) -> Result<String, String> {
  let file_path = params.file_path;
  let tool_type = params.tool_type;
  
  // 规范化路径（转换为绝对路径）
  let path = Path::new(&file_path);
  let abs_path = if path.is_absolute() {
    path.canonicalize()
      .map_err(|e| format!("无法解析路径 {}: {}", file_path, e))?
  } else {
    std::env::current_dir()
      .map_err(|e| format!("获取当前目录失败: {}", e))?
      .join(path)
      .canonicalize()
      .map_err(|e| format!("无法解析路径 {}: {}", file_path, e))?
  };
  
  // 移除 Windows 长路径前缀 (\\?\)，因为某些 API 可能不支持
  let file_path_str = abs_path.to_string_lossy().to_string();
  let file_path_str = if file_path_str.starts_with("\\\\?\\") {
    &file_path_str[4..]
  } else {
    &file_path_str
  }.to_string();
  
  if !abs_path.exists() {
    return Err(format!("文件不存在: {}", file_path_str));
  }
  
  let tool_type = tool_type.as_deref().unwrap_or("");
  
  // 检查缓存（使用绝对路径作为缓存键）
  let cache_key = hash_path(&file_path_str);
  let cache_path = get_icons_dir().join(format!("{}.png", cache_key));
  if cache_path.exists() {
    // 从缓存读取
    match fs::read(&cache_path) {
      Ok(data) => {
        match image::load_from_memory(&data) {
          Ok(img) => {
            log::info!("从缓存加载图标: {}", file_path_str);
            return process_icon_to_base64(img, 160);
          }
          Err(_) => {
            // 缓存文件损坏，删除并重新提取
            let _ = fs::remove_file(&cache_path);
          }
        }
      }
      Err(_) => {}
    }
  }
  
  log::info!("开始提取图标: file_path={}, tool_type={}", file_path_str, tool_type);
  
  // 根据工具类型提取图标
  let icon_data = match tool_type {
    "LNK" => {
      #[cfg(target_os = "windows")]
      {
        log::info!("提取 LNK 图标: {}", file_path_str);
        extract_lnk_icon(&file_path_str)?
      }
      #[cfg(not(target_os = "windows"))]
      {
        return Err("LNK 图标提取仅在 Windows 上支持".to_string());
      }
    }
    "HTML" => {
      log::info!("提取 HTML 图标: {}", file_path_str);
      extract_html_icon(&file_path_str)?
    }
    _ => {
      // EXE 或其他可执行文件
      #[cfg(target_os = "windows")]
      {
        log::info!("提取 EXE 图标: {}", file_path_str);
        extract_exe_icon(&file_path_str)?
      }
      #[cfg(not(target_os = "windows"))]
      {
        return Err("EXE 图标提取仅在 Windows 上支持".to_string());
      }
    }
  };
  
  // 处理图标尺寸
  let base64 = process_icon_to_base64(icon_data, 160)?;
  
  log::info!("图标提取成功: file_path={}, base64_length={}", file_path_str, base64.len());
  
  // 保存到缓存
  if let Ok(img) = image::load_from_memory(&general_purpose::STANDARD.decode(
    base64.strip_prefix("data:image/png;base64,").unwrap_or(&base64)
  ).map_err(|e| format!("Base64 解码失败: {}", e))?) {
    let rgba = img.to_rgba8();
    let mut png_data = Vec::new();
    {
      use image::codecs::png::PngEncoder;
      use image::ImageEncoder;
      let encoder = PngEncoder::new(&mut png_data);
      encoder.write_image(&rgba, 160, 160, image::ColorType::Rgba8.into())
        .map_err(|e| format!("PNG 编码失败: {}", e))?;
    }
    fs::write(&cache_path, &png_data)
      .map_err(|e| format!("保存图标缓存失败: {}", e))?;
    log::info!("图标缓存已保存: {}", cache_path.to_string_lossy());
  }
  
  Ok(base64)
}

/// 将 HICON 句柄转换为 DynamicImage（在内存中转换，不生成文件）
#[cfg(target_os = "windows")]
fn hicon_to_image(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Result<DynamicImage, String> {
  use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo};
  use windows::Win32::Graphics::Gdi::{
    GetObjectW, BITMAP, GetDC, ReleaseDC, CreateCompatibleDC, 
    SelectObject, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS
  };
  use std::ptr;
  
  unsafe {
    // 获取图标信息
    let mut icon_info = std::mem::zeroed();
    if GetIconInfo(hicon, &mut icon_info).is_err() {
      let _ = DestroyIcon(hicon);
      return Err("获取图标信息失败".to_string());
    }
    
    // 获取位图信息
    let mut bmp = BITMAP::default();
    let result = GetObjectW(icon_info.hbmColor, std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp as *const _ as *mut _));
    if result == 0 {
      let _ = DestroyIcon(hicon);
      return Err("获取位图信息失败".to_string());
    }
    
    let width = bmp.bmWidth as u32;
    let height = bmp.bmHeight as u32;
    
    // 创建兼容的 DC
    let hdc = GetDC(None);
    let hdc_mem = CreateCompatibleDC(hdc);
    
    // 创建 BITMAPINFO 结构
    let mut bmi = BITMAPINFO {
      bmiHeader: BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32), // 负值表示从上到下的位图
        biPlanes: 1,
        biBitCount: 32,
        biCompression: 0, // BI_RGB
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
      },
      bmiColors: [std::mem::zeroed(); 1],
    };
    
    // 创建 DIB section
    let mut bits_ptr: *mut std::ffi::c_void = ptr::null_mut();
    let hdib = CreateDIBSection(
      hdc_mem,
      &bmi,
      DIB_RGB_COLORS,
      &mut bits_ptr,
      None,
      0,
    ).map_err(|e| format!("创建 DIB section 失败: {:?}", e))?;
    
    // 选择 DIB 到内存 DC
    let _old_obj = SelectObject(hdc_mem, hdib);
    
    // 将图标位图复制到 DIB
    use windows::Win32::Graphics::Gdi::{BitBlt, SRCCOPY};
    let hdc_src = CreateCompatibleDC(hdc);
    let _old_src = SelectObject(hdc_src, icon_info.hbmColor);
    
    if BitBlt(hdc_mem, 0, 0, width as i32, height as i32, hdc_src, 0, 0, SRCCOPY).is_ok() {
      // 从 DIB 读取数据
      let stride = (width * 4) as usize; // RGBA = 4 bytes per pixel
      let buffer_size = stride * height as usize;
      let mut buffer = vec![0u8; buffer_size];
      
      // 复制位图数据
      if !bits_ptr.is_null() {
        std::ptr::copy_nonoverlapping(bits_ptr as *const u8, buffer.as_mut_ptr(), buffer_size);
        
        // 转换 BGRA 到 RGBA（Windows 位图是 BGRA 格式）
        let mut rgba_buffer = Vec::with_capacity(buffer_size);
        for chunk in buffer.chunks_exact(4) {
          rgba_buffer.push(chunk[2]); // R
          rgba_buffer.push(chunk[1]); // G
          rgba_buffer.push(chunk[0]); // B
          rgba_buffer.push(chunk[3]); // A
        }
        
        // 清理资源
        SelectObject(hdc_src, _old_src);
        SelectObject(hdc_mem, _old_obj);
        let _ = DeleteObject(hdib);
        ReleaseDC(None, hdc_src);
        ReleaseDC(None, hdc_mem);
        ReleaseDC(None, hdc);
        let _ = DestroyIcon(hicon);
        
        // 创建图像
        match image::RgbaImage::from_raw(width, height, rgba_buffer) {
          Some(img) => return Ok(DynamicImage::ImageRgba8(img)),
          None => return Err("无法从位图数据创建图像".to_string()),
        }
      }
    }
    
    // 清理资源（失败情况）
    SelectObject(hdc_src, _old_src);
    SelectObject(hdc_mem, _old_obj);
    let _ = DeleteObject(hdib);
    ReleaseDC(None, hdc_src);
    ReleaseDC(None, hdc_mem);
    ReleaseDC(None, hdc);
    let _ = DestroyIcon(hicon);
    Err("无法提取图标位图数据".to_string())
  }
}

/// 提取 EXE 图标（Windows）
/// 使用 PowerShell/.NET 的 System.Drawing.Icon::ExtractAssociatedIcon 提取图标
/// 支持 EXE、DLL、LNK 等文件类型
#[cfg(target_os = "windows")]
fn extract_exe_icon(file_path: &str) -> Result<DynamicImage, String> {
  // 移除 Windows 长路径前缀 (\\?\)，因为 PowerShell 可能不支持
  let clean_path = if file_path.starts_with("\\\\?\\") {
    &file_path[4..]
  } else {
    file_path
  };
  
  log::info!("提取 EXE 图标，清理后的路径: {}", clean_path);
  
  // 转义路径中的反斜杠，用于 PowerShell 命令
  let escaped_path = clean_path.replace('\\', "\\\\");
  
  // 使用 PowerShell/.NET 提取图标
  let script = format!(r#"
try {{
  Add-Type -AssemblyName System.Drawing
  $ic = [System.Drawing.Icon]::ExtractAssociatedIcon('{}')
  if ($ic -ne $null) {{
    $bmp = $ic.ToBitmap()
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    [Convert]::ToBase64String($ms.ToArray())
  }}
}} catch {{
  Write-Error $_
  exit 1
}}
"#, escaped_path);
  
  let output = Command::new("powershell")
    .arg("-NoProfile")
    .arg("-Command")
    .arg(&script)
    .output()
    .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
  
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(format!("PowerShell 提取图标失败: {}", stderr));
  }
  
  let stdout = String::from_utf8_lossy(&output.stdout);
  let base64_str = stdout.trim();
  
  if base64_str.is_empty() {
    return Err("PowerShell 返回空结果".to_string());
  }
  
  // 解码 base64 并转换为图片
  let image_bytes = general_purpose::STANDARD.decode(base64_str)
    .map_err(|e| format!("Base64 解码失败: {}", e))?;
  
  image::load_from_memory(&image_bytes)
    .map_err(|e| format!("加载图片失败: {}", e))
}

/// 提取 LNK 图标（Windows）
/// 使用 PowerShell/.NET 解析快捷方式并提取图标
/// 优先使用快捷方式的 IconLocation，如果没有则使用目标路径的图标
#[cfg(target_os = "windows")]
fn extract_lnk_icon(lnk_path: &str) -> Result<DynamicImage, String> {
  // 移除 Windows 长路径前缀 (\\?\)，因为 PowerShell 可能不支持
  let clean_path = if lnk_path.starts_with("\\\\?\\") {
    &lnk_path[4..]
  } else {
    lnk_path
  };
  
  log::info!("提取 LNK 图标，清理后的路径: {}", clean_path);
  
  // 转义路径中的反斜杠，用于 PowerShell 命令
  let escaped_path = clean_path.replace('\\', "\\\\");
  
  // 使用 PowerShell/.NET 提取 LNK 图标
  // 优先使用快捷方式的 IconLocation，如果没有则使用目标路径
  let script = format!(r#"
try {{
  $s = (New-Object -ComObject WScript.Shell).CreateShortcut('{}')
  $icon = $s.IconLocation
  if ([string]::IsNullOrEmpty($icon)) {{
    $icon = $s.TargetPath
  }}
  $parts = $icon -split ','
  $iconFile = $parts[0].Trim()
  Add-Type -AssemblyName System.Drawing
  $ic = [System.Drawing.Icon]::ExtractAssociatedIcon($iconFile)
  if ($ic -ne $null) {{
    $bmp = $ic.ToBitmap()
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    [Convert]::ToBase64String($ms.ToArray())
  }}
}} catch {{
  Write-Error $_
  exit 1
}}
"#, escaped_path);
  
  let output = Command::new("powershell")
    .arg("-NoProfile")
    .arg("-Command")
    .arg(&script)
    .output()
    .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
  
  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(format!("PowerShell 提取 LNK 图标失败: {}", stderr));
  }
  
  let stdout = String::from_utf8_lossy(&output.stdout);
  let base64_str = stdout.trim();
  
  if base64_str.is_empty() {
    return Err("PowerShell 返回空结果".to_string());
  }
  
  // 解码 base64 并转换为图片
  let image_bytes = general_purpose::STANDARD.decode(base64_str)
    .map_err(|e| format!("Base64 解码失败: {}", e))?;
  
  image::load_from_memory(&image_bytes)
    .map_err(|e| format!("加载图片失败: {}", e))
}

/// 提取 HTML 文件的图标
fn extract_html_icon(html_path: &str) -> Result<DynamicImage, String> {
  // 读取 HTML 文件内容
  let content = fs::read_to_string(html_path)
    .map_err(|e| format!("读取 HTML 文件失败: {}", e))?;
  
  // 查找 favicon 链接
  let favicon_re = Regex::new(r#"(?i)<link[^>]+rel=["'](?:icon|shortcut\s+icon)["'][^>]*href=["']([^"']+)["']"#)
    .map_err(|e| format!("正则表达式错误: {}", e))?;
  
  if let Some(cap) = favicon_re.captures(&content) {
    if let Some(favicon_path) = cap.get(1) {
      let favicon_url = favicon_path.as_str();
      
      // 如果是相对路径，转换为绝对路径
      let favicon_abs_path = if favicon_url.starts_with("http://") || favicon_url.starts_with("https://") {
        return Err("HTML 文件中的绝对 URL favicon 需要使用 fetch_favicon 命令".to_string());
      } else {
        let html_dir = Path::new(html_path).parent()
          .ok_or("无法获取 HTML 文件目录")?;
        html_dir.join(favicon_url)
      };
      
      if favicon_abs_path.exists() {
        return image::open(&favicon_abs_path)
          .map_err(|e| format!("加载 favicon 图片失败: {}", e));
      }
    }
  }
  
  // 如果没有找到 favicon，返回默认图标
  Ok(DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(160, 160, image::Rgba([200, 200, 200, 255]))))
}

/// 从 URL 抓取 favicon
#[tauri::command]
fn fetch_favicon(params: FetchFaviconParams) -> Result<String, String> {
  let url_str = params.url_str;
  // 解析 URL
  let url = url::Url::parse(&url_str)
    .map_err(|e| format!("无效的 URL: {}", e))?;
  
  // 检查缓存
  let cache_key = hash_path(&url_str);
  let cache_path = get_icons_dir().join(format!("{}.png", cache_key));
  if cache_path.exists() {
    match fs::read(&cache_path) {
      Ok(data) => {
        match image::load_from_memory(&data) {
          Ok(img) => {
            return process_icon_to_base64(img, 160);
          }
          Err(_) => {
            let _ = fs::remove_file(&cache_path);
          }
        }
      }
      Err(_) => {}
    }
  }
  
  // 尝试多个常见的 favicon 路径
  let base_url = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
  let favicon_paths = vec![
    format!("{}/favicon.ico", base_url),
    format!("{}/favicon.png", base_url),
    format!("{}/apple-touch-icon.png", base_url),
  ];
  
  for favicon_url in favicon_paths {
    match reqwest::blocking::get(&favicon_url) {
      Ok(response) => {
        if response.status().is_success() {
          match response.bytes() {
            Ok(bytes) => {
              match image::load_from_memory(&bytes) {
                Ok(img) => {
                  let base64 = process_icon_to_base64(img, 160)?;
                  
                  // 保存到缓存
                  if let Ok(img) = image::load_from_memory(&bytes) {
                    let rgba = img.resize_exact(160, 160, image::imageops::FilterType::Lanczos3).to_rgba8();
                    let mut png_data = Vec::new();
                    {
                      use image::codecs::png::PngEncoder;
                      use image::ImageEncoder;
                      let encoder = PngEncoder::new(&mut png_data);
                      encoder.write_image(&rgba, 160, 160, image::ColorType::Rgba8.into())
                        .map_err(|e| format!("PNG 编码失败: {}", e))?;
                    }
                    fs::write(&cache_path, &png_data)
                      .map_err(|e| format!("保存图标缓存失败: {}", e))?;
                  }
                  
                  return Ok(base64);
                }
                Err(_) => continue,
              }
            }
            Err(_) => continue,
          }
        }
      }
      Err(_) => continue,
    }
  }
  
  // 如果所有路径都失败，尝试从 HTML 中提取
  match reqwest::blocking::get(&url_str) {
    Ok(response) => {
      if response.status().is_success() {
        if let Ok(html_content) = response.text() {
          let favicon_re = Regex::new(r#"(?i)<link[^>]+rel=["'](?:icon|shortcut\s+icon)["'][^>]*href=["']([^"']+)["']"#)
            .map_err(|e| format!("正则表达式错误: {}", e))?;
          
          if let Some(cap) = favicon_re.captures(&html_content) {
            if let Some(favicon_rel_path) = cap.get(1) {
              let favicon_url = if favicon_rel_path.as_str().starts_with("http://") || favicon_rel_path.as_str().starts_with("https://") {
                favicon_rel_path.as_str().to_string()
              } else {
                format!("{}/{}", base_url, favicon_rel_path.as_str().trim_start_matches('/'))
              };
              
              match reqwest::blocking::get(&favicon_url) {
                Ok(response) => {
                  if response.status().is_success() {
                    if let Ok(bytes) = response.bytes() {
                      if let Ok(img) = image::load_from_memory(&bytes) {
                        let base64 = process_icon_to_base64(img, 160)?;
                        
                        // 保存到缓存
                        if let Ok(img) = image::load_from_memory(&bytes) {
                          let rgba = img.resize_exact(160, 160, image::imageops::FilterType::Lanczos3).to_rgba8();
                          let mut png_data = Vec::new();
                          {
                            use image::codecs::png::PngEncoder;
                            use image::ImageEncoder;
                            let encoder = PngEncoder::new(&mut png_data);
                            encoder.write_image(&rgba, 160, 160, image::ColorType::Rgba8.into())
                              .map_err(|e| format!("PNG 编码失败: {}", e))?;
                          }
                          fs::write(&cache_path, &png_data)
                            .map_err(|e| format!("保存图标缓存失败: {}", e))?;
                        }
                        
                        return Ok(base64);
                      }
                    }
                  }
                }
                Err(_) => {}
              }
            }
          }
        }
      }
    }
    Err(_) => {}
  }
  
  // 如果都失败，返回默认图标
  let default_icon = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(160, 160, image::Rgba([200, 200, 200, 255])));
  Ok(process_icon_to_base64(default_icon, 160)?)
}

/// 上传文件并保存到 uploads 目录
/// 返回保存后的文件路径
#[tauri::command]
fn upload_file(params: UploadFileParams) -> Result<String, String> {
  let file_name = params.file_name;
  let file_data = params.file_data;
  let tool_id = params.tool_id;
  
  // 解码 base64 数据
  let file_bytes = general_purpose::STANDARD.decode(
    file_data.strip_prefix("data:").and_then(|s| s.split(',').nth(1))
      .unwrap_or(&file_data)
  ).map_err(|e| format!("Base64 解码失败: {}", e))?;
  
  // 确定保存目录
  let uploads_dir = if let Some(id) = tool_id {
    // 如果有工具ID，创建子目录
    get_uploads_dir().join(id)
  } else {
    get_uploads_dir()
  };
  
  // 确保目录存在
  std::fs::create_dir_all(&uploads_dir)
    .map_err(|e| format!("创建上传目录失败: {}", e))?;
  
  // 生成安全的文件名（防止路径遍历攻击）
  let safe_file_name = Path::new(&file_name)
    .file_name()
    .and_then(|n| n.to_str())
    .ok_or("无效的文件名")?;
  
  // 如果文件已存在，添加时间戳后缀
  let mut final_path = uploads_dir.join(safe_file_name);
  if final_path.exists() {
    let stem = final_path.file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or("file");
    let extension = final_path.extension()
      .and_then(|s| s.to_str())
      .unwrap_or("");
    let timestamp = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs();
    let new_name = if extension.is_empty() {
      format!("{}_{}", stem, timestamp)
    } else {
      format!("{}_{}.{}", stem, timestamp, extension)
    };
    final_path = uploads_dir.join(new_name);
  }
  
  // 保存文件
  fs::write(&final_path, &file_bytes)
    .map_err(|e| format!("保存文件失败: {}", e))?;
  
  // 确保返回绝对路径
  let abs_path = final_path.canonicalize()
    .unwrap_or(final_path);
  
  // 返回保存后的文件路径（绝对路径）
  Ok(abs_path.to_string_lossy().to_string())
}

/// 解析文件路径为绝对路径
#[tauri::command]
fn resolve_file_path(params: ResolveFilePathParams) -> Result<String, String> {
  let file_path = params.file_path;
  let path = Path::new(&file_path);
  
  // 如果已经是绝对路径，直接规范化
  let abs_path = if path.is_absolute() {
    path.canonicalize()
      .map_err(|e| format!("无法解析路径 {}: {}", file_path, e))?
  } else {
    // 如果是相对路径，尝试从当前工作目录解析
    std::env::current_dir()
      .map_err(|e| format!("获取当前目录失败: {}", e))?
      .join(path)
      .canonicalize()
      .map_err(|e| format!("无法解析路径 {}: {}", file_path, e))?
  };
  
  // 移除 Windows 长路径前缀 (\\?\)，返回标准路径格式
  let path_str = abs_path.to_string_lossy().to_string();
  let clean_path = if path_str.starts_with("\\\\?\\") {
    path_str[4..].to_string()
  } else {
    path_str
  };
  
  Ok(clean_path)
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
      write_categories_config,
      read_config_file,
      write_config_file,
      get_config_file_path,
      config_file_exists,
      launch_tool,
      extract_icon_from_file,
      fetch_favicon,
      upload_file,
      resolve_file_path
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

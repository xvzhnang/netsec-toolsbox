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
  icon_emoji: Option<String>,
  wiki_url: Option<String>,
  tool_type: Option<String>,
  exec_path: Option<String>,
  args: Option<Vec<String>>,
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
fn extract_icon_from_file(file_path: String, tool_type: Option<String>) -> Result<String, String> {
  let path = Path::new(&file_path);
  if !path.exists() {
    return Err(format!("文件不存在: {}", file_path));
  }
  
  let tool_type = tool_type.as_deref().unwrap_or("");
  
  // 检查缓存
  let cache_key = hash_path(&file_path);
  let cache_path = get_icons_dir().join(format!("{}.png", cache_key));
  if cache_path.exists() {
    // 从缓存读取
    match fs::read(&cache_path) {
      Ok(data) => {
        match image::load_from_memory(&data) {
          Ok(img) => {
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
  
  // 根据工具类型提取图标
  let icon_data = match tool_type {
    "LNK" => {
      #[cfg(target_os = "windows")]
      {
        extract_lnk_icon(&file_path)?
      }
      #[cfg(not(target_os = "windows"))]
      {
        return Err("LNK 图标提取仅在 Windows 上支持".to_string());
      }
    }
    "HTML" => {
      extract_html_icon(&file_path)?
    }
    _ => {
      // EXE 或其他可执行文件
      #[cfg(target_os = "windows")]
      {
        extract_exe_icon(&file_path)?
      }
      #[cfg(not(target_os = "windows"))]
      {
        return Err("EXE 图标提取仅在 Windows 上支持".to_string());
      }
    }
  };
  
  // 处理图标尺寸
  let base64 = process_icon_to_base64(icon_data, 160)?;
  
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
  }
  
  Ok(base64)
}

/// 提取 EXE 图标（Windows）
#[cfg(target_os = "windows")]
fn extract_exe_icon(file_path: &str) -> Result<DynamicImage, String> {
  use windows::Win32::System::LibraryLoader::LoadLibraryW;
  use windows::Win32::UI::WindowsAndMessaging::{ExtractIconW, DestroyIcon, GetIconInfo, HICON};
  use windows::Win32::Graphics::Gdi::{GetObjectW, BITMAP};
  use windows::core::PCWSTR;
  use std::ffi::OsStr;
  use std::os::windows::ffi::OsStrExt;
  
  // 转换路径为宽字符串
  let wide_path: Vec<u16> = OsStr::new(file_path).encode_wide().chain(Some(0)).collect();
  
  // 加载库并提取图标
  unsafe {
    let hmod = LoadLibraryW(PCWSTR(wide_path.as_ptr()))
      .map_err(|e| format!("加载库失败: {:?}", e))?;
    
    let hicon: HICON = ExtractIconW(hmod, PCWSTR(wide_path.as_ptr()), 0);
    if hicon.is_invalid() {
      return Err("无法提取图标".to_string());
    }
    
    // 获取图标信息
    let mut icon_info = std::mem::zeroed();
    if !GetIconInfo(hicon, &mut icon_info).as_bool() {
      DestroyIcon(hicon);
      return Err("获取图标信息失败".to_string());
    }
    
    // 获取位图信息
    let mut bmp = BITMAP::default();
    let result = GetObjectW(icon_info.hbmColor, std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp as *const _ as *mut _));
    if result == 0 {
      DestroyIcon(hicon);
      return Err("获取位图信息失败".to_string());
    }
    
    // 创建 DIB 并复制位图数据
    let width = bmp.bmWidth as u32;
    let height = bmp.bmHeight as u32;
    
    // 简化实现：返回一个占位图标
    // 实际实现需要更复杂的位图转换逻辑
    DestroyIcon(hicon);
    
    // 创建一个简单的占位图标
    Ok(DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(width.max(1), height.max(1), image::Rgba([128, 128, 128, 255]))))
  }
}

/// 提取 LNK 图标（Windows）
#[cfg(target_os = "windows")]
fn extract_lnk_icon(lnk_path: &str) -> Result<DynamicImage, String> {
  // LNK 文件指向的目标路径
  // 简化实现：读取 LNK 文件的目标路径，然后提取目标文件的图标
  // 这里先返回占位图标
  extract_exe_icon(lnk_path)
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
fn fetch_favicon(url_str: String) -> Result<String, String> {
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
      fetch_favicon
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;

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
  jar_path: String,
  java_path: Option<String>,
  jvm_args: Option<Vec<String>>,
  program_args: Option<Vec<String>>,
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
fn launch_tool(
  #[serde(alias = "tool_type")] tool_type: Option<String>,
  #[serde(alias = "exec_path")] exec_path: Option<String>,
  args: Option<Vec<String>>,
  #[serde(alias = "working_dir")] working_dir: Option<String>,
  #[serde(alias = "jar_config")] jar_config: Option<JarConfig>,
) -> Result<(), String> {
  let tool_type = tool_type.as_deref().unwrap_or("GUI");
  
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
      launch_tool
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

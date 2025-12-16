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

fn get_config_path(app: &tauri::AppHandle) -> PathBuf {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .expect("failed to get app data dir");
  std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
  app_data_dir.join("categories.json")
}

#[tauri::command]
fn read_categories_config(app: tauri::AppHandle) -> Result<String, String> {
  let config_path = get_config_path(&app);
  if config_path.exists() {
    fs::read_to_string(&config_path)
      .map_err(|e| format!("Failed to read config file: {}", e))
  } else {
    // 返回空JSON，前端会使用默认值
    Ok("{}".to_string())
  }
}

#[tauri::command]
fn write_categories_config(app: tauri::AppHandle, content: String) -> Result<(), String> {
  let config_path = get_config_path(&app);
  fs::write(&config_path, content)
    .map_err(|e| format!("Failed to write config file: {}", e))
}

#[tauri::command]
fn launch_tool(
  tool_type: Option<String>,
  exec_path: String,
  args: Option<Vec<String>>,
  working_dir: Option<String>,
  jar_config: Option<JarConfig>,
) -> Result<(), String> {
  let tool_type = tool_type.as_deref().unwrap_or("GUI");
  let args = args.unwrap_or_default();
  let working_dir = working_dir
    .map(PathBuf::from)
    .or_else(|| {
      // 如果没有指定工作目录，使用可执行文件所在目录
      Path::new(&exec_path)
        .parent()
        .map(|p| p.to_path_buf())
    })
    .ok_or_else(|| "无法确定工作目录".to_string())?;

  match tool_type {
    "Python" => {
      // Python 脚本：在脚本所在目录打开终端，执行 python script.py [args]
      let script_path = Path::new(&exec_path);
      if !script_path.exists() {
        return Err(format!("Python 脚本不存在: {}", exec_path));
      }

      let script_name = script_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "无效的脚本路径".to_string())?;

      // 构建命令：python script.py [args]
      let mut cmd_args = vec![script_name.to_string()];
      cmd_args.extend(args);

      // Windows: 使用 PowerShell 打开终端并执行
      #[cfg(target_os = "windows")]
      {
        let cmd_str = format!(
          "python {}",
          cmd_args
            .iter()
            .map(|a| format!("\"{}\"", a.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ")
        );
        let working_dir_str = working_dir.to_string_lossy().to_string();
        Command::new("powershell")
          .args(&[
            "-NoExit",
            "-Command",
            &format!("cd '{}'; {}", working_dir_str, cmd_str),
          ])
          .spawn()
          .map_err(|e| format!("启动 PowerShell 失败: {}", e))?;
      }

      // Linux/macOS: 使用终端打开并执行
      #[cfg(not(target_os = "windows"))]
      {
        let cmd_str = format!(
          "python3 {}",
          cmd_args
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join(" ")
        );
        let working_dir_str = working_dir.to_string_lossy().to_string();
        Command::new("sh")
          .args(&[
            "-c",
            &format!("cd '{}' && {}; exec $SHELL", working_dir_str, cmd_str),
          ])
          .spawn()
          .map_err(|e| format!("启动终端失败: {}", e))?;
      }
    }
    "CLI" => {
      // CLI 工具：在可执行文件所在目录打开终端，执行可执行文件 [args]
      let exec_path_buf = Path::new(&exec_path);
      if !exec_path_buf.exists() {
        return Err(format!("可执行文件不存在: {}", exec_path));
      }

      // Windows: 使用 PowerShell 打开终端并执行
      #[cfg(target_os = "windows")]
      {
        let cmd_str = format!(
          "\"{}\" {}",
          exec_path.replace('"', "\"\""),
          args.iter()
            .map(|a| format!("\"{}\"", a.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ")
        );
        let working_dir_str = working_dir.to_string_lossy().to_string();
        Command::new("powershell")
          .args(&[
            "-NoExit",
            "-Command",
            &format!("cd '{}'; {}", working_dir_str, cmd_str),
          ])
          .spawn()
          .map_err(|e| format!("启动 PowerShell 失败: {}", e))?;
      }

      // Linux/macOS: 使用终端打开并执行
      #[cfg(not(target_os = "windows"))]
      {
        let cmd_str = format!(
          "'{}' {}",
          exec_path.replace('\'', "'\\''"),
          args.iter()
            .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join(" ")
        );
        let working_dir_str = working_dir.to_string_lossy().to_string();
        Command::new("sh")
          .args(&[
            "-c",
            &format!("cd '{}' && {}; exec $SHELL", working_dir_str, cmd_str),
          ])
          .spawn()
          .map_err(|e| format!("启动终端失败: {}", e))?;
      }
    }
    "JAR" => {
      // JAR 文件：在 JAR 文件所在目录打开终端，执行 java -jar [jvm_args] jar_path [program_args]
      if let Some(jar_config) = jar_config {
        let jar_path = Path::new(&jar_config.jar_path);
        if !jar_path.exists() {
          return Err(format!("JAR 文件不存在: {}", jar_config.jar_path));
        }

        // 确定 Java 可执行文件路径
        let java_exe = jar_config
          .java_path
          .unwrap_or_else(|| "java".to_string());

        // 构建命令：java [jvm_args] -jar jar_path [program_args]
        let mut cmd_parts = vec![java_exe];
        if let Some(jvm_args) = jar_config.jvm_args {
          cmd_parts.extend(jvm_args);
        }
        cmd_parts.push("-jar".to_string());
        cmd_parts.push(jar_config.jar_path);
        if let Some(program_args) = jar_config.program_args {
          cmd_parts.extend(program_args);
        }

        // Windows: 使用 PowerShell 打开终端并执行
        #[cfg(target_os = "windows")]
        {
          let cmd_str = cmd_parts
            .iter()
            .map(|a| format!("\"{}\"", a.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ");
          let working_dir_str = working_dir.to_string_lossy().to_string();
          Command::new("powershell")
            .args(&[
              "-NoExit",
              "-Command",
              &format!("cd '{}'; {}", working_dir_str, cmd_str),
            ])
            .spawn()
            .map_err(|e| format!("启动 PowerShell 失败: {}", e))?;
        }

        // Linux/macOS: 使用终端打开并执行
        #[cfg(not(target_os = "windows"))]
        {
          let cmd_str = cmd_parts
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join(" ");
          let working_dir_str = working_dir.to_string_lossy().to_string();
          Command::new("sh")
            .args(&[
              "-c",
              &format!("cd '{}' && {}; exec $SHELL", working_dir_str, cmd_str),
            ])
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
        }
      } else {
        return Err("JAR 配置缺失".to_string());
      }
    }
    _ => {
      // GUI 或其他类型：直接启动，不需要终端
      let exec_path_buf = Path::new(&exec_path);
      if !exec_path_buf.exists() {
        return Err(format!("可执行文件不存在: {}", exec_path));
      }

      let mut cmd = Command::new(&exec_path);
      if !args.is_empty() {
        cmd.args(&args);
      }
      if let Some(wd) = working_dir.to_str() {
        cmd.current_dir(wd);
      }
      cmd.spawn()
        .map_err(|e| format!("启动程序失败: {}", e))?;
    }
  }

  Ok(())
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
      write_categories_config
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

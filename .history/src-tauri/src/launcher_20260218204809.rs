use crate::types::{JarConfig, LaunchToolParams};
use crate::utils::get_file_dir;
use std::path::Path;
use std::process::Command;

/// 启动 GUI 工具（直接启动，不打开终端）
fn launch_gui_tool(
    exec_path: &str,
    args: Option<Vec<String>>,
    working_dir: Option<String>,
) -> Result<(), String> {
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
    cmd.spawn().map_err(|e| format!("启动工具失败: {}", e))?;

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
        .args(["-Command", &ps_command])
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
        (
            "gnome-terminal",
            vec![
                "--working-directory",
                &working_dir.to_string_lossy(),
                "--",
                "bash",
                "-c",
                &format!("{}; exec bash", command),
            ],
        ),
        (
            "xterm",
            vec![
                "-e",
                "bash",
                "-c",
                &format!(
                    "cd '{}' && {}; exec bash",
                    working_dir.to_string_lossy().replace('\'', "'\\''"),
                    command.replace('\'', "'\\''")
                ),
            ],
        ),
        (
            "konsole",
            vec![
                "--workdir",
                &working_dir.to_string_lossy(),
                "-e",
                "bash",
                "-c",
                &format!("{}; exec bash", command),
            ],
        ),
        (
            "x-terminal-emulator",
            vec![
                "-e",
                "bash",
                "-c",
                &format!(
                    "cd '{}' && {}; exec bash",
                    working_dir.to_string_lossy().replace('\'', "'\\''"),
                    command.replace('\'', "'\\''")
                ),
            ],
        ),
    ];

    for (terminal, args) in terminal_commands {
        if let Ok(mut child) = Command::new(terminal).args(&args).spawn() {
            // 不等待子进程，让它独立运行
            let _ = child.wait();
            return Ok(());
        }
    }

    Err(
        "无法找到可用的终端模拟器（请安装 gnome-terminal、xterm、konsole 或 x-terminal-emulator）"
            .to_string(),
    )
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

/// 在 Windows 上使用 PowerShell 打开终端并执行命令
#[cfg(target_os = "windows")]
fn launch_in_terminal_windows_pwsh(
    working_dir: &Path,
    command: &str,
    env_path: Option<&str>,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    // 构建 PATH 环境变量设置命令
    let env_setup = if let Some(path) = env_path {
        // 使用相对路径设置 PATH
        // $env:PATH = "..\..\.config\env\python\python3;" + $env:PATH
        format!("$env:PATH = '{}' + [System.IO.Path]::PathSeparator + $env:PATH; ", path.replace('\'', "''"))
    } else {
        String::new()
    };

    // 使用 Start-Process 启动新的 PowerShell 窗口，并执行命令
    // -WorkingDirectory 指定工作目录
    // -ArgumentList 指定要执行的命令
    let ps_command = format!(
        "Start-Process powershell -WorkingDirectory '{}' -ArgumentList '-NoExit', '-Command', \"{}{}\"",
        working_dir.to_string_lossy().replace('\'', "''"), // 转义路径中的单引号
        env_setup,
        command.replace('"', "`\"") // 转义 PowerShell 中的双引号
    );

    Command::new("powershell")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .args(["-Command", &ps_command])
        .spawn()
        .map_err(|e| format!("启动终端失败: {}", e))?;

    Ok(())
}

/// 启动 Python 脚本（在对应目录打开终端执行）
fn launch_python_tool(
    exec_path: &str,
    args: Option<Vec<String>>,
    python_env: Option<String>,
) -> Result<(), String> {
    let path = Path::new(exec_path);
    if !path.exists() {
        return Err(format!("文件不存在: {}", exec_path));
    }

    let working_dir = get_file_dir(exec_path);

    #[cfg(target_os = "windows")]
    {
        // 确定 Python 命令和环境路径
        let (python_cmd, env_path_str) = if let Some(env) = python_env {
            // 如果 env 字符串为空，则直接使用系统 python
            if env.trim().is_empty() {
                ("python", None)
            } else {
                // 获取配置目录
                let config_dir = crate::utils::get_config_dir();
                // 构建环境目录绝对路径: .config/env/python/{env}
                let env_dir = config_dir.join("env").join("python").join(&env);
                
                if env_dir.exists() {
                    // 计算相对路径：从 working_dir 到 env_dir
                    let rel_path = crate::utils::get_relative_path(&working_dir, &env_dir);
                    // 使用通用 Python 命令，因为 PATH 已经设置
                    ("python", Some(rel_path.to_string_lossy().to_string()))
                } else {
                    log::warn!("指定的 Python 环境不存在: {}, 降级使用系统环境", env_dir.display());
                    ("python", None)
                }
            }
        } else {
            ("python", None)
        };

        // 构建提示信息，不直接运行
        let file_name = Path::new(exec_path).file_name().unwrap_or_default().to_string_lossy();
        let mut hint_cmd = format!("{} '{}'", python_cmd, file_name);
        
        if let Some(ref args_vec) = args {
            for arg in args_vec {
                // 参数处理
                hint_cmd.push_str(&format!(" '{}'", arg));
            }
        }

        // 使用 Write-Host 输出提示信息
        // 注意：PowerShell 中单引号字符串内不能直接包含单引号，需要用两个单引号转义
        let command = format!(
            "Write-Host 'Python 环境已配置 (Environment Configured).' -ForegroundColor Green; Write-Host '您可以执行以下命令来运行脚本 (Run the following command):'; Write-Host '{}' -ForegroundColor Yellow", 
            hint_cmd.replace('\'', "''")
        );
        
        launch_in_terminal_windows_pwsh(&working_dir, &command, env_path_str.as_deref())?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let python_cmd = "python3";
        let file_name = Path::new(exec_path).file_name().unwrap_or_default().to_string_lossy();
        
        // 构建提示命令
        let mut hint_cmd = format!("{} \"{}\"", python_cmd, file_name);
        if let Some(ref args_vec) = args {
            for arg in args_vec {
                if arg.contains(' ') || arg.contains('\'') || arg.contains('"') {
                    hint_cmd.push_str(&format!(" \"{}\"", arg.replace('"', "\\\"")));
                } else {
                    hint_cmd.push_str(&format!(" {}", arg));
                }
            }
        }
        
        // 在 Unix 上，我们使用 echo 打印提示信息
        let command = format!("echo 'Python environment configured.'; echo 'Run the following command:'; echo '{}'", hint_cmd.replace('\'', "'\\''"));
        
        launch_in_terminal_unix(&working_dir, &command)?;
    }

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
    let mut command = java_cmd.to_string();

    // 添加 JVM 参数
    if let Some(ref jvm_args) = jar_config.jvm_args {
        for arg in jvm_args {
            command.push_str(&format!(" {}", arg));
        }
    }

    // 添加 -jar 和 JAR 路径
    command.push_str(&format!(
        " -jar \"{}\"",
        jar_config.jar_path.replace('"', "\\\"")
    ));

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
    let mut cmd = Command::new("cmd");
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
    cmd.args(["/C", "start", "", exec_path])
        .spawn()
        .map_err(|e| format!("启动快捷方式失败: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn launch_lnk_tool(_exec_path: &str) -> Result<(), String> {
    Err("LNK 工具仅在 Windows 系统上支持".to_string())
}

/// 打开 URL 在默认浏览器中（用于网页工具）
#[tauri::command]
pub fn open_url_in_browser(url: String) -> Result<(), String> {
    // 验证 URL 格式
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!("无效的 URL 格式: {}", url));
    }

    // 在默认浏览器中打开
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 start 命令打开默认浏览器
        let mut cmd = Command::new("cmd");
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
        cmd.args(["/C", "start", "", url.as_str()])
            .spawn()
            .map_err(|e| format!("打开浏览器失败: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: 使用 open 命令
        Command::new("open")
            .arg(url.as_str())
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
        let mut cmd = Command::new("cmd");
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
        cmd.args(["/C", "start", "", &file_url])
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
pub fn launch_tool(params: LaunchToolParams) -> Result<(), String> {
    let tool_type = params.tool_type.as_deref().unwrap_or("GUI");
    let exec_path = params.exec_path;
    let args = params.args;
    let working_dir = params.working_dir;
    let jar_config = params.jar_config;
    let python_env = params.python_env;

    // 调试日志（开发环境）
    #[cfg(debug_assertions)]
    {
        log::info!(
      "启动工具: type={:?} (len={}), exec_path={:?}, args={:?}, working_dir={:?}, jar_config={:?}, python_env={:?}",
      tool_type,
      tool_type.len(),
      exec_path,
      args,
      working_dir,
      jar_config,
      python_env
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
            launch_python_tool(&exec_path, args, python_env)
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
            open_url_in_browser(exec_path)
        }
        _ => Err(format!("不支持的工具类型: {}", tool_type)),
    }
}

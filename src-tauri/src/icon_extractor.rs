use std::fs;
use std::path::Path;
use std::process::Command;
use regex::Regex;
use image::{DynamicImage, GenericImageView};
use base64::{Engine as _, engine::general_purpose};
use crate::types::ExtractIconParams;
use crate::utils::{get_icons_dir, hash_path};

/// 将图标转换为统一尺寸的 PNG base64
/// 容错处理：不假设输入图像的尺寸，总是调整到目标尺寸
fn process_icon_to_base64(img: DynamicImage, size: u32) -> Result<String, String> {
  // 获取实际尺寸
  let (actual_width, actual_height) = img.dimensions();
  
  // 调整尺寸为正方形（如果尺寸不匹配）
  let resized = if actual_width == size && actual_height == size {
    // 尺寸已经正确，直接使用
    img
  } else {
    // 需要调整尺寸
    log::debug!("调整图标尺寸: {}x{} -> {}x{}", actual_width, actual_height, size, size);
    img.resize_exact(size, size, image::imageops::FilterType::Lanczos3)
  };
  
  // 转换为 RGBA（容错处理）
  let rgba = resized.to_rgba8();
  
  // 验证尺寸（容错，不要 panic）
  let (rgba_width, rgba_height) = rgba.dimensions();
  if rgba_width != size || rgba_height != size {
    return Err(format!(
      "图标尺寸验证失败: 期望 {}x{}，实际 {}x{}",
      size, size, rgba_width, rgba_height
    ));
  }
  
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

/// 根据文件路径自动判断文件类型
fn detect_file_type_from_path(file_path: &str) -> String {
  if file_path.is_empty() {
    return "其他".to_string();
  }
  
  // 检查是否是 URL
  if file_path.starts_with("http://") || file_path.starts_with("https://") {
    return "网页".to_string();
  }
  
  // 提取文件扩展名
  let path = Path::new(file_path);
  if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
    let ext_lower = ext.to_lowercase();
    match ext_lower.as_str() {
      "exe" | "dll" | "com" | "bat" | "cmd" | "scr" => "GUI".to_string(),
      "lnk" => "LNK".to_string(),
      "html" | "htm" => "HTML".to_string(),
      "py" | "pyw" => "Python".to_string(),
      "jar" => "JAR".to_string(),
      "sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "psd1" => "CLI".to_string(),
      _ => "其他".to_string(),
    }
  } else {
    "其他".to_string()
  }
}

/// 提取 EXE 图标（Windows）
/// 使用 PowerShell/.NET 的 System.Drawing.Icon::ExtractAssociatedIcon 提取图标
/// 支持 EXE、DLL、LNK 等文件类型
/// 参考代码：resolve_file_icon_base64
#[cfg(target_os = "windows")]
fn extract_exe_icon(file_path: &str) -> Result<DynamicImage, String> {
  // 移除 Windows 长路径前缀 (\\?\)，因为 PowerShell 可能不支持
  let clean_path = if file_path.starts_with("\\\\?\\") {
    &file_path[4..]
  } else {
    file_path
  };
  
  log::info!("提取 EXE 图标，清理后的路径: {}", clean_path);
  
  // 检查文件是否存在（参考代码中的检查）
  let path = Path::new(clean_path);
  if !path.exists() {
    return Err(format!("文件不存在: {}", clean_path));
  }
  
  // 使用环境变量传递路径，避免 PowerShell 脚本中的路径转义问题
  // 这样可以正确处理包含中文、空格、特殊字符的路径
  let script = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
try {
  $filePath = $env:ICON_FILE_PATH
  if ([string]::IsNullOrEmpty($filePath)) {
    Write-Error "环境变量 ICON_FILE_PATH 未设置"
    exit 1
  }
  if (-not (Test-Path $filePath)) {
    Write-Error "文件不存在: $filePath"
    exit 1
  }
  Add-Type -AssemblyName System.Drawing
  $ic = [System.Drawing.Icon]::ExtractAssociatedIcon($filePath)
  if ($ic -ne $null) {
    $bmp = $ic.ToBitmap()
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    [Convert]::ToBase64String($ms.ToArray())
  } else {
    Write-Error "无法提取图标"
    exit 1
  }
} catch {
  Write-Error $_.Exception.Message
  exit 1
}
"#;
  
  // 使用环境变量传递路径，避免编码和转义问题
  let output = Command::new("powershell")
    .arg("-NoProfile")
    .arg("-ExecutionPolicy")
    .arg("Bypass")
    .arg("-Command")
    .arg(script)
    .env("ICON_FILE_PATH", clean_path)
    .output()
    .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
  
  // 按照参考代码的方式处理输出
  if output.status.success() {
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
  } else {
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("PowerShell 提取图标失败: {}", stderr))
  }
}

/// 提取 LNK 图标（Windows）
/// 使用 PowerShell/.NET 解析快捷方式并提取图标
/// 优先使用快捷方式的 IconLocation，如果没有则使用目标路径的图标
/// 参考代码：resolve_lnk_icon_base64
#[cfg(target_os = "windows")]
fn extract_lnk_icon(lnk_path: &str) -> Result<DynamicImage, String> {
  // 移除 Windows 长路径前缀 (\\?\)，因为 PowerShell 可能不支持
  let clean_path = if lnk_path.starts_with("\\\\?\\") {
    &lnk_path[4..]
  } else {
    lnk_path
  };
  
  log::info!("提取 LNK 图标，清理后的路径: {}", clean_path);
  
  // 检查文件是否存在（参考代码中的检查）
  let path = Path::new(clean_path);
  if !path.exists() {
    return Err(format!("LNK 文件不存在: {}", clean_path));
  }
  
  // 使用环境变量传递路径，避免 PowerShell 脚本中的路径转义问题
  // 这样可以正确处理包含中文、空格、特殊字符的路径
  let script = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
try {
  $lnkPath = $env:ICON_LNK_PATH
  if ([string]::IsNullOrEmpty($lnkPath)) {
    Write-Error "环境变量 ICON_LNK_PATH 未设置"
    exit 1
  }
  $s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnkPath)
  $icon = $s.IconLocation
  if ([string]::IsNullOrEmpty($icon)) {
    $icon = $s.TargetPath
  }
  if ([string]::IsNullOrEmpty($icon)) {
    Write-Error "无法获取图标路径"
    exit 1
  }
  $parts = $icon -split ','
  $iconFile = $parts[0].Trim()
  if (-not (Test-Path $iconFile)) {
    Write-Error "图标文件不存在: $iconFile"
    exit 1
  }
  Add-Type -AssemblyName System.Drawing
  $ic = [System.Drawing.Icon]::ExtractAssociatedIcon($iconFile)
  if ($ic -ne $null) {
    $bmp = $ic.ToBitmap()
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    [Convert]::ToBase64String($ms.ToArray())
  } else {
    Write-Error "无法提取图标"
    exit 1
  }
} catch {
  Write-Error $_.Exception.Message
  exit 1
}
"#;
  
  // 使用环境变量传递路径，避免编码和转义问题
  let output = Command::new("powershell")
    .arg("-NoProfile")
    .arg("-ExecutionPolicy")
    .arg("Bypass")
    .arg("-Command")
    .arg(script)
    .env("ICON_LNK_PATH", clean_path)
    .output()
    .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
  
  // 按照参考代码的方式处理输出
  if output.status.success() {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let base64_str = stdout.trim();
    
    // 解码 base64 并转换为图片
    let image_bytes = general_purpose::STANDARD.decode(base64_str)
      .map_err(|e| format!("Base64 解码失败: {}", e))?;
    
    image::load_from_memory(&image_bytes)
      .map_err(|e| format!("加载图片失败: {}", e))
  } else {
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("PowerShell 提取 LNK 图标失败: {}", stderr))
  }
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

/// 从文件提取图标（EXE/LNK/HTML）
#[tauri::command]
pub fn extract_icon_from_file(params: ExtractIconParams) -> Result<String, String> {
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
  let mut file_path_str = abs_path.to_string_lossy().to_string();
  if file_path_str.starts_with("\\\\?\\") {
    file_path_str = file_path_str[4..].to_string();
  }
  
  if !abs_path.exists() {
    return Err(format!("文件不存在: {}", file_path_str));
  }
  
  // 如果没有提供工具类型，根据文件路径自动判断
  let tool_type = if let Some(tt) = tool_type.as_deref() {
    if tt.is_empty() {
      detect_file_type_from_path(&file_path_str)
    } else {
      tt.to_string()
    }
  } else {
    detect_file_type_from_path(&file_path_str)
  };
  
  log::info!("检测到的文件类型: {}", tool_type);
  
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
  let icon_data = match tool_type.as_str() {
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
    "GUI" | "CLI" | "Python" | "JAR" | "其他" => {
      // EXE、CLI、Python、JAR 或其他可执行文件
      #[cfg(target_os = "windows")]
      {
        log::info!("提取可执行文件图标: {}", file_path_str);
        extract_exe_icon(&file_path_str)?
      }
      #[cfg(not(target_os = "windows"))]
      {
        return Err("可执行文件图标提取仅在 Windows 上支持".to_string());
      }
    }
    "网页" => {
      // 网页类型应该使用 fetch_favicon 命令，这里不应该被调用
      return Err("网页类型应使用 fetch_favicon 命令".to_string());
    }
    _ => {
      // 未知类型，尝试作为可执行文件提取
      #[cfg(target_os = "windows")]
      {
        log::info!("未知类型，尝试作为可执行文件提取图标: {}", file_path_str);
        extract_exe_icon(&file_path_str)?
      }
      #[cfg(not(target_os = "windows"))]
      {
        return Err(format!("不支持的文件类型: {}", tool_type));
      }
    }
  };
  
  // 处理图标尺寸（先调整尺寸，确保是 160x160）
  // 不要假设图标是特定尺寸，Windows 可能返回任意尺寸的图标
  let resized_icon = icon_data.resize_exact(160, 160, image::imageops::FilterType::Lanczos3);
  let base64 = process_icon_to_base64(resized_icon.clone(), 160)?;
  
  log::info!("图标提取成功: file_path={}, base64_length={}", file_path_str, base64.len());
  
  // 保存到缓存（使用调整后的图标数据，确保尺寸正确）
  let rgba = resized_icon.to_rgba8();
  // 验证尺寸（容错处理，不要 assert）
  let (actual_width, actual_height) = rgba.dimensions();
  if actual_width != 160 || actual_height != 160 {
    log::warn!(
      "图标尺寸不匹配: 期望 160x160，实际 {}x{}，将调整尺寸",
      actual_width,
      actual_height
    );
    // 重新调整尺寸
    let resized = resized_icon.resize_exact(160, 160, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8();
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
  } else {
    // 尺寸正确，直接保存
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
  log::info!("图标缓存已保存: {}", cache_path.to_string_lossy());
  
  Ok(base64)
}

/// 从 URL 抓取 favicon
#[tauri::command]
pub fn fetch_favicon(params: crate::types::FetchFaviconParams) -> Result<String, String> {
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
  
  // 尝试下载 favicon
  for favicon_url in favicon_paths {
    match reqwest::blocking::get(&favicon_url) {
      Ok(response) => {
        if response.status().is_success() {
          match response.bytes() {
            Ok(bytes) => {
              match image::load_from_memory(&bytes) {
                Ok(img) => {
                  // 处理图标尺寸（确保是 160x160）
                  let resized_img = img.resize_exact(160, 160, image::imageops::FilterType::Lanczos3);
                  let base64 = process_icon_to_base64(resized_img.clone(), 160)?;
                  
                  // 保存到缓存（使用调整后的图像）
                  let rgba = resized_img.to_rgba8();
                  let (rgba_width, rgba_height) = rgba.dimensions();
                  if rgba_width == 160 && rgba_height == 160 {
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
                  } else {
                    log::warn!(
                      "Favicon 尺寸不匹配: 期望 160x160，实际 {}x{}，跳过缓存",
                      rgba_width,
                      rgba_height
                    );
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
  
  // 如果所有路径都失败，返回默认图标
  Ok(process_icon_to_base64(
    DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(160, 160, image::Rgba([200, 200, 200, 255]))),
    160
  )?)
}


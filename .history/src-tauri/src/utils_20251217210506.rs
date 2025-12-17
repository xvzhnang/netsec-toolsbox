use std::path::PathBuf;
use sha2::{Sha256, Digest};

/// 获取配置目录路径
pub fn get_config_dir() -> PathBuf {
  // 使用当前工作目录下的 .config 文件夹
  let current_dir = std::env::current_dir()
    .expect("failed to get current directory");
  let config_dir = current_dir.join(".config");
  
  // 确保 .config 目录存在
  std::fs::create_dir_all(&config_dir)
    .expect("failed to create .config directory");
  
  config_dir
}

/// 获取图标缓存目录路径
pub fn get_icons_dir() -> PathBuf {
  let icons_dir = get_config_dir().join("icons");
  std::fs::create_dir_all(&icons_dir)
    .expect("failed to create icons directory");
  icons_dir
}

/// 获取上传文件目录路径
pub fn get_uploads_dir() -> PathBuf {
  let uploads_dir = get_config_dir().join("uploads");
  std::fs::create_dir_all(&uploads_dir)
    .expect("failed to create uploads directory");
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

/// 获取 Wiki 目录路径（在同级目录下，不在 .config 下）
pub fn get_wiki_dir() -> PathBuf {
  // 使用当前工作目录下的 wiki 文件夹
  let current_dir = std::env::current_dir()
    .expect("failed to get current directory");
  let wiki_dir = current_dir.join("wiki");
  
  // 确保 wiki 目录存在
  std::fs::create_dir_all(&wiki_dir)
    .expect("failed to create wiki directory");
  
  wiki_dir
}


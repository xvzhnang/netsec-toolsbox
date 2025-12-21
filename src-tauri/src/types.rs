use serde::{Deserialize, Serialize};

/// 分类配置
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoriesConfig {
    pub categories: Vec<CategoryConfig>,
}

/// 单个分类配置
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryConfig {
    pub id: String,
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub icon: String,
    pub color: String,
    pub order: i32,
    pub enabled: bool,
}

/// 分类数据
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoriesData {
    pub categories: Vec<CategoryPageData>,
}

/// 分类页面数据
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryPageData {
    pub id: String,
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub sub_categories: Vec<SubCategory>,
}

/// 子分类
#[derive(Debug, Serialize, Deserialize)]
pub struct SubCategory {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tools: Vec<ToolItem>,
}

/// 工具项
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(alias = "iconUrl", alias = "icon_url")]
    pub icon_url: Option<String>,
    #[serde(alias = "wikiUrl", alias = "wiki_url")]
    pub wiki_url: Option<String>,
    #[serde(alias = "toolType", alias = "tool_type")]
    pub tool_type: Option<String>,
    #[serde(alias = "execPath", alias = "exec_path")]
    pub exec_path: Option<String>,
    pub args: Option<Vec<String>>,
    #[serde(alias = "workingDir", alias = "working_dir")]
    pub working_dir: Option<String>,
}

/// JAR 配置
#[derive(Debug, Serialize, Deserialize)]
pub struct JarConfig {
    #[serde(alias = "jarPath", alias = "jar_path")]
    pub jar_path: String,
    #[serde(alias = "javaPath", alias = "java_path")]
    pub java_path: Option<String>,
    #[serde(alias = "jvmArgs", alias = "jvm_args")]
    pub jvm_args: Option<Vec<String>>,
    #[serde(alias = "programArgs", alias = "program_args")]
    pub program_args: Option<Vec<String>>,
}

/// 启动工具的参数结构体（支持 camelCase 和 snake_case）
#[derive(Debug, Deserialize)]
pub struct LaunchToolParams {
    #[serde(alias = "toolType", alias = "tool_type")]
    pub tool_type: Option<String>,
    #[serde(alias = "execPath", alias = "exec_path")]
    pub exec_path: Option<String>,
    pub args: Option<Vec<String>>,
    #[serde(alias = "workingDir", alias = "working_dir")]
    pub working_dir: Option<String>,
    #[serde(alias = "jarConfig", alias = "jar_config")]
    pub jar_config: Option<JarConfig>,
}

/// 提取图标的参数结构体（支持 camelCase 和 snake_case）
#[derive(Debug, Deserialize)]
pub struct ExtractIconParams {
    #[serde(alias = "filePath", alias = "file_path")]
    pub file_path: String,
    #[serde(alias = "toolType", alias = "tool_type")]
    pub tool_type: Option<String>,
}

/// 获取 favicon 的参数结构体（支持 camelCase 和 snake_case）
#[derive(Debug, Deserialize)]
pub struct FetchFaviconParams {
    #[serde(alias = "urlStr", alias = "url_str")]
    pub url_str: String,
}

/// 上传文件的参数结构体（支持 camelCase 和 snake_case）
#[derive(Debug, Deserialize)]
pub struct UploadFileParams {
    #[serde(alias = "fileName", alias = "file_name")]
    pub file_name: String,
    #[serde(alias = "fileData", alias = "file_data")]
    pub file_data: String, // base64 编码的文件数据
    #[serde(alias = "toolId", alias = "tool_id")]
    pub tool_id: Option<String>, // 可选的工具ID，用于组织文件
}

/// 解析文件路径的参数结构体
#[derive(Debug, Deserialize)]
pub struct ResolveFilePathParams {
    #[serde(alias = "filePath", alias = "file_path")]
    pub file_path: String,
}

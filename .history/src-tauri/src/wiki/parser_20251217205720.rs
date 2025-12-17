use std::fs;
use std::path::Path;
use crate::wiki::types::{RenderResult, TocItem};
use pulldown_cmark::{Parser, Options, html};
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::util::LinesWithEndings;

/// Markdown 解析器
pub struct MarkdownParser {
  syntax_set: SyntaxSet,
  theme_set: ThemeSet,
}

impl MarkdownParser {
  pub fn new() -> Self {
    Self {
      syntax_set: SyntaxSet::load_defaults_newlines(),
      theme_set: ThemeSet::load_defaults(),
    }
  }

  /// 渲染 Markdown 文件为 HTML
  pub fn render_file(&self, file_path: &Path) -> Result<RenderResult, String> {
    let content = fs::read_to_string(file_path)
      .map_err(|e| format!("读取文件失败: {}", e))?;
    
    self.render(&content)
  }

  /// 渲染 Markdown 内容为 HTML
  pub fn render(&self, markdown: &str) -> Result<RenderResult, String> {
    // 提取标题
    let title = extract_title(markdown);
    
    // 解析 Markdown
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    
    let parser = Parser::new_ext(markdown, options);
    
    // 提取目录结构
    let toc = extract_toc(parser.clone());
    
    // 处理代码块高亮
    let processed_markdown = highlight_code_blocks(markdown, &self.syntax_set, &self.theme_set);
    
    // 重新解析处理后的 Markdown
    let parser = Parser::new_ext(&processed_markdown, options);
    
    // 转换为 HTML
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    Ok(RenderResult {
      html: html_output,
      toc: Some(toc),
      title,
    })
  }
}

/// 提取文档标题（从第一个一级标题或文件名）
fn extract_title(markdown: &str) -> String {
  for line in markdown.lines() {
    let trimmed = line.trim();
    if trimmed.starts_with("# ") {
      return trimmed[2..].trim().to_string();
    }
  }
  "Wiki".to_string()
}

/// 提取目录结构
fn extract_toc(parser: Parser) -> Vec<TocItem> {
  // 简化实现：直接从文本中提取
  // 实际应该从 Parser 事件中提取，但需要更复杂的实现
  Vec::new()
}

/// 从文本中提取目录
fn extract_toc_from_text(text: &str) -> Vec<TocItem> {
  let mut toc: Vec<TocItem> = Vec::new();
  let mut stack: Vec<&mut TocItem> = Vec::new();
  
  for line in text.lines() {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
      let level = trimmed.chars().take_while(|&c| c == '#').count() as u32;
      if level >= 1 && level <= 6 {
        let text = trimmed[level..].trim().to_string();
        let id = generate_anchor_id(&text);
        
        let item = TocItem {
          id,
          text,
          level,
          children: Vec::new(),
        };
        
        // 找到合适的父节点
        while let Some(parent) = stack.last_mut() {
          if parent.level < level {
            parent.children.push(item);
            if let Some(last) = parent.children.last_mut() {
              stack.push(last);
            }
            break;
          } else {
            stack.pop();
          }
        }
        
        if stack.is_empty() {
          toc.push(item);
          if let Some(last) = toc.last_mut() {
            stack.push(last);
          }
        }
      }
    }
  }
  
  toc
}

/// 生成锚点 ID
fn generate_anchor_id(text: &str) -> String {
  text
    .to_lowercase()
    .chars()
    .map(|c| match c {
      'a'..='z' | '0'..='9' => c,
      ' ' | '-' | '_' => '-',
      _ => '-',
    })
    .collect::<String>()
    .trim_matches('-')
    .to_string()
}

/// 高亮代码块
fn highlight_code_blocks(
  markdown: &str,
  syntax_set: &SyntaxSet,
  theme_set: &ThemeSet,
) -> String {
  // 使用 Monokai 主题
  let theme = theme_set.themes.get("Monokai").unwrap_or_else(|| {
    theme_set.themes.values().next().unwrap()
  });
  
  let mut result = String::new();
  let mut in_code_block = false;
  let mut code_language = String::new();
  let mut code_content = String::new();
  
  for line in markdown.lines() {
    if line.starts_with("```") {
      if in_code_block {
        // 结束代码块，进行高亮
        let highlighted = highlight_code(&code_content, &code_language, syntax_set, theme);
        result.push_str(&format!("<pre><code class=\"language-{}\">{}</code></pre>\n", code_language, highlighted));
        code_content.clear();
        code_language.clear();
        in_code_block = false;
      } else {
        // 开始代码块
        code_language = line[3..].trim().to_string();
        in_code_block = true;
      }
    } else if in_code_block {
      code_content.push_str(line);
      code_content.push('\n');
    } else {
      result.push_str(line);
      result.push('\n');
    }
  }
  
  // 如果还有未结束的代码块
  if in_code_block && !code_content.is_empty() {
    let highlighted = highlight_code(&code_content, &code_language, syntax_set, theme);
    result.push_str(&format!("<pre><code class=\"language-{}\">{}</code></pre>\n", code_language, highlighted));
  }
  
  result
}

/// 高亮代码
fn highlight_code(
  code: &str,
  language: &str,
  syntax_set: &SyntaxSet,
  theme: &syntect::highlighting::Theme,
) -> String {
  // 根据语言查找语法
  let syntax = syntax_set.find_syntax_by_extension(language)
    .or_else(|| syntax_set.find_syntax_by_name(language))
    .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
  
  let mut highlighter = HighlightLines::new(syntax, theme);
  let mut highlighted = String::new();
  
  for line in LinesWithEndings::from(code) {
    let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, syntax_set)
      .unwrap_or_default();
    
    for (style, text) in ranges {
      let (r, g, b) = style.foreground.rgb;
      highlighted.push_str(&format!(
        "<span style=\"color: #{:02x}{:02x}{:02x}\">{}</span>",
        r, g, b,
        html_escape(text)
      ));
    }
  }
  
  highlighted
}

/// HTML 转义
fn html_escape(text: &str) -> String {
  text
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&#39;")
}


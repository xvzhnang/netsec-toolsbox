#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Wiki HTTP 服务器
使用 Flask 提供 Markdown 渲染和文件服务
"""

import os
import sys
import json
import re
from pathlib import Path
from typing import Optional, List, Dict, Any
from flask import Flask, send_from_directory, jsonify, request, Response
from flask_cors import CORS
import markdown
from markdown.extensions import codehilite, tables, fenced_code, toc
from pygments import highlight
from pygments.lexers import get_lexer_by_name, guess_lexer_for_filename
from pygments.formatters import HtmlFormatter
from pygments.util import ClassNotFound

# 配置
PORT = 8777
HOST = "127.0.0.1"

app = Flask(__name__)
CORS(app)  # 允许跨域请求

# Markdown 扩展配置
md_extensions = [
    'codehilite',
    'fenced_code',
    'tables',
    'toc',
    'nl2br',
    'sane_lists',
]

# 代码高亮样式
codehilite_config = {
    'use_pygments': True,
    'css_class': 'highlight',
    'linenums': False,
}

def get_wiki_dir() -> Path:
    """获取 Wiki 根目录路径"""
    # 尝试从环境变量获取，否则使用脚本所在目录下的 wiki 文件夹
    if 'WIKI_DIR' in os.environ:
        wiki_dir = Path(os.environ['WIKI_DIR'])
    else:
        # 脚本在项目根目录，wiki 目录也在根目录
        wiki_dir = Path(__file__).parent / "wiki"
    wiki_dir.mkdir(exist_ok=True)
    return wiki_dir

def get_docs_dir() -> Path:
    """获取 Wiki 文档目录路径（wiki/docs/）"""
    docs_dir = get_wiki_dir() / "docs"
    docs_dir.mkdir(exist_ok=True)
    return docs_dir

def get_theme_dir() -> Path:
    """获取主题目录路径（wiki/themes/）"""
    theme_dir = get_wiki_dir() / "themes"
    theme_dir.mkdir(exist_ok=True)
    return theme_dir

def extract_title(markdown_content: str) -> str:
    """从 Markdown 内容提取标题"""
    lines = markdown_content.strip().split('\n')
    for line in lines:
        line = line.strip()
        if line.startswith('# '):
            return line[2:].strip()
        elif line.startswith('## '):
            return line[3:].strip()
    # 如果没有找到标题，使用文件名
    return "Wiki 文档"

def list_wiki_files(base_dir: Path, current_dir: Path = None) -> List[Dict[str, Any]]:
    """递归列出 Wiki 文件（从 docs 目录）"""
    if current_dir is None:
        current_dir = base_dir
    
    files = []
    
    try:
        for item in sorted(current_dir.iterdir()):
            if item.name.startswith('.'):
                continue
            
            if item.is_dir():
                # 递归处理子目录
                children = list_wiki_files(base_dir, item)
                files.append({
                    'path': str(item.relative_to(base_dir)).replace('\\', '/'),
                    'name': item.name,
                    'title': item.name,
                    'is_dir': True,
                    'children': children if children else None,
                })
            elif item.suffix == '.md':
                # 读取文件第一行作为标题
                try:
                    with open(item, 'r', encoding='utf-8') as f:
                        first_line = f.readline().strip()
                        title = extract_title(first_line) if first_line.startswith('#') else item.stem
                except:
                    title = item.stem
                
                files.append({
                    'path': str(item.relative_to(base_dir)).replace('\\', '/'),
                    'name': item.name,
                    'title': title,
                    'is_dir': False,
                    'children': None,
                })
    except Exception as e:
        print(f"Error listing files: {e}", file=sys.stderr)
    
    return files

def render_markdown(file_path: Path) -> Dict[str, Any]:
    """渲染 Markdown 文件为 HTML"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        return {'error': f'读取文件失败: {e}'}
    
    # 提取标题
    title = extract_title(content)
    
    # 配置 Markdown
    md = markdown.Markdown(
        extensions=md_extensions,
        extension_configs={
            'codehilite': codehilite_config,
        }
    )
    
    # 渲染 HTML
    html = md.convert(content)
    
    # 提取目录
    toc_html = md.toc if hasattr(md, 'toc') else ''
    toc_items = parse_toc(toc_html) if toc_html else []
    
    return {
        'html': html,
        'title': title,
        'toc': toc_items,
    }

def parse_toc(toc_html: str) -> List[Dict[str, Any]]:
    """解析目录 HTML 为结构化数据"""
    if not toc_html:
        return []
    
    items = []
    # 简单的目录解析（可以根据需要改进）
    # 这里返回空列表，前端会从 HTML 中提取
    return items

def search_wiki_files(query: str, base_dir: Path) -> List[Dict[str, Any]]:
    """搜索 Wiki 文件"""
    results = []
    query_lower = query.lower()
    
    for md_file in base_dir.rglob('*.md'):
        try:
            with open(md_file, 'r', encoding='utf-8') as f:
                content = f.read()
            
            if query_lower in content.lower():
                title = extract_title(content)
                results.append({
                    'file_path': str(md_file.relative_to(base_dir)).replace('\\', '/'),
                    'title': title,
                })
        except:
            continue
    
    return results

def load_theme(theme_name: Optional[str] = None) -> str:
    """加载主题 CSS（支持 Typora 主题直接使用）"""
    theme_dir = get_theme_dir()
    
    if not theme_name or theme_name == 'default':
        # 尝试从配置文件读取
        config_file = theme_dir.parent / "theme_config.json"
        if config_file.exists():
            try:
                with open(config_file, 'r', encoding='utf-8') as f:
                    config = json.load(f)
                    theme_name = config.get('theme', 'default')
            except:
                theme_name = 'default'
        else:
            theme_name = 'default'
    
    # 尝试加载指定的主题文件（支持 Typora 主题，直接使用 CSS 文件名）
    theme_file = theme_dir / f"{theme_name}.css"
    if theme_file.exists():
        try:
            with open(theme_file, 'r', encoding='utf-8') as f:
                return f.read()
        except Exception as e:
            print(f"加载主题文件失败: {e}", file=sys.stderr)
            pass
    
    # 如果指定的主题不存在，尝试加载 default.css
    default_theme = theme_dir / "default.css"
    if default_theme.exists():
        try:
            with open(default_theme, 'r', encoding='utf-8') as f:
                return f.read()
        except:
            pass
    
    return ""

def get_available_themes() -> List[str]:
    """获取可用主题列表"""
    theme_dir = get_theme_dir()
    themes = []
    
    if theme_dir.exists():
        for theme_file in theme_dir.glob("*.css"):
            themes.append(theme_file.stem)
    
    if not themes:
        themes = ["default"]
    else:
        themes.sort()
        if "default" not in themes:
            themes.insert(0, "default")
    
    return themes

# API 路由

@app.route('/')
def index():
    """Wiki 首页"""
    docs_dir = get_docs_dir()
    theme_name = request.args.get('theme', 'default')
    theme_css = load_theme(theme_name)
    
    # 读取首页 HTML 模板
    script_dir = Path(__file__).parent
    index_file = script_dir / "src-tauri" / "static" / "wiki_index.html"
    if not index_file.exists():
        # 尝试其他可能的位置
        index_file = script_dir / "static" / "wiki_index.html"
    
    if index_file.exists():
        try:
            with open(index_file, 'r', encoding='utf-8') as f:
                html = f.read()
        except Exception as e:
            print(f"读取首页模板失败: {e}", file=sys.stderr)
            html = get_default_index_html()
    else:
        html = get_default_index_html()
    
    # 注入主题样式
    if theme_css:
        html = html.replace('</head>', f'<style>{theme_css}</style></head>')
    
    return html

@app.route('/api/files')
def api_files():
    """获取文件列表"""
    docs_dir = get_docs_dir()
    files = list_wiki_files(docs_dir)
    return jsonify(files)

@app.route('/api/render')
def api_render():
    """渲染 Markdown 文件"""
    file_path = request.args.get('path')
    if not file_path:
        return jsonify({'error': '缺少 path 参数'}), 400
    
    docs_dir = get_docs_dir()
    full_path = docs_dir / file_path
    
    if not full_path.exists() or not full_path.is_file():
        return jsonify({'error': '文件不存在'}), 404
    
    result = render_markdown(full_path)
    if 'error' in result:
        return jsonify(result), 500
    
    return jsonify(result)

@app.route('/api/tree')
def api_tree():
    """获取目录树"""
    docs_dir = get_docs_dir()
    files = list_wiki_files(docs_dir)
    return jsonify(files)

@app.route('/api/search')
def api_search():
    """搜索 Wiki"""
    query = request.args.get('q')
    if not query:
        return jsonify([])
    
    docs_dir = get_docs_dir()
    results = search_wiki_files(query, docs_dir)
    return jsonify(results)

@app.route('/api/themes')
def api_themes():
    """获取可用主题列表"""
    themes = get_available_themes()
    return jsonify(themes)

@app.route('/file/<path:file_path>')
def file_handler(file_path: str):
    """处理文件请求"""
    docs_dir = get_docs_dir()
    full_path = docs_dir / file_path
    
    if not full_path.exists():
        return "文件不存在", 404
    
    # 如果是 Markdown 文件，渲染为 HTML
    if full_path.suffix == '.md':
        theme_name = request.args.get('theme')
        result = render_markdown(full_path)
        
        if 'error' in result:
            return result['error'], 500
        
        # 加载主题
        theme_css = load_theme(theme_name)
        
        # 获取文件树和目录
        files = list_wiki_files(docs_dir, docs_dir)
        toc_html = generate_toc_html(result.get('toc', []))
        file_tree_html = generate_file_tree_html(files)
        theme_selector_html = generate_theme_selector_html(theme_name)
        
        # 读取样式文件
        script_dir = Path(__file__).parent
        styles_file = script_dir / "src-tauri" / "static" / "wiki_styles.css"
        if not styles_file.exists():
            styles_file = script_dir / "static" / "wiki_styles.css"
        
        styles = ""
        if styles_file.exists():
            try:
                with open(styles_file, 'r', encoding='utf-8') as f:
                    styles = f.read()
            except Exception as e:
                print(f"读取样式文件失败: {e}", file=sys.stderr)
        
        # 生成完整 HTML
        html = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{result['title']}</title>
  <style>{styles}</style>
  {f'<style>{theme_css}</style>' if theme_css else ''}
</head>
<body>
  <div class="wiki-container">
    <aside class="wiki-sidebar">
      <div class="wiki-sidebar-header">
        <h2><a href="/" style="color: inherit; text-decoration: none;">Wiki</a></h2>
        {theme_selector_html}
        <button class="wiki-search-btn" onclick="toggleSearch()">🔍 搜索</button>
      </div>
      <div id="wiki-search" class="wiki-search" style="display: none;">
        <input type="text" id="search-input" placeholder="搜索 Wiki..." onkeyup="performSearch(event)">
        <div id="search-results"></div>
      </div>
      <div class="wiki-file-tree">
        <h3>文件导航</h3>
        {file_tree_html}
      </div>
      <div class="wiki-toc-section">
        <h3>页面目录</h3>
        {toc_html}
      </div>
    </aside>
    <main class="wiki-content">
      <article class="markdown-body">
        {result['html']}
      </article>
    </main>
  </div>
  <script>
    function toggleSearch() {{
      const search = document.getElementById('wiki-search');
      search.style.display = search.style.display === 'none' ? 'block' : 'none';
      if (search.style.display === 'block') {{
        document.getElementById('search-input').focus();
      }}
    }}
    
    async function performSearch(event) {{
      if (event.key === 'Enter' || event.keyCode === 13) {{
        const query = event.target.value;
        if (!query.trim()) {{
          document.getElementById('search-results').innerHTML = '';
          return;
        }}
        try {{
          const response = await fetch(`/api/search?q=${{encodeURIComponent(query)}}`);
          const results = await response.json();
          let html = '<ul class="search-results-list">';
          for (const result of results) {{
            html += `<li><a href="/file/${{result.file_path}}">${{result.title}}</a></li>`;
          }}
          html += '</ul>';
          document.getElementById('search-results').innerHTML = html;
        }} catch (error) {{
          document.getElementById('search-results').innerHTML = '<p>搜索失败</p>';
        }}
      }}
    }}
    
    function changeTheme(themeName) {{
      const url = new URL(window.location.href);
      if (themeName && themeName !== 'default') {{
        url.searchParams.set('theme', themeName);
      }} else {{
        url.searchParams.delete('theme');
      }}
      window.location.href = url.toString();
    }}
    
    async function loadThemes() {{
      try {{
        const response = await fetch('/api/themes');
        const themes = await response.json();
        const themeSelect = document.getElementById('theme-select');
        if (themeSelect) {{
          themeSelect.innerHTML = '';
          const urlParams = new URLSearchParams(window.location.search);
          const currentTheme = urlParams.get('theme') || 'default';
          for (const theme of themes) {{
            const option = document.createElement('option');
            option.value = theme;
            option.textContent = theme === 'default' ? '默认主题' : theme.replace(/_/g, ' ').replace(/-/g, ' ');
            if (theme === currentTheme) {{
              option.selected = true;
            }}
            themeSelect.appendChild(option);
          }}
        }}
      }} catch (error) {{
        console.error('加载主题列表失败:', error);
      }}
    }}
    
    document.addEventListener('DOMContentLoaded', function() {{
      loadThemes();
      
      const savedTheme = localStorage.getItem('wiki-theme');
      if (savedTheme) {{
        const urlParams = new URLSearchParams(window.location.search);
        if (!urlParams.has('theme')) {{
          changeTheme(savedTheme);
        }}
      }}
      
      const themeSelect = document.getElementById('theme-select');
      if (themeSelect) {{
        themeSelect.addEventListener('change', function() {{
          localStorage.setItem('wiki-theme', this.value);
        }});
      }}
      
      // 为所有标题添加锚点
      document.querySelectorAll('h1, h2, h3, h4, h5, h6').forEach((heading, index) => {{
        const id = heading.textContent?.toLowerCase().replace(/[^a-z0-9]+/g, '-') || `heading-${{index}}`;
        heading.id = id;
      }});
    }});
  </script>
</body>
</html>"""
        
        return html
    else:
        # 其他文件直接返回
        return send_from_directory(str(full_path.parent), full_path.name)

def generate_toc_html(toc_items: List[Dict[str, Any]]) -> str:
    """生成目录 HTML"""
    # 目录会通过 JavaScript 从 HTML 中自动提取，这里返回占位符
    # 实际目录会在前端通过 JavaScript 动态生成
    return '<nav class="wiki-toc"><p>页面加载后自动生成目录</p></nav>'

def generate_file_tree_html(files: List[Dict[str, Any]], level: int = 0) -> str:
    """生成文件树 HTML（支持折叠）"""
    html = '<ul class="wiki-tree-list">'
    for file in files:
        if file['is_dir']:
            dir_id = f"dir-{file['path'].replace('/', '-').replace('\\', '-')}"
            has_children = file.get('children') and len(file['children']) > 0
            toggle_class = 'wiki-tree-toggle' if has_children else 'wiki-tree-toggle-empty'
            html += f'''<li class="wiki-tree-dir">
                <span class="{toggle_class}" onclick="toggleDir('{dir_id}')" {'style="cursor: pointer;"' if has_children else ''}>
                    {'▼' if has_children else '▶'} 📁 {file["name"]}
                </span>
                <div id="{dir_id}" class="wiki-tree-children" style="display: {'block' if has_children else 'none'};">
                    {generate_file_tree_html(file['children'], level + 1) if has_children else ''}
                </div>
            </li>'''
        else:
            html += f'<li class="wiki-tree-file"><a href="/file/{file["path"]}">📄 {file["title"]}</a></li>'
    html += '</ul>'
    return html

def get_default_index_html() -> str:
    """获取默认首页 HTML"""
    return """<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Wiki</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
            max-width: 800px;
            margin: 40px auto;
            padding: 20px;
            line-height: 1.6;
        }
        h1 { color: #24292e; }
        ul { list-style: none; padding: 0; }
        li { margin: 8px 0; }
        a { color: #0366d6; text-decoration: none; }
        a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <h1>Wiki</h1>
    <p>欢迎使用 Wiki</p>
    <p>请在 wiki 目录下创建 Markdown 文件。</p>
</body>
</html>"""

def generate_theme_selector_html(current_theme: Optional[str] = None) -> str:
    """生成主题选择器 HTML"""
    themes = get_available_themes()
    if len(themes) <= 1:
        return ""
    
    options = ""
    for theme in themes:
        selected = ' selected' if (current_theme == theme or (current_theme is None and theme == 'default')) else ''
        options += f'<option value="{theme}"{selected}>{theme if theme != "default" else "默认主题"}</option>'
    
    return f"""<div class="wiki-theme-selector" style="margin-top: 12px;">
    <label for="theme-select" style="display: block; font-size: 12px; color: #586069; margin-bottom: 4px;">主题:</label>
    <select id="theme-select" onchange="changeTheme(this.value)" style="width: 100%; padding: 6px; border: 1px solid #d1d5db; border-radius: 4px; font-size: 13px; background: white;">
      {options}
    </select>
  </div>"""

if __name__ == '__main__':
    # 确保 Wiki 目录存在
    wiki_dir = get_wiki_dir()
    theme_dir = get_theme_dir()
    
    print(f"Wiki 服务器启动在 http://{HOST}:{PORT}", file=sys.stderr)
    print(f"Wiki 目录: {wiki_dir}", file=sys.stderr)
    print(f"主题目录: {theme_dir}", file=sys.stderr)
    
    # 使用 stderr 输出，避免干扰 HTTP 响应
    sys.stderr.flush()
    
    app.run(host=HOST, port=PORT, debug=False, use_reloader=False)


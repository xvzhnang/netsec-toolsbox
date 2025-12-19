# Markdown 渲染问题分析与修复方案

## 问题分析

### 1. Mermaid 流程图解析错误

**问题现象：**
```
Parse error on line 1:
flowchart TD    A[开
^
Expecting 'NEWLINE', 'SPACE', 'GRAPH', got 'ALPHA'
```

**根本原因：**
1. **markdown-it-mermaid 插件处理问题**：插件可能将代码块转换为 `<div class="mermaid">` 时，丢失了换行符
2. **文本提取问题**：从 DOM 提取文本时，`textContent` 在某些情况下可能丢失换行符
3. **代码清理问题**：清理函数可能在规范化过程中移除了必要的换行符

**修复方案：**
- 改进文本提取函数，确保保留换行符
- 在清理函数中，强制在流程图声明后添加换行符
- 在渲染前再次清理，确保格式正确

### 2. KaTeX 公式渲染不完整

**问题现象：**
- 块级公式（`$$...$$`）渲染不完整或不显示
- 行内公式（`$...$`）可能被误识别

**根本原因：**
1. **markdown-it-katex 插件配置问题**：delimiters 配置可能不正确
2. **异步渲染问题**：KaTeX 库可能未完全加载就开始渲染
3. **DOM 更新时序问题**：公式元素可能在 DOM 更新前就被处理

**修复方案：**
- 确保 markdown-it-katex 正确配置 delimiters
- 使用 `Promise.resolve().then()` 确保异步渲染
- 添加回退机制，手动查找并渲染未处理的公式

### 3. 代码高亮不生效

**问题现象：**
- 代码块没有语法高亮
- highlight.js 主题颜色不显示

**根本原因：**
1. **highlight 函数返回格式问题**：返回的 HTML 格式可能不正确
2. **highlight.js 加载时序问题**：highlight 函数执行时，hljs 可能未加载
3. **语言别名问题**：某些语言别名（如 `ps1`）不被 highlight.js 支持

**修复方案：**
- 确保 highlight 函数返回正确的 HTML 格式
- 添加语言别名映射
- 使用 `highlightElement` 作为回退方案

### 4. 插件加载顺序问题

**问题现象：**
- 某些插件功能冲突
- 渲染结果不正确

**根本原因：**
1. **插件依赖关系**：某些插件必须在其他插件之前加载
2. **代码块处理冲突**：mermaid 和 highlight 插件可能冲突

**修复方案：**
- 确保正确的插件加载顺序：
  1. mermaid（必须在最前面，拦截 mermaid 代码块）
  2. anchor, toc, taskLists, attrs, footnote, emoji
  3. container
  4. katex（在最后，避免与其他插件冲突）

## 修复实施

### 修复 1: Mermaid 文本提取和清理

**关键改进：**
1. 使用 `extractTextWithNewlines` 函数确保保留换行符
2. 在清理函数中，强制在 `flowchart TD` 或 `graph LR` 后添加换行符
3. 支持中文字符（使用 `[^\]]*` 匹配方括号内容）

### 修复 2: KaTeX 渲染优化

**关键改进：**
1. 确保 markdown-it-katex 正确配置
2. 添加手动查找和渲染未处理公式的回退机制
3. 使用异步渲染避免阻塞

### 修复 3: 代码高亮修复

**关键改进：**
1. 确保 highlight 函数返回正确的 HTML 格式
2. 添加语言别名映射（ps1 -> powershell）
3. 使用 `highlightElement` 作为回退

### 修复 4: 插件顺序优化

**关键改进：**
1. 确保 mermaid 插件在最前面
2. 确保 katex 插件在最后
3. 其他插件按依赖关系排序


# Markdown 渲染问题修复说明

## 问题分析与修复

### 1. Mermaid 流程图解析错误

#### 问题原因
- **markdown-it-mermaid 插件处理问题**：插件将代码块转换为 `<div class="mermaid">` 时，可能丢失换行符
- **文本提取问题**：从 DOM 提取文本时，`textContent` 在某些情况下可能丢失换行符
- **代码清理问题**：清理函数可能在规范化过程中移除了必要的换行符

#### 修复方案

**1.1 改进文本提取函数 (`extractTextWithNewlines`)**
```typescript
// 关键改进：
// - 对于 <pre> 和 <code> 元素，优先使用 textContent（保留换行符）
// - 对于 <div> 元素，如果 textContent 没有换行符，从 innerHTML 提取
// - 将 <br> 标签转换为换行符
```

**为什么有效：**
- `<pre>` 元素的 `textContent` 会保留所有换行符和空格
- 如果 markdown-it-mermaid 生成了 `<div>` 且丢失了换行符，可以从 `innerHTML` 恢复
- 确保无论插件如何生成 HTML，都能正确提取原始文本

**1.2 强制添加换行符 (`cleanMermaidCode`)**
```typescript
// 关键修复：确保流程图声明后必须有换行符
cleaned = cleaned.replace(/(flowchart\s+[A-Z]{2})\s+([A-Z]\[[^\]]*)/gi, '$1\n$2')
cleaned = cleaned.replace(/(graph\s+[A-Z]{2})\s+([A-Z]\[[^\]]*)/gi, '$1\n$2')
```

**为什么有效：**
- 正则表达式 `/(flowchart\s+[A-Z]{2})\s+([A-Z]\[[^\]]*)/gi` 匹配：
  - `flowchart TD` 或 `graph LR`（第一部分）
  - 多个空格（`\s+`）
  - 节点定义如 `A[开始]`（第二部分，`[^\]]*` 匹配方括号内的所有内容，包括中文）
- 替换为 `$1\n$2`，强制在声明和节点之间添加换行符
- 无论原始格式如何，都能确保 Mermaid 解析器收到正确的格式

**1.3 渲染前再次清理**
```typescript
// 在渲染前，再次应用清理函数，确保格式正确
content = finalCleanMermaidCode(content)
node.textContent = content
```

**为什么有效：**
- 双重保险：即使第一次清理失败，第二次清理也能修复
- 确保传递给 Mermaid 的内容格式完全正确

### 2. KaTeX 公式渲染不完整

#### 问题原因
1. **markdown-it-katex 插件可能未处理所有公式**：某些格式的公式可能被遗漏
2. **异步渲染时序问题**：KaTeX 库可能未完全加载就开始渲染
3. **DOM 更新时序问题**：公式元素可能在 DOM 更新前就被处理

#### 修复方案

**2.1 改进公式查找和渲染逻辑**
```typescript
// 第一步：处理 markdown-it-katex 已生成的元素
const katexElements = container.querySelectorAll('.katex-display, .katex-block, .katex-inline, .katex:not(.katex-display):not(.katex-block)')

// 第二步：查找并渲染未被处理的公式
// 使用 TreeWalker 遍历所有文本节点，查找 $$...$$ 和 $...$ 模式
```

**为什么有效：**
- **两步处理**：先处理插件已生成的元素，再查找遗漏的公式
- **TreeWalker**：遍历所有文本节点，确保不遗漏任何公式
- **跳过代码块**：避免处理代码块内的 `$` 符号

**2.2 使用异步渲染**
```typescript
Promise.resolve().then(() => {
  // 渲染逻辑
})
```

**为什么有效：**
- 确保在 DOM 完全更新后再渲染
- 避免阻塞主线程
- 给 KaTeX 库足够的时间加载

**2.3 验证公式有效性**
```typescript
// 验证是否像数学公式（包含数学符号）
const hasMathSymbols = /[+\-*/=()\[\]{},.^_\\]/.test(formula)
if (formula.length >= 2 && hasMathSymbols) {
  // 渲染
}
```

**为什么有效：**
- 避免误识别普通文本中的 `$` 符号
- 确保只渲染真正的数学公式

### 3. 代码高亮不生效

#### 问题原因
1. **highlight 函数返回格式问题**：返回的 HTML 格式可能不正确
2. **highlight.js 加载时序问题**：highlight 函数执行时，hljs 可能未加载
3. **语言别名问题**：某些语言别名（如 `ps1`）不被 highlight.js 支持

#### 修复方案

**3.1 确保返回正确的 HTML 格式**
```typescript
// markdown-it 会自动包装在 <pre><code> 中
// 所以我们只返回 <code> 标签内的内容
return `<code class="hljs language-${normalizedLang}">${result.value}</code>`
```

**为什么有效：**
- markdown-it 的 highlight 函数应该返回 `<code>` 标签内的 HTML
- markdown-it 会自动添加 `<pre>` 和 `<code>` 外层标签
- 必须包含 `hljs` 类和 `language-xxx` 类，这样：
  - highlight.js 的 CSS 主题才能应用
  - 后续的 `highlightElement` 也能识别

**3.2 添加语言别名映射**
```typescript
const langMap = {
  'ps1': 'powershell',  // ps1 -> powershell
  'pwsh': 'powershell', // pwsh -> powershell
  'ps': 'powershell',   // ps -> powershell
  'powershell': 'powershell',
  'shell': 'bash',
  'sh': 'bash',
  'zsh': 'bash',
}
```

**为什么有效：**
- highlight.js 支持 `powershell` 但不支持 `ps1`
- 通过映射，确保所有 PowerShell 相关别名都能正确高亮

**3.3 使用 highlightElement 作为回退**
```typescript
// 如果 highlight 函数返回 null，markdown-it 会生成 <pre><code class="language-xxx">
// 后续可以通过 highlightElement 处理
hljs.highlightElement(codeElement as HTMLElement)
```

**为什么有效：**
- 双重保障：即使 highlight 函数失败，highlightElement 也能处理
- highlightElement 会自动识别 `language-xxx` 类并应用高亮

**3.4 添加调试日志**
```typescript
console.warn('[markdown-it] highlight.js 未加载，代码块将使用默认样式')
console.warn('[markdown-it] 语言不支持:', lang, '代码块将使用默认样式，后续可通过 highlightElement 处理')
```

**为什么有效：**
- 帮助开发者快速定位问题
- 明确说明回退机制

### 4. 插件加载顺序优化

#### 问题原因
- **插件依赖关系**：某些插件必须在其他插件之前加载
- **代码块处理冲突**：mermaid 和 highlight 插件可能冲突

#### 修复方案

**正确的插件加载顺序：**
```typescript
// 1. mermaid（必须在最前面，拦截 mermaid 代码块）
md.use(mermaidPlugin)

// 2. anchor, toc, taskLists, attrs, footnote, emoji
md.use(anchor, {...})
md.use(toc, {...})
md.use(taskLists, {...})
md.use(attrs, {...})
md.use(footnote)
md.use(emoji)

// 3. container（自定义容器）
md.use(container, 'tip')
md.use(container, 'info')
// ...

// 4. katex（在最后，避免与其他插件冲突）
md.use(katex, {
  delimiters: [...],
  strict: false  // 确保不处理代码块内的公式
})
```

**为什么有效：**
1. **mermaid 在最前面**：
   - 必须在 highlight 函数之前，这样 highlight 函数可以返回 `null` 让 mermaid 插件处理
   - 如果 mermaid 在后面，highlight 函数可能已经处理了代码块

2. **katex 在最后**：
   - 避免与其他插件冲突
   - 确保所有文本处理完成后再处理公式
   - `strict: false` 确保不处理代码块内的 `$` 符号

3. **其他插件顺序**：
   - anchor 和 toc 可以互相配合
   - attrs 应该在大多数插件之后，以便处理属性
   - container 应该在 attrs 之后，以便支持属性语法

### 5. 支持中文节点

#### 修复方案

**5.1 正则表达式支持中文**
```typescript
// [^\]]* 匹配方括号内的所有内容，包括中文字符
cleaned = cleaned.replace(/(flowchart\s+[A-Z]{2})\s+([A-Z]\[[^\]]*)/gi, '$1\n$2')
```

**为什么有效：**
- `[^\]]*` 匹配任何非 `]` 的字符，包括：
  - 中文字符（如"开始"、"登录"）
  - 英文、数字、符号
- 确保中文节点名称不会被截断或丢失

**5.2 Mermaid 配置支持中文**
```typescript
mermaid.initialize({
  theme: 'dark',
  securityLevel: 'loose',
  flowchart: {
    useMaxWidth: true,
    htmlLabels: true, // 确保 HTML 标签正确渲染，支持中文节点
  },
})
```

**为什么有效：**
- `htmlLabels: true` 允许 Mermaid 使用 HTML 标签渲染节点文本
- 这样可以正确显示中文字符，避免编码问题

## 总结

### 关键修复点

1. **Mermaid**：
   - 改进文本提取，确保保留换行符
   - 强制在流程图声明后添加换行符
   - 支持中文字符（使用 `[^\]]*` 匹配）

2. **KaTeX**：
   - 两步处理：先处理插件生成的元素，再查找遗漏的公式
   - 使用 TreeWalker 遍历所有文本节点
   - 验证公式有效性，避免误识别

3. **代码高亮**：
   - 确保返回正确的 HTML 格式（包含 `hljs` 和 `language-xxx` 类）
   - 添加语言别名映射
   - 使用 `highlightElement` 作为回退

4. **插件顺序**：
   - mermaid 在最前面
   - katex 在最后
   - 其他插件按依赖关系排序

### 为什么这些修复有效

1. **保留原始格式**：通过改进文本提取，确保 Mermaid 代码的原始格式（特别是换行符）不被破坏

2. **强制规范化**：即使原始格式有问题，清理函数也能修复，确保 Mermaid 解析器收到正确格式

3. **双重保障**：使用两步处理（插件处理 + 手动查找），确保所有公式都能渲染

4. **正确的插件顺序**：避免插件冲突，确保每个插件都能正确处理其负责的内容

5. **回退机制**：即使主要方法失败，也有回退方案（如 `highlightElement`）

这些修复确保了：
- ✅ Mermaid 流程图能正确解析，支持中文节点
- ✅ KaTeX 公式能完整渲染（块级和行内）
- ✅ 代码高亮正常工作，支持多种语言
- ✅ 所有插件按正确顺序加载，避免冲突


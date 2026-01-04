
import MarkdownIt from 'markdown-it';
import markdownItAttrs from 'markdown-it-attrs';
import { full as markdownItEmoji } from 'markdown-it-emoji';
import markdownItAnchor from 'markdown-it-anchor';
import markdownItTocDoneRight from 'markdown-it-toc-done-right';
import markdownItContainer from 'markdown-it-container';
import markdownItTaskLists from 'markdown-it-task-lists';
import markdownItTexmath from 'markdown-it-texmath';
import katex from 'katex';

console.log("Initializing MarkdownIt...");

const markdown = new MarkdownIt({
  html: true,
  linkify: true,
  breaks: true,
});

markdown.use(markdownItTexmath, {
    engine: katex,
    delimiters: 'dollars',
    katexOptions: { macros: { "\\RR": "\\mathbb{R}" } }
});
markdown.use(markdownItTaskLists, {
    enabled: true,
    label: true,
    labelAfter: true,
});
markdown.use(markdownItAttrs);
markdown.use(markdownItEmoji);
markdown.use(markdownItAnchor, {
    permalink: markdownItAnchor.permalink.headerLink()
});
markdown.use(markdownItTocDoneRight, {
    listType: 'ul',
    level: [1, 2, 3],
});

const containers = ['info', 'warning', 'danger', 'tip', 'success', 'details'];
containers.forEach(name => {
    markdown.use(markdownItContainer, name, {
        validate: (params) => params.trim().match(new RegExp(`^${name}\\s*(.*)$`)),
        render: (tokens, idx) => {
            const m = tokens[idx].info.trim().match(new RegExp(`^${name}\\s*(.*)$`));
            if (tokens[idx].nesting === 1) {
                return `<div class="${name} custom-block">\n` +
                       (m && m[1] ? `<p class="custom-block-title">${markdown.utils.escapeHtml(m[1])}</p>\n` : '');
            } else {
                return '</div>\n';
            }
        }
    });
});

const input = "**Bold Text**";
const output = markdown.render(input);

console.log("Input:", input);
console.log("Output:", output);

if (output.includes("\\**")) {
    console.error("BUG REPRODUCED: Output contains escaped stars!");
    process.exit(1);
} else {
    console.log("SUCCESS: Output appears to be correct HTML.");
}

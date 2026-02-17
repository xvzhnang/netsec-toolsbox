mermaid.initialize({ startOnLoad: false });

document$.subscribe(() => {
  mermaid.init(undefined, document.querySelectorAll('.mermaid'));
});

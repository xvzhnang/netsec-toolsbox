window.MathJax = {
  tex: {
    inlineMath: [['$', '$'], ['\\(', '\\)']]
  },
  chtml: {
    fontURL: 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/output/chtml/fonts/woff-v2'
  }
};

document$.subscribe(() => {
  if (window.MathJax) {
    MathJax.typesetPromise();
  }
});

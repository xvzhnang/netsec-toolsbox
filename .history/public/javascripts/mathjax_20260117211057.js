window.MathJax = {
  tex: {
    inlineMath: [['$', '$'], ['\\(', '\\)']]
  }
};

document$.subscribe(() => {
  if (window.MathJax) {
    MathJax.typesetPromise();
  }
});

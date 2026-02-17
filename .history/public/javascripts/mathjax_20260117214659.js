window.MathJax = {
  tex: {
    inlineMath: [['$', '$'], ['\\(', '\\)']]
  },
  chtml: {
    fontURL: 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/output/chtml/fonts/woff-v2'
  },
  startup: {
    typeset: false,
    ready: () => {
      MathJax.startup.defaultReady()

      const safeTypeset = async () => {
        try {
          MathJax.typesetClear()
          await MathJax.typesetPromise()
        } catch (_err) {
        }
      }

      const subscribe = window.document$?.subscribe
      if (typeof subscribe === 'function') {
        window.document$.subscribe(() => {
          setTimeout(() => {
            safeTypeset()
          }, 0)
        })
      }

      setTimeout(() => {
        safeTypeset()
      }, 0)
    }
  },
};

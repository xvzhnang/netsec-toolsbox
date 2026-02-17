window.MathJax = {
  tex: {
    inlineMath: [['$', '$'], ['\\(', '\\)']]
  },
  svg: {
    fontCache: 'global'
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
          }, 50)
        })
      }

      setTimeout(() => {
        safeTypeset()
      }, 50)
    }
  },
};

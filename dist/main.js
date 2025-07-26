function highlight_all() {
  if (window.hljs) {
    window.hljs.highlightAll();
    document.querySelectorAll('pre').forEach(el => {
      if (window.autoScroll) {
        window.autoScroll(el);
      }
    });
  }
}
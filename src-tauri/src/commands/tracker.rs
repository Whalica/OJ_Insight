pub(crate) const TRACKER_INIT_SCRIPT: &str = r#"
(() => {
  if (window.location.origin === 'https://cftracker.netlify.app') {
    const handle = new URLSearchParams(window.location.search).get('oji_handle');
    if (handle) {
      try {
        const state = JSON.parse(window.localStorage.getItem('statev2') || '{}');
        const oldList = state.userList && typeof state.userList === 'object' ? state.userList : {};
        state.userList = { ...oldList, handles: [handle], error: '', id: oldList.id || 0 };
        window.localStorage.setItem('statev2', JSON.stringify(state));
      } catch (_) {}
    }
  }
  const trackerHosts = new Set(['cftracker.netlify.app', 'kenkoooo.com']);
  if (trackerHosts.has(window.location.hostname)) {
    const shouldOpenOutside = (value) => {
      try {
        const url = new URL(value, window.location.href);
        const host = url.hostname.replace(/^www\./, '');
        if (host === 'codeforces.com') {
          return /^\/(contest|gym)\/\d+(?:\/problem\/[^/]+)?\/?$/i.test(url.pathname)
            || /^\/problemset\/problem\/\d+\/[^/]+\/?$/i.test(url.pathname);
        }
        if (host === 'atcoder.jp') {
          return /^\/contests\/[^/]+(?:\/tasks\/[^/]+)?\/?$/i.test(url.pathname);
        }
        return false;
      } catch (_) { return false; }
    };
    const openOutside = (url) => {
      if (typeof url !== 'string' || !shouldOpenOutside(url)) return false;
      window.top.postMessage({ type: 'oj-insight:open-external', url: new URL(url, window.location.href).href }, '*');
      return true;
    };
    const originalOpen = window.open.bind(window);
    window.open = (url, target, features) => {
      if (openOutside(typeof url === 'string' ? url : '')) return null;
      return originalOpen(url, target, features);
    };
    window.addEventListener('click', (event) => {
      const target = event.target;
      const anchor = target && target.closest ? target.closest('a[href]') : null;
      if (!anchor || !anchor.href) return;
      if (!openOutside(anchor.href)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
    }, true);
  }
})();
"#;

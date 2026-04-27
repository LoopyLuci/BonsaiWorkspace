export type ActivityPayload = {
  level?: 'debug' | 'info' | 'warn' | 'error';
  category?: 'tool' | 'swarm' | 'chat' | 'system' | 'terminal' | 'ui';
  source?: string;
  summary?: string;
  details?: unknown;
};

// Sanitize activity details to avoid circular refs, DOM nodes, and huge strings
function sanitizeDetail(value: any): any {
  const seen = new WeakSet();
  function _sanitize(v: any): any {
    if (v === null || v === undefined) return v;
    const t = typeof v;
    if (t === 'string') return v.length > 2000 ? v.slice(0, 2000) + '...[truncated]' : v;
    if (t === 'number' || t === 'boolean') return v;
    if (t === 'function') return `[Function ${v.name || 'anonymous'}]`;
    if (v instanceof Date) return v.toISOString();
    try {
      if (typeof Element !== 'undefined' && v instanceof Element) {
        return `[DOM Element: ${v.tagName}${v.id ? '#' + v.id : ''}]`;
      }
    } catch {
      // ignore cross-realm
    }
    if (t === 'object') {
      if (seen.has(v)) return '[Circular]';
      seen.add(v);
      if (Array.isArray(v)) return v.slice(0, 20).map(_sanitize);
      const out: Record<string, any> = {};
      for (const k of Object.keys(v)) {
        try {
          out[k] = _sanitize((v as any)[k]);
        } catch {
          out[k] = String((v as any)[k]);
        }
      }
      return out;
    }
    try {
      return String(v);
    } catch {
      return '[Unserializable]';
    }
  }

  try {
    return _sanitize(value);
  } catch {
    try { return String(value); } catch { return '[Unserializable]'; }
  }
}

export function pushActivity(payload: ActivityPayload) {
  try {
    const safePayload = { ...payload, details: sanitizeDetail(payload.details) } as ActivityPayload;
    const ev = new CustomEvent('bonsai-activity', { detail: safePayload });
    window.dispatchEvent(ev);
  } catch (e) {
    // best-effort — swallow errors to avoid breaking UI
    // fallback: try console log
    // eslint-disable-next-line no-console
    console.warn('[bonsai] pushActivity failed', e, payload);
  }
}

// Instrument common UI actions so they're captured in the Activity Log.
export function instrumentUserActions() {
  if ((window as any).__bonsai_user_actions_installed) return;
  (window as any).__bonsai_user_actions_installed = true;

  function findActionFromEvent(ev: Event): { label: string | null; el: Element | null } {
    try {
      // Prefer composedPath for shadow DOM friendliness
      const path: any[] = (ev as any).composedPath ? (ev as any).composedPath() : [];
      if (Array.isArray(path) && path.length) {
        for (const node of path) {
          if (!node || (node as Node).nodeType !== 1) continue;
          const el = node as Element;
          try {
            const attrAction = el.getAttribute && el.getAttribute('data-bonsai-action');
            const datasetAction = (el as HTMLElement).dataset?.bonsaiAction;
            const action = attrAction || datasetAction;
            if (action) return { label: action, el };
          } catch {}
          try {
            const aria = (el as HTMLElement).getAttribute?.('aria-label');
            if (aria) return { label: aria, el };
          } catch {}
          try {
            if (el.matches && el.matches('button, a, [role="button"], input[type="button"], input[type="submit"]')) {
              const txt = (el as HTMLElement).textContent;
              if (txt) return { label: txt.trim().slice(0, 120), el };
            }
          } catch {}
        }
      }

      // Fallback: walk up from event.target through parentElement
      let node: any = ev.target as any;
      if (!node) return { label: null, el: null };
      let el: Element | null = node.nodeType === 1 ? node as Element : node.parentElement;
      while (el) {
        try {
          const attrAction = el.getAttribute && el.getAttribute('data-bonsai-action');
          const datasetAction = (el as HTMLElement).dataset?.bonsaiAction;
          const action = attrAction || datasetAction;
          if (action) return { label: action, el };
        } catch {}
        try {
          const aria = (el as HTMLElement).getAttribute?.('aria-label');
          if (aria) return { label: aria, el };
        } catch {}
        try {
          if (el.matches && el.matches('button, a, [role="button"], input[type="button"], input[type="submit"]')) {
            const txt = (el as HTMLElement).textContent;
            if (txt) return { label: txt.trim().slice(0, 120), el };
          }
        } catch {}
        el = el.parentElement;
      }
    } catch {
      // ignore
    }
    return { label: null, el: null };
  }

  window.addEventListener('click', (ev) => {
    try {
      const found = findActionFromEvent(ev as Event);
      const label = found.label ?? 'ui-click';
      // try to identify a logical element for metadata
      const targetEl = found.el ?? ((ev.target as Element)?.closest?.('button, a, [role="button"], input[type="button"], input[type="submit"]') as HTMLElement | null) ?? (ev.target as Element | null);
      const details = {
        tag: (targetEl?.tagName ?? (ev.target as Element | null)?.tagName ?? '').toLowerCase(),
        id: targetEl?.id || null,
        classes: typeof targetEl?.className === 'string' ? targetEl.className : null,
        dataset: targetEl ? { ...targetEl.dataset } : {},
      };
      pushActivity({ level: 'info', category: 'ui', source: 'user', summary: String(label), details });
    } catch (e) {
      // ignore instrumentation errors
    }
  }, true);
}

// Instrument network activity (fetch and WebSocket) and report to Activity Log
export function instrumentNetwork() {
  if ((window as any).__bonsai_network_installed) return;
  (window as any).__bonsai_network_installed = true;

  // Wrap fetch
  try {
    const originalFetch = window.fetch.bind(window);
    // eslint-disable-next-line func-names
    (window as any).fetch = async function (input: RequestInfo, init?: RequestInit) {
      const url = typeof input === 'string' ? input : (input && (input as Request).url) || String(input);
      const method = (init && init.method) || (typeof input !== 'string' && (input as Request).method) || 'GET';
      const start = performance.now();
      pushActivity({ level: 'debug', category: 'system', source: 'fetch', summary: `fetch ${method} ${url}`, details: { method, url } });
      try {
        const res = await originalFetch(input as any, init as any);
        const duration = performance.now() - start;
        pushActivity({ level: res.ok ? 'info' : 'warn', category: 'system', source: 'fetch', summary: `${method} ${url} -> ${res.status}`, details: { method, url, status: res.status, duration } });
        return res;
      } catch (err) {
        const duration = performance.now() - start;
        pushActivity({ level: 'error', category: 'system', source: 'fetch', summary: `${method} ${url} failed`, details: { method, url, duration, error: String(err) } });
        throw err;
      }
    } as typeof fetch;
  } catch (e) {
    // ignore if fetch cannot be patched
  }

  // Wrap WebSocket
  try {
    const NativeWS = (window as any).WebSocket;
    // eslint-disable-next-line @typescript-eslint/ban-types
    const WrappedWS: any = function (this: any, url: any, protocols?: any) {
      const instance = protocols ? new NativeWS(url, protocols) : new NativeWS(url);
      try {
        const u = String(url);
        pushActivity({ level: 'info', category: 'system', source: 'ws', summary: `ws connect ${u}` });
        instance.addEventListener('open', () => pushActivity({ level: 'info', category: 'system', source: 'ws', summary: `ws open ${u}` }));
        instance.addEventListener('message', (ev: any) => {
          let preview: any = '';
          try { preview = typeof ev.data === 'string' ? ev.data.slice(0, 500) : (ev.data instanceof Blob ? 'binary' : String(ev.data)); } catch { preview = '[unreadable]'; }
          pushActivity({ level: 'debug', category: 'system', source: 'ws', summary: `ws message ${u}`, details: { preview } });
        });
        instance.addEventListener('close', (ev: any) => pushActivity({ level: ev && ev.wasClean ? 'info' : 'warn', category: 'system', source: 'ws', summary: `ws close ${u}`, details: { code: ev?.code, reason: ev?.reason } }));
        instance.addEventListener('error', () => pushActivity({ level: 'error', category: 'system', source: 'ws', summary: `ws error ${u}` }));
      } catch { /* ignore */ }
      // proxy send to capture outbound messages
      const origSend = instance.send.bind(instance);
      instance.send = (data: any) => {
        try { pushActivity({ level: 'debug', category: 'system', source: 'ws-send', summary: `ws-send ${String(url)}`, details: { preview: typeof data === 'string' ? data.slice(0, 500) : typeof data } }); } catch {}
        return origSend(data);
      };
      return instance;
    } as unknown as typeof WebSocket;
    WrappedWS.prototype = NativeWS.prototype;
    (window as any).WebSocket = WrappedWS;
  } catch (e) {
    // ignore if WebSocket cannot be wrapped
  }
}

export function instrumentAll() {
  try { instrumentUserActions(); } catch {}
  try { instrumentNetwork(); } catch {}
}

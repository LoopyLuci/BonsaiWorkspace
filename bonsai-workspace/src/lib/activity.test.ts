// @vitest-environment jsdom
import { beforeEach, afterEach, describe, it, expect, vi } from 'vitest';
import { pushActivity, instrumentNetwork, instrumentUserActions } from './activity';

describe('activity instrumentation', () => {
  beforeEach(() => {
    // reset install flags so instrumentation can be re-applied per test
    (window as any).__bonsai_network_installed = false;
    (window as any).__bonsai_user_actions_installed = false;
  });

  afterEach(() => {
    // cleanup listeners
    // remove all bonsai-activity listeners by cloning and removing
  });

  it('pushActivity dispatches bonsai-activity event', () => {
    const received: any[] = [];
    const handler = (e: any) => received.push(e.detail);
    window.addEventListener('bonsai-activity', handler as EventListener);

    pushActivity({ level: 'info', category: 'test', summary: 'hello', details: { foo: 1 } });

    expect(received.length).toBeGreaterThanOrEqual(1);
    expect(received[0].summary).toBe('hello');

    window.removeEventListener('bonsai-activity', handler as EventListener);
  });

  it('instrumentNetwork wraps fetch and emits activity events', async () => {
    const events: any[] = [];
    const handler = (e: any) => events.push(e.detail);
    window.addEventListener('bonsai-activity', handler as EventListener);

    const origFetch = (window as any).fetch;
    // simple mock fetch that resolves with ok/status
    (window as any).fetch = async (input: any, init?: any) => ({ ok: true, status: 200, json: async () => ({}) });

    try {
      instrumentNetwork();
      // call fetch which is wrapped
      await (window as any).fetch('http://example.test/ok', { method: 'POST' });
      // allow microtasks
      await new Promise((r) => setTimeout(r, 5));

      // should have emitted at least one fetch-related activity
      const found = events.some(ev => ev && ev.source === 'fetch' && typeof ev.summary === 'string' && ev.summary.includes('http://example.test/ok'));
      expect(found).toBe(true);
    } finally {
      // restore
      (window as any).fetch = origFetch;
      window.removeEventListener('bonsai-activity', handler as EventListener);
    }
  });

  it('instrumentNetwork wraps WebSocket and emits connect/open events', async () => {
    const events: any[] = [];
    const handler = (e: any) => events.push(e.detail);
    window.addEventListener('bonsai-activity', handler as EventListener);

    const OrigWS = (window as any).WebSocket;
    // Minimal mock native WebSocket implementation
    class MockNativeWS {
      public url: string;
      private listeners: Record<string, Function[]> = {};
      constructor(url: string) {
        this.url = url;
        // simulate async open
        setTimeout(() => {
          (this.listeners.open || []).forEach(cb => cb({}));
        }, 0);
      }
      addEventListener(ev: string, cb: Function) {
        this.listeners[ev] = this.listeners[ev] || [];
        this.listeners[ev].push(cb);
      }
      send(_data: any) { /* noop */ }
      close() { /* noop */ }
    }

    try {
      (window as any).WebSocket = MockNativeWS;
      instrumentNetwork();
      // instantiate a wrapped WS which should emit activities
      const ws = new (window as any).WebSocket('ws://localhost/ws');
      // wait for the simulated open
      await new Promise((r) => setTimeout(r, 20));

      const hasWsConnect = events.some(ev => ev && ev.source === 'ws' && typeof ev.summary === 'string' && ev.summary.includes('ws connect'));
      const hasWsOpen = events.some(ev => ev && ev.source === 'ws' && typeof ev.summary === 'string' && ev.summary.includes('ws open'));
      expect(hasWsConnect || hasWsOpen).toBe(true);
    } finally {
      (window as any).WebSocket = OrigWS;
      window.removeEventListener('bonsai-activity', handler as EventListener);
    }
  });

  it('instrumentUserActions captures clicks on elements with data-bonsai-action', () => {
    const events: any[] = [];
    const handler = (e: any) => events.push(e.detail);
    window.addEventListener('bonsai-activity', handler as EventListener);

    try {
      instrumentUserActions();
      const btn = document.createElement('button');
      btn.setAttribute('data-bonsai-action', 'Testing:Click');
      btn.textContent = 'ClickMe';
      document.body.appendChild(btn);

      btn.click();

      const found = events.some(ev => ev && ev.category === 'ui' && ev.summary === 'Testing:Click');
      expect(found).toBe(true);
    } finally {
      window.removeEventListener('bonsai-activity', handler as EventListener);
    }
  });

  it('instrumentUserActions captures clicks when the actual target is a nested node', () => {
    const events: any[] = [];
    const handler = (e: any) => events.push(e.detail);
    window.addEventListener('bonsai-activity', handler as EventListener);

    try {
      instrumentUserActions();
      const btn = document.createElement('button');
      btn.setAttribute('data-bonsai-action', 'Nested:Click');
      btn.textContent = '';
      const span = document.createElement('span');
      span.textContent = 'inner';
      btn.appendChild(span);
      document.body.appendChild(btn);

      // simulate clicking the inner span (event.target is the span)
      span.click();

      const found = events.some(ev => ev && ev.category === 'ui' && ev.summary === 'Nested:Click');
      expect(found).toBe(true);
    } finally {
      window.removeEventListener('bonsai-activity', handler as EventListener);
    }
  });
});

import type { BackgroundRequest, PageSnapshot, SelectedElementInfo } from '../lib/types';
import browser from '../lib/browser';

function cssSelector(el: Element): string {
  const path: string[] = [];
  let element: Element | null = el;

  while (element && element.nodeType === Node.ELEMENT_NODE && path.length < 6) {
    let selector = element.nodeName.toLowerCase();
    if (element.id) {
      selector += `#${CSS.escape(element.id)}`;
      path.unshift(selector);
      break;
    }
    const className = (element.getAttribute('class') || '').trim();
    if (className) {
      const firstClass = className.split(/\s+/).filter(Boolean)[0];
      if (firstClass) selector += `.${CSS.escape(firstClass)}`;
    }
    const parent: Element | null = element.parentElement;
    if (parent) {
      const siblings = Array.from(parent.children).filter((node: Element) => node.nodeName === element?.nodeName);
      if (siblings.length > 1) {
        const index = siblings.indexOf(element) + 1;
        selector += `:nth-of-type(${index})`;
      }
    }
    path.unshift(selector);
    element = parent;
  }

  return path.join(' > ');
}

function getVisibleText(maxLen = 16000): string {
  const text = document.body?.innerText || '';
  return text.replace(/\s+\n/g, '\n').replace(/\n{3,}/g, '\n\n').slice(0, maxLen);
}

function highlightElement(element: Element): void {
  const existing = document.getElementById('__bonsai_highlight__');
  existing?.remove();

  const rect = element.getBoundingClientRect();
  const box = document.createElement('div');
  box.id = '__bonsai_highlight__';
  box.style.position = 'fixed';
  box.style.left = `${rect.left}px`;
  box.style.top = `${rect.top}px`;
  box.style.width = `${rect.width}px`;
  box.style.height = `${rect.height}px`;
  box.style.border = '2px solid #0f766e';
  box.style.background = 'rgba(15,118,110,0.12)';
  box.style.zIndex = '2147483647';
  box.style.pointerEvents = 'none';
  box.style.borderRadius = '6px';
  document.documentElement.appendChild(box);
}

function createConfirmationOverlay(message: string): Promise<boolean> {
  return new Promise((resolve) => {
    const container = document.createElement('div');
    container.style.position = 'fixed';
    container.style.right = '16px';
    container.style.bottom = '16px';
    container.style.maxWidth = '360px';
    container.style.background = '#111827';
    container.style.color = '#f9fafb';
    container.style.padding = '12px';
    container.style.borderRadius = '10px';
    container.style.zIndex = '2147483647';
    container.style.font = '13px/1.4 "Segoe UI", sans-serif';
    container.style.boxShadow = '0 10px 30px rgba(0,0,0,0.35)';

    const text = document.createElement('div');
    text.textContent = message;
    container.appendChild(text);

    const buttons = document.createElement('div');
    buttons.style.display = 'flex';
    buttons.style.gap = '8px';
    buttons.style.marginTop = '10px';

    const allow = document.createElement('button');
    allow.textContent = 'Allow';
    allow.style.border = '0';
    allow.style.padding = '6px 10px';
    allow.style.borderRadius = '8px';
    allow.style.background = '#0f766e';
    allow.style.color = '#fff';

    const deny = document.createElement('button');
    deny.textContent = 'Deny';
    deny.style.border = '0';
    deny.style.padding = '6px 10px';
    deny.style.borderRadius = '8px';
    deny.style.background = '#991b1b';
    deny.style.color = '#fff';

    const cleanup = (decision: boolean) => {
      container.remove();
      resolve(decision);
    };

    allow.onclick = () => cleanup(true);
    deny.onclick = () => cleanup(false);

    buttons.appendChild(allow);
    buttons.appendChild(deny);
    container.appendChild(buttons);
    document.documentElement.appendChild(container);

    window.setTimeout(() => cleanup(false), 5000);
  });
}

async function executeAutomation(payload: Extract<BackgroundRequest, { type: 'REQUEST_AUTOMATION' }>): Promise<{ ok: boolean; error?: string }> {
  try {
    if (payload.action === 'navigate' && payload.url) {
      window.location.assign(payload.url);
      return { ok: true };
    }

    if (payload.action === 'scroll') {
      window.scrollBy({ top: 600, behavior: 'smooth' });
      return { ok: true };
    }

    if (!payload.selector) {
      return { ok: false, error: 'Missing selector for action' };
    }

    const element = document.querySelector(payload.selector);
    if (!element) {
      return { ok: false, error: `Element not found for selector: ${payload.selector}` };
    }

    highlightElement(element);

    if (payload.action === 'click') {
      (element as HTMLElement).click();
      return { ok: true };
    }

    if (payload.action === 'type') {
      const input = element as HTMLInputElement | HTMLTextAreaElement;
      input.focus();
      input.value = payload.text ?? '';
      input.dispatchEvent(new InputEvent('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
      return { ok: true };
    }

    return { ok: false, error: 'Unsupported action' };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : 'Unknown automation error'
    };
  }
}

browser.runtime.onMessage.addListener(async (message: unknown) => {
  const msg = (message ?? {}) as { type?: string; payload?: Record<string, unknown> };
  if (!msg.type) return null;

  switch (msg.type) {
    case 'GET_PAGE_SNAPSHOT': {
      const includeHtml = Boolean(msg.payload?.includeHtml);
      const data: PageSnapshot = {
        title: document.title,
        url: location.href,
        visibleText: getVisibleText(),
        html: includeHtml ? document.documentElement.outerHTML.slice(0, 200000) : undefined
      };
      return data;
    }

    case 'PICK_ELEMENT': {
      const active = document.activeElement as Element | null;
      if (!active) {
        return null;
      }
      highlightElement(active);
      const response: SelectedElementInfo = {
        selector: cssSelector(active),
        text: (active as HTMLElement).innerText?.slice(0, 200) ?? '',
        tagName: active.tagName.toLowerCase()
      };
      return response;
    }

    case 'AUTOMATION_CONFIRM': {
      const action = String(msg.payload?.action ?? 'action');
      const selector = msg.payload?.selector ? ` on ${String(msg.payload.selector)}` : '';
      const allowed = await createConfirmationOverlay(`Bonsai wants to ${action}${selector}.`);
      return { allowed };
    }

    case 'AUTOMATION_EXECUTE':
      return executeAutomation(msg.payload as Extract<BackgroundRequest, { type: 'REQUEST_AUTOMATION' }>);

    default:
      return null;
  }
});

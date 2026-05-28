/**
 * ReconnectingWebSocket — drop-in WebSocket with automatic reconnection.
 *
 * Features:
 * - Exponential back-off with jitter (max 30 s).
 * - Outbound message queue: messages sent while disconnected are buffered
 *   and flushed automatically on reconnect.
 * - Sequence-number tracking: each outbound message gets a monotone seq so
 *   the server can detect gaps and replay missed events on reconnect.
 * - Observable via `.on('open'|'close'|'message'|'reconnect'|'error')`.
 * - Graceful close (code 1000) skips reconnect.
 */

export type RwsEventType = 'open' | 'close' | 'message' | 'reconnect' | 'error';

export interface RwsOptions {
  /** Maximum reconnection delay in ms (default 30 000). */
  maxDelay?:         number;
  /** Initial reconnection delay in ms (default 500). */
  baseDelay?:        number;
  /** Ping interval in ms; 0 to disable (default 20 000). */
  pingIntervalMs?:   number;
  /** Protocols forwarded to the WebSocket constructor. */
  protocols?:        string | string[];
  /** Max outbound messages to queue while disconnected (default 200). */
  maxQueueSize?:     number;
}

interface QueuedMessage {
  seq: number;
  payload: string;
}

export class ReconnectingWebSocket {
  private url:      string;
  private opts:     Required<RwsOptions>;
  private ws:       WebSocket | null = null;

  private attempts  = 0;
  private seq       = 0;            // monotone outbound sequence counter
  private queue:    QueuedMessage[] = [];
  private listeners: Map<RwsEventType, Array<(data: unknown) => void>> = new Map();

  private reconnectTimer:  ReturnType<typeof setTimeout> | null = null;
  private pingTimer:       ReturnType<typeof setInterval> | null = null;
  private closed = false;           // set by explicit .close() call

  /** Last received server sequence number (for replay negotiation). */
  public lastServerSeq = 0;

  constructor(url: string, options: RwsOptions = {}) {
    this.url  = url;
    this.opts = {
      maxDelay:       options.maxDelay       ?? 30_000,
      baseDelay:      options.baseDelay      ?? 500,
      pingIntervalMs: options.pingIntervalMs ?? 20_000,
      protocols:      options.protocols      ?? [],
      maxQueueSize:   options.maxQueueSize   ?? 200,
    };
    this.connect();
  }

  // ── Public API ─────────────────────────────────────────────────────────────

  /** Send a message. Queues it if not currently connected. */
  send(data: string): void {
    const msg: QueuedMessage = { seq: ++this.seq, payload: data };
    if (this.ws?.readyState === WebSocket.OPEN) {
      this._transmit(msg);
    } else {
      if (this.queue.length >= this.opts.maxQueueSize) {
        // Drop oldest to respect budget.
        this.queue.shift();
      }
      this.queue.push(msg);
    }
  }

  /** Send a JSON-serialisable value. */
  sendJson(value: unknown): void {
    this.send(JSON.stringify(value));
  }

  /** Register an event listener. */
  on(event: RwsEventType, cb: (data: unknown) => void): () => void {
    if (!this.listeners.has(event)) this.listeners.set(event, []);
    this.listeners.get(event)!.push(cb);
    // Returns an unlisten function.
    return () => {
      const arr = this.listeners.get(event);
      if (arr) {
        const i = arr.indexOf(cb);
        if (i !== -1) arr.splice(i, 1);
      }
    };
  }

  /** Current WebSocket ready state (or CLOSED if no socket). */
  get readyState(): number {
    return this.ws?.readyState ?? WebSocket.CLOSED;
  }

  /** Close permanently — will not reconnect. */
  close(code = 1000, reason = 'client close'): void {
    this.closed = true;
    if (this.reconnectTimer) { clearTimeout(this.reconnectTimer); this.reconnectTimer = null; }
    if (this.pingTimer)      { clearInterval(this.pingTimer);     this.pingTimer      = null; }
    this.ws?.close(code, reason);
    this.ws = null;
  }

  // ── Internal ───────────────────────────────────────────────────────────────

  private connect(): void {
    if (this.closed) return;

    try {
      this.ws = new WebSocket(this.url, this.opts.protocols || undefined);
    } catch (e) {
      this._emit('error', e);
      this._scheduleReconnect();
      return;
    }

    this.ws.onopen = () => {
      this.attempts = 0;
      this._startPing();
      this._flushQueue();
      this._emit('open', {});
    };

    this.ws.onclose = (ev: CloseEvent) => {
      this._stopPing();
      this._emit('close', ev);
      if (!this.closed && ev.code !== 1000) {
        this._scheduleReconnect();
      }
    };

    this.ws.onerror = (ev: Event) => {
      this._emit('error', ev);
      // onclose will fire next — reconnect happens there.
    };

    this.ws.onmessage = (ev: MessageEvent) => {
      try {
        const data = JSON.parse(ev.data as string);
        // Track server sequence for replay negotiation.
        if (typeof data?.seq === 'number') this.lastServerSeq = data.seq;
        this._emit('message', data);
      } catch {
        this._emit('message', ev.data);
      }
    };
  }

  private _scheduleReconnect(): void {
    if (this.closed) return;
    const base  = this.opts.baseDelay * Math.pow(2, this.attempts);
    const capped = Math.min(base, this.opts.maxDelay);
    const jitter = capped * 0.2 * (Math.random() - 0.5);
    const delay  = Math.round(capped + jitter);
    this.attempts++;
    this._emit('reconnect', { attempt: this.attempts, delayMs: delay });
    this.reconnectTimer = setTimeout(() => this.connect(), delay);
  }

  private _flushQueue(): void {
    const pending = [...this.queue];
    this.queue = [];
    for (const msg of pending) {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this._transmit(msg);
      } else {
        // Socket closed again while flushing — re-queue.
        this.queue.unshift(msg);
        break;
      }
    }
  }

  private _transmit(msg: QueuedMessage): void {
    // Wrap with seq so the server can detect duplicates / gaps.
    this.ws!.send(JSON.stringify({ _rws_seq: msg.seq, _payload: msg.payload }));
  }

  private _startPing(): void {
    if (this.opts.pingIntervalMs <= 0) return;
    this.pingTimer = setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify({ type: 'ping' }));
      }
    }, this.opts.pingIntervalMs);
  }

  private _stopPing(): void {
    if (this.pingTimer) { clearInterval(this.pingTimer); this.pingTimer = null; }
  }

  private _emit(event: RwsEventType, data: unknown): void {
    for (const cb of this.listeners.get(event) ?? []) {
      try { cb(data); } catch (e) { console.error(`[RWS] listener error on '${event}':`, e); }
    }
  }
}

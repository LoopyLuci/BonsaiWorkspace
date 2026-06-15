# Web Service Example

This example demonstrates Omnisystem's Aether actor system handling HTTP requests with automatic supervision and restart.

## What This Shows

- **Actor spawn**: Dynamic actor creation per request
- **Supervision tree**: ONE_FOR_ONE restart strategy
- **Message routing**: Router actor dispatches requests to handlers
- **Failure tolerance**: Dead actors automatically restart
- **Telemetry**: Event emission for monitoring
- **Supervision lifecycle**: on_start, on_message, on_stop, on_restart hooks

## Architecture

```
Supervisor (ONE_FOR_ONE)
├── HttpServer Actor
│   └── Handles incoming requests
│       └── Emits telemetry events
│       └── Restarts on failure
└── Router Actor
    └── Routes requests to handlers
    └── Calls Titan compute functions
```

## How It Works

1. **Request arrives** → HttpServer actor receives HttpRequest message
2. **Track state** → Increment request_count, active_connections
3. **Route** → Match path and method to handler
4. **Compute** → Call Titan function for heavy work (e.g., matrix multiply)
5. **Respond** → Send HttpResponse back
6. **Monitor** → Emit telemetry event

## Supervision

If an actor crashes:
- Supervisor detects dead actor (background monitor thread)
- Waits cooldown period (1 second)
- Restarts actor with fresh state
- on_restart() hook resets counters

If supervisor itself crashes, parent supervisor restarts it.

## Running

Currently a **stub** demonstrating syntax and design. Full implementation requires:
- HTTP server socket binding
- Message serialization for network transport
- Integration with OmniCore scheduler

Target: Phase 3 (multi-node Aether)

## Integration with Sylva

In the REPL, you could interact with the web service:

```
sylva> import abc123def456 as ws
  Imported ws from registry

sylva> ws::start_web_service()
  Web service started with supervision tree

sylva> ws::make_request("/health", "GET", "")
  = HttpResponse { status: 200, body: "OK", headers: [] }
```

## See Also

- `hello_world.build` — Basic actor example
- `data_pipeline/` — Sylva + Titan integration
- CONTRIBUTING.md — How to build on this

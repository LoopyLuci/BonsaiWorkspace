Python runtime worker (prototype)

This simple worker is a prototype for the Python runtime. It exposes a
minimal HTTP `/health` endpoint and can be used with `bonsai-runtime`'s
`start_python_worker` to validate process spawning and health checks.

Run locally:

```bash
python runtimes/python/worker.py 8001
curl http://127.0.0.1:8001/health
```

No external dependencies required (uses Python 3 stdlib `http.server`).

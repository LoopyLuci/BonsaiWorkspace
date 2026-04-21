Babashka runtime worker (prototype)

This is a minimal Babashka (bb) script that implements a simple line-based
protocol on stdin/stdout. It responds to the `health` command with a JSON
payload. Use `bb runtimes/clojure/bb_runner.clj` to run.

Example:

```bash
echo health | bb runtimes/clojure/bb_runner.clj
```

Babashka is recommended for quick scripting and transformations; for heavier
JVM tasks a dedicated JVM worker can be added later.

#!/usr/bin/env python3
import argparse
import os
import sys
import threading
import time
import tracemalloc
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import subprocess
from urllib.parse import urlparse

DEFAULT_MAX_CPU_SECONDS = 30
DEFAULT_MAX_MEMORY_MB = 512


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Bonsai Python worker")
    parser.add_argument("port", nargs="?", type=int, default=8000)
    parser.add_argument("--max-cpu-seconds", type=int, default=DEFAULT_MAX_CPU_SECONDS)
    parser.add_argument("--max-memory-mb", type=int, default=DEFAULT_MAX_MEMORY_MB)
    return parser.parse_args()


def _terminate_worker(reason: str) -> None:
    print(f"python worker limit reached: {reason}", file=sys.stderr)
    os._exit(137)


def apply_resource_limits(max_cpu_seconds: int, max_memory_mb: int) -> None:
    max_cpu_seconds = max(1, int(max_cpu_seconds))
    max_memory_mb = max(64, int(max_memory_mb))

    # Start Python-level trackers for all platforms.
    tracemalloc.start()

    if os.name != "nt":
        try:
            import resource

            memory_bytes = max_memory_mb * 1024 * 1024
            resource.setrlimit(resource.RLIMIT_CPU, (max_cpu_seconds, max_cpu_seconds))
            resource.setrlimit(resource.RLIMIT_AS, (memory_bytes, memory_bytes))
        except Exception as exc:
            print(f"python worker warning: failed to set rlimit: {exc}", file=sys.stderr)

        # Backup timer to ensure long-running workloads are interrupted.
        try:
            import signal

            signal.alarm(max_cpu_seconds)
        except Exception as exc:
            print(f"python worker warning: failed to set alarm: {exc}", file=sys.stderr)

    def watchdog() -> None:
        start_cpu = time.process_time()
        memory_limit = max_memory_mb * 1024 * 1024
        while True:
            time.sleep(0.25)
            cpu_used = time.process_time() - start_cpu
            if cpu_used > max_cpu_seconds:
                _terminate_worker(f"cpu>{max_cpu_seconds}s")

            current, peak = tracemalloc.get_traced_memory()
            if peak > memory_limit or current > memory_limit:
                _terminate_worker(f"memory>{max_memory_mb}MB")

    threading.Thread(target=watchdog, name="bonsai-worker-watchdog", daemon=True).start()

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'status': 'ok'}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        if self.path == '/run':
            content_length = int(self.headers.get('Content-Length', 0))
            if content_length == 0:
                self.send_response(400)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({'error': 'empty request body'}).encode())
                return

            try:
                body = self.rfile.read(content_length)
                request = json.loads(body.decode())
                script = request.get('script', '')
                args = request.get('args', '')

                if not script:
                    self.send_response(400)
                    self.send_header('Content-Type', 'application/json')
                    self.end_headers()
                    self.wfile.write(json.dumps({'error': 'script is required'}).encode())
                    return

                # Execute the script with optional arguments
                cmd = [sys.executable, '-c', script]
                if args:
                    cmd.append(args)

                try:
                    result = subprocess.run(
                        cmd,
                        capture_output=True,
                        text=True,
                        timeout=30  # Safety timeout
                    )
                    response = {
                        'stdout': result.stdout,
                        'stderr': result.stderr,
                        'exit_code': result.returncode
                    }
                except subprocess.TimeoutExpired:
                    response = {
                        'stdout': '',
                        'stderr': 'script execution exceeded 30 second timeout',
                        'exit_code': 124
                    }
                except Exception as e:
                    response = {
                        'stdout': '',
                        'stderr': f'script execution failed: {str(e)}',
                        'exit_code': 1
                    }

                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(response).encode())
            except json.JSONDecodeError:
                self.send_response(400)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({'error': 'invalid JSON'}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        # Silence default logging to keep output clean for tests
        return

if __name__ == '__main__':
    args = parse_args()
    apply_resource_limits(args.max_cpu_seconds, args.max_memory_mb)

    port = args.port
    server = HTTPServer(('127.0.0.1', port), Handler)
    print(f"python worker listening on {port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass

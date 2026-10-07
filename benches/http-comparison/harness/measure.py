#!/usr/bin/env python3
"""
Process launcher that measures:
  - Cold start time: from process spawn to first 200 OK on /health
  - RSS after warmup: resident set size (KB) sampled N times, we report the median

Why median RSS instead of peak: peak is noisy (GC cycles, lazy loading),
median after warmup is what an operator actually cares about.

Usage:
    python3 measure.py \\
        --name zun \\
        --cwd /path/to/zun/fixture \\
        --cmd "/path/to/zun serve ." \\
        --health http://127.0.0.1:8080/health \\
        --warmup-url http://127.0.0.1:8080/hello \\
        --rss-samples 5 --rss-interval 0.5

Emits a JSON blob to stdout.
"""
from __future__ import annotations

import argparse
import json
import os
import shlex
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request


def poll_ready(url: str, timeout_s: float) -> float | None:
    """Return seconds from call until first 200 OK, or None on timeout."""
    t0 = time.monotonic()
    deadline = t0 + timeout_s
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=0.5) as resp:
                if resp.status == 200:
                    return time.monotonic() - t0
        except (urllib.error.URLError, ConnectionError, OSError):
            time.sleep(0.02)
    return None


def rss_kb(pid: int) -> int | None:
    """Cross-platform RSS sampling via `ps`.

    Linux and macOS both ship `ps -o rss= -p <pid>` returning KB.
    """
    try:
        out = subprocess.check_output(
            ["ps", "-o", "rss=", "-p", str(pid)],
            stderr=subprocess.DEVNULL,
        ).decode().strip()
        return int(out) if out else None
    except Exception:
        return None


def warmup(url: str, n: int = 200) -> None:
    for _ in range(n):
        try:
            with urllib.request.urlopen(url, timeout=1.0):
                pass
        except Exception:
            pass


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--name", required=True)
    ap.add_argument("--cwd", required=True)
    ap.add_argument("--cmd", required=True, help="shell command to launch server")
    ap.add_argument("--health", required=True, help="health URL to poll")
    ap.add_argument("--warmup-url", required=True)
    ap.add_argument("--ready-timeout", type=float, default=30.0)
    ap.add_argument("--rss-samples", type=int, default=5)
    ap.add_argument("--rss-interval", type=float, default=0.5)
    args = ap.parse_args()

    # Spawn in its own process group so we can SIGTERM the whole tree
    # (some servers fork workers — e.g. uvicorn with --workers=N).
    env = os.environ.copy()
    # Force unbuffered I/O for Python so we don't dodge startup cost.
    env.setdefault("PYTHONUNBUFFERED", "1")

    proc = subprocess.Popen(
        shlex.split(args.cmd),
        cwd=args.cwd,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    try:
        ready_s = poll_ready(args.health, args.ready_timeout)
        if ready_s is None:
            result = {
                "name": args.name,
                "error": f"server did not become ready within {args.ready_timeout}s",
                "pid": proc.pid,
            }
            print(json.dumps(result, indent=2))
            sys.exit(1)

        warmup(args.warmup_url, n=200)

        rss_samples = []
        for _ in range(args.rss_samples):
            v = rss_kb(proc.pid)
            if v is not None:
                rss_samples.append(v)
            time.sleep(args.rss_interval)

        rss_samples.sort()
        rss_median = rss_samples[len(rss_samples) // 2] if rss_samples else None

        result = {
            "name": args.name,
            "pid": proc.pid,
            "cold_start_s": round(ready_s, 4),
            "rss_kb_median": rss_median,
            "rss_kb_samples": rss_samples,
        }
        print(json.dumps(result, indent=2))
    finally:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
            proc.wait(timeout=5)
        except Exception:
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
            except Exception:
                pass


if __name__ == "__main__":
    main()

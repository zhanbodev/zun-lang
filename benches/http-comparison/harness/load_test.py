#!/usr/bin/env python3
"""
Async HTTP load tester — deliberately stdlib-only so it runs anywhere without
a `pip install` step. Reports QPS, latency percentiles, error counts.

Usage:
    python3 load_test.py --url http://127.0.0.1:8080/hello --duration 30 --connections 64

Design decisions (document these alongside results so critics can verify):
  - HTTP/1.1 with keep-alive (one persistent connection per worker)
  - Fixed-duration test (not fixed-request-count) — removes warm/cold bias
  - Latency measured per-request, percentiles computed on the full sample
  - Workers run independently; we do NOT coordinate request pacing
  - No request pipelining (one in-flight per connection)

Caveats:
  - A pure-Python async loop WILL be slower than wrk / bombardier as a *client*.
    That's fine for comparing *servers to each other* (all three receive the
    same pressure), but absolute numbers underestimate real ceiling. When you
    report numbers, call this out.
"""
from __future__ import annotations

import argparse
import asyncio
import json
import statistics
import time
from dataclasses import dataclass, field


@dataclass
class Sample:
    latencies_us: list[int] = field(default_factory=list)
    ok: int = 0
    errors: int = 0
    bytes_read: int = 0


async def worker(
    host: str,
    port: int,
    path: str,
    deadline: float,
    sample: Sample,
) -> None:
    """One persistent connection, sequential requests until deadline."""
    request = (
        f"GET {path} HTTP/1.1\r\n"
        f"Host: {host}:{port}\r\n"
        f"User-Agent: zun-bench/0.1\r\n"
        f"Accept: application/json\r\n"
        f"Connection: keep-alive\r\n"
        f"\r\n"
    ).encode()

    try:
        reader, writer = await asyncio.open_connection(host, port)
    except Exception:
        sample.errors += 1
        return

    try:
        while time.monotonic() < deadline:
            t0 = time.monotonic_ns()
            writer.write(request)
            try:
                await writer.drain()
            except Exception:
                sample.errors += 1
                return

            # Read status line + headers
            content_length = -1
            chunked = False
            try:
                # status line — accept any HTTP version with 2xx
                line = await reader.readuntil(b"\r\n")
                # Format: "HTTP/<ver> <code> <reason>\r\n"
                try:
                    _ver, code, *_ = line.split(b" ", 2)
                    if not code.startswith(b"2"):
                        sample.errors += 1
                        return
                except ValueError:
                    sample.errors += 1
                    return
                # headers
                while True:
                    h = await reader.readuntil(b"\r\n")
                    if h == b"\r\n":
                        break
                    lower = h.lower()
                    if lower.startswith(b"content-length:"):
                        content_length = int(h.split(b":", 1)[1].strip())
                    elif lower.startswith(b"transfer-encoding:") and b"chunked" in lower:
                        chunked = True

                # body
                if content_length >= 0:
                    body = await reader.readexactly(content_length)
                    sample.bytes_read += len(body)
                elif chunked:
                    while True:
                        size_line = await reader.readuntil(b"\r\n")
                        size = int(size_line.strip(), 16)
                        if size == 0:
                            await reader.readuntil(b"\r\n")
                            break
                        chunk = await reader.readexactly(size + 2)
                        sample.bytes_read += size
                else:
                    # No Content-Length and not chunked — server will close; bail
                    sample.errors += 1
                    return
            except Exception:
                sample.errors += 1
                return

            t1 = time.monotonic_ns()
            sample.latencies_us.append((t1 - t0) // 1000)
            sample.ok += 1
    finally:
        try:
            writer.close()
            await writer.wait_closed()
        except Exception:
            pass


def percentile(sorted_us: list[int], p: float) -> float:
    if not sorted_us:
        return float("nan")
    k = (len(sorted_us) - 1) * p
    lo = int(k)
    hi = min(lo + 1, len(sorted_us) - 1)
    frac = k - lo
    return sorted_us[lo] * (1 - frac) + sorted_us[hi] * frac


async def run(args: argparse.Namespace) -> dict:
    # Parse URL
    from urllib.parse import urlparse

    u = urlparse(args.url)
    host = u.hostname or "127.0.0.1"
    port = u.port or 80
    path = u.path or "/"

    # Warm-up: a few sequential requests on one connection
    warm = Sample()
    await worker(host, port, path, time.monotonic() + args.warmup, warm)

    # Real test
    deadline = time.monotonic() + args.duration
    samples = [Sample() for _ in range(args.connections)]
    t0 = time.monotonic()
    await asyncio.gather(
        *(worker(host, port, path, deadline, s) for s in samples)
    )
    elapsed = time.monotonic() - t0

    # Aggregate
    all_latencies: list[int] = []
    ok = 0
    errors = 0
    bytes_read = 0
    for s in samples:
        all_latencies.extend(s.latencies_us)
        ok += s.ok
        errors += s.errors
        bytes_read += s.bytes_read
    all_latencies.sort()

    def _r(v):
        return None if v != v else round(v)  # NaN-safe

    result = {
        "url": args.url,
        "duration_s": round(elapsed, 3),
        "connections": args.connections,
        "warmup_s": args.warmup,
        "requests": ok,
        "errors": errors,
        "qps": round(ok / elapsed, 1) if elapsed > 0 else 0.0,
        "throughput_mbps": round((bytes_read * 8) / (elapsed * 1_000_000), 2) if elapsed > 0 else 0.0,
        "latency_us": {
            "min": all_latencies[0] if all_latencies else None,
            "p50": _r(percentile(all_latencies, 0.50)) if all_latencies else None,
            "p90": _r(percentile(all_latencies, 0.90)) if all_latencies else None,
            "p99": _r(percentile(all_latencies, 0.99)) if all_latencies else None,
            "max": all_latencies[-1] if all_latencies else None,
            "mean": round(statistics.fmean(all_latencies)) if all_latencies else None,
        },
    }
    return result


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--url", required=True)
    ap.add_argument("--duration", type=float, default=30.0, help="seconds")
    ap.add_argument("--warmup", type=float, default=3.0, help="seconds")
    ap.add_argument("--connections", type=int, default=64)
    ap.add_argument("--json", action="store_true", help="emit JSON only")
    args = ap.parse_args()

    result = asyncio.run(run(args))

    if args.json:
        print(json.dumps(result, indent=2))
        return

    lat = result["latency_us"]
    print(f"URL:         {result['url']}")
    print(f"Duration:    {result['duration_s']}s  (warmup {result['warmup_s']}s)")
    print(f"Connections: {result['connections']}")
    print(f"Requests:    {result['requests']} ({result['errors']} errors)")
    print(f"QPS:         {result['qps']}")
    print(f"Throughput:  {result['throughput_mbps']} Mbps")
    print(f"Latency (μs): min={lat['min']}  p50={lat['p50']}  p90={lat['p90']}  p99={lat['p99']}  max={lat['max']}")


if __name__ == "__main__":
    main()

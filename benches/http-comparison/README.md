# HTTP Service Benchmark: zun vs FastAPI vs Hono

Three equivalent "hello JSON" services, three real measurements:

1. **Cold start** — time from process spawn to first 200 OK on `/health`
2. **Resident memory** — median RSS after warmup
3. **Throughput & latency** — QPS and p50/p90/p99 on `/hello` and `/greet/:name`

All three services implement the same three endpoints with the same payloads,
run single-process with default settings, and face the same client.

## Why this exists

zun is an interpreted language. The natural objection is "interpreters are
slow". This suite gives real numbers so the conversation moves from vibes to
data. When numbers make zun look bad, we publish them anyway — credibility
compounds, cherry-picking doesn't.

## Layout

```
benches/http-comparison/
├── zun/              # package.toml + main.zun + routers
├── fastapi/             # FastAPI + uvicorn
├── hono/                # Hono on Node (not Bun — see below)
├── harness/
│   ├── measure.py       # cold-start + RSS sampler
│   ├── load_test.py     # pure-stdlib async load generator
│   ├── collate.py       # → results/summary.md
│   └── run_all.sh       # orchestrator
└── results/             # JSON + summary.md
```

## How to run

```bash
cd benches/http-comparison
./harness/run_all.sh

# Tuning knobs (env vars):
DURATION=60 CONNECTIONS=128 ./harness/run_all.sh
```

Output lands in `results/summary.md`.

## Reproducibility requirements (put these next to any published numbers)

- macOS / Linux, no other load on the machine, power plugged in
- Report `uname -a`, CPU model, Python version, Node version, zun commit
- Three consecutive runs — report median, note variance
- Note whether other things were running (Chrome, Docker Desktop, etc.)

## Known limitations of this harness

These matter because any honest critic will point them out first:

1. **Pure-Python client**. `wrk` or `bombardier` would drive higher load.
   All three servers receive identical pressure from this client, so
   relative ranking is fair; absolute ceilings are underestimated.
2. **Single process, default config**. We don't run uvicorn `--workers 8`
   or `node --cluster`. That's deliberate — production tuning is a different
   conversation. But call it out.
3. **No TLS, no HTTP/2**. Plain HTTP/1.1.
4. **Trivial payloads**. Real apps have middleware, auth, serialization,
   DB I/O. Those costs dwarf the framework. If zun wins here by 2x,
   that often translates to 1.05x in a real app.
5. **Hono via Node, not Bun**. Bun + Hono is faster. Adding a Bun variant
   is a one-line change in `hono/` — do it if you want the fair "Hono at
   its best" number.

## What to do with the numbers

If zun wins cold start and RSS by a meaningful margin:
→ That's the pitch. Edge / FaaS / AI-agent-sidecar scenarios.
→ Cold start < 50ms and RSS < 20MB are the numbers that actually sell.

If zun loses QPS: **expected, fine**. Interpreted vs Node V8 / Python uvloop
is not a winnable fight today. Don't pretend it is. The pitch is elsewhere.

If zun loses everything:
→ Don't ship. Find what's slow, profile, iterate.

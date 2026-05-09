#!/usr/bin/env bash
# capture-screenshots.sh
#
# Spins up `illuminate wiki serve` against a tempdir populated with sample
# data, prints the URLs you should hit with your screenshot tool, then
# pauses until you press a key. The whole thing tears down on exit.
#
# Captured screenshots go in docs/screenshots/ in the illuminate repo:
#
#   docs/screenshots/dashboard-home.png        from /
#   docs/screenshots/page-view.png             from /page/decisions/dec-no-redis
#   docs/screenshots/audit-playground.png      from /audit (after submitting a Redis plan)
#   docs/screenshots/search.png                from /search?q=caching
#   docs/screenshots/decisions-list.png        from /decisions
#
# Usage:
#   scripts/capture-screenshots.sh [PORT]
#
# Default port: 8765. Use a different port if 8765 is busy.

set -euo pipefail

PORT="${1:-8765}"
ILLUMINATE="${ILLUMINATE:-target/release/illuminate}"

if [[ ! -x "$ILLUMINATE" ]]; then
    echo "binary not found at $ILLUMINATE — run 'cargo build --release' first" >&2
    exit 1
fi

TMP="$(mktemp -d -t illuminate-screenshots.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

echo "setting up sample repo at $TMP"
cd "$TMP"
git init -q

"$ILLUMINATE" init -n demo > /dev/null

mkdir -p .illuminate/wiki/decisions .illuminate/wiki/patterns .illuminate/wiki/failures .illuminate/wiki/modules

# Decision: no redis
cat > .illuminate/wiki/decisions/dec-no-redis.md <<'EOF'
---
id: dec-no-redis
title: No Redis for caching
page_type: decision
status: active
tags: [caching, infrastructure]
created: 2026-05-09T09:00:00Z
updated: 2026-05-09T09:00:00Z
---

## Decision

We do not use Redis for caching in this project.

## Context

Redis adds a stateful sidecar that breaks our deployment story (single binary + SQLite). The performance gain over a tuned in-memory LRU is rarely worth the operational overhead.

## Consequences

- Caches are in-memory LRUs with TTL, scoped per-process.
- Cache stampedes are mitigated with ±10% jitter on TTLs.
EOF

# Pattern: LRU
cat > .illuminate/wiki/patterns/pat-lru-cache.md <<'EOF'
---
id: pat-lru-cache
title: LRU cache with TTL and jitter
page_type: pattern
status: active
tags: [caching]
created: 2026-05-09T09:00:00Z
updated: 2026-05-09T09:00:00Z
---

## Pattern

Use an in-process LRU cache with a TTL plus ±10% jitter for any frequently-read, infrequently-written data.

## Implementation

```rust
let cache = lru::LruCache::new(NonZeroUsize::new(1024).unwrap());
let ttl = Duration::from_secs(30) + jitter(0.1);
```
EOF

# Failure: cache stampede
cat > .illuminate/wiki/failures/fail-cache-stampede.md <<'EOF'
---
id: fail-cache-stampede
title: Cache stampede on hot keys
page_type: failure
status: active
tags: [caching, incident]
created: 2026-05-09T09:00:00Z
updated: 2026-05-09T09:00:00Z
---

## Root Cause

No jitter on cache TTLs. Hot keys expired simultaneously across replicas; thundering-herd backfill spiked DB load.

## Fix

Added ±10% jitter to all TTLs.

## Lesson for future agents

Always jitter cache TTLs.
EOF

# Module: payments
cat > .illuminate/wiki/modules/mod-payments.md <<'EOF'
---
id: mod-payments
title: Payments service
page_type: module
status: active
tags: [service]
created: 2026-05-09T09:00:00Z
updated: 2026-05-09T09:00:00Z
---

## Overview

Handles checkout, refunds, and idempotent retry of failed transactions. Caches read-heavy account state with the LRU pattern.
EOF

# Policy
cat > .illuminate/illuminate.toml <<'EOF'
[project]
name = "demo"

[extraction]
confidence_threshold = 0.5

[audit]
semantic_top_k = 5
semantic_threshold = 0.0

[policies.no_redis]
rule = "rejected_pattern"
pattern = "Redis"
reason = "Use in-memory LRU with TTL instead — see dec-no-redis"
severity = "error"
decision_ref = "dec-no-redis"
EOF

"$ILLUMINATE" wiki rebuild > /dev/null

echo
echo "════════════════════════════════════════════════════════════════"
echo "  Sample repo ready. Starting wiki serve on port $PORT..."
echo "════════════════════════════════════════════════════════════════"
echo
echo "  Open these URLs in a browser and capture screenshots:"
echo
echo "    Home:               http://127.0.0.1:$PORT/"
echo "    Page view:          http://127.0.0.1:$PORT/page/decisions/dec-no-redis"
echo "    Decisions list:     http://127.0.0.1:$PORT/decisions"
echo "    Search:             http://127.0.0.1:$PORT/search?q=caching"
echo "    Audit playground:   http://127.0.0.1:$PORT/audit"
echo
echo "  At the audit playground, paste:"
echo "    add Redis caching to txn lookup"
echo "  and capture the result page (showing the violation)."
echo
echo "  Press Ctrl-C to tear down the tempdir."
echo

exec "$ILLUMINATE" wiki serve --port "$PORT"

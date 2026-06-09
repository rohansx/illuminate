---
id: dec-bs-readme-readme-md-aha-moment
title: Aha moment
type: decision
status: active
created: 2026-06-09T10:23:42.298689593+00:00
updated: 2026-06-09T10:23:42.298689593+00:00
tags: ["bootstrap", "readme"]
confidence: 0.5
sources:
  - kind: readme
    ref: README.md
---

## Decision

```
> Add Redis caching to txn lookup endpoint

illuminate_audit response:

  Warning: dec-2025-12-no-redis-payments
    Team rejected Redis for this service 3 months ago.
    Reason: deployment target disallows stateful sidecars.
    Alternative: pat-lru-cache-with-ttl (LRU with 30s TTL).
    See wiki/decisions/2025-12-no-redis-payments.md

  Status: warn
```

The agent surfaces the past decision to the dev and proposes the LRU pattern instead. The dev didn't have to remember. The agent didn't have to guess.

---

## Context

Extracted from README.md during bootstrap.

## Consequences

_Drafted from project README; review for accuracy._


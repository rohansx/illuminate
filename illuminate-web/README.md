# illuminate-web

Static web frontend for Illuminate: a marketing **landing page** and an ops
**dashboard**.

## Files

| File | Page | Notes |
|------|------|-------|
| `index.html` | Landing — "agent context engineering for coding teams" | section anchors only (`#shift`, `#pipeline`, `#surfaces`, `#trust`, `#day`, `#arch`, `#aud`, `#landscape`, `#plan`) |
| `dashboard.html` | Ops dashboard — per-project (`payments-service` in the mock) | tabs: overview, live, sessions, knowledge, graph, audit |

## Status: static mockups

Both pages are **static HTML** with **no inline styles** and **no live data**.
The dashboard's figures (`graph nodes 1,247`, `enrich p50 38ms`, the live feed,
sessions, audit rows) are hard-coded placeholders carried on `data-*`
attributes (`data-metric`, `data-clock`, `data-copy`, …) — ready to be wired to
a live source but not yet connected.

## Missing assets (required to render)

Both pages `<link>` to stylesheets that are **not yet in this repo**:

- `illuminate-v4.css` — shared theme (referenced by both pages)
- `illuminate-dashboard.css` — dashboard-only layout (referenced by `dashboard.html`)

Until these are added, the pages render unstyled. The class vocabulary is
bespoke and semantic (`sec-marker`, `sec-head`, `crate`, `aud-row`, `metric`,
`badge`, `session`, …) — it is **not** a utility framework, so a CDN cannot
substitute for the real stylesheets.

Typography is pulled from Google Fonts via CDN (Geist, Geist Mono, Inter Tight,
JetBrains Mono) — no local font assets needed.

## Next steps

1. **Add the two CSS files** (`illuminate-v4.css`, `illuminate-dashboard.css`).
2. **Wire the dashboard to live data** — replace the `data-*` placeholders with
   values from `illuminate serve` (the binary already serves a wiki dashboard +
   JSON API on `:8765`; the stats/decisions/failures endpoints can back the
   overview, knowledge, and audit panels).
3. **Serve from the binary (optional)** — host these pages from `illuminate
   serve` so `illuminate-web` is reachable alongside the existing dashboard.

## Preview locally

```bash
# from this directory (once the CSS files are present)
python3 -m http.server 8080
# then open http://localhost:8080/index.html
```

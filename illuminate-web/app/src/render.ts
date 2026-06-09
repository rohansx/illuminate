// Section renderers. Each takes real API data and returns DOM. They render
// ONLY the fields declared in types.ts — there is no code path that can emit
// a prompt feed, latency, cpu, publish count, or audit history, because none
// of those fields exist in the contract.

import type { Dashboard, GraphSource, RecentItem } from "./types.ts";
import { num, pct, relativeTime } from "./format.ts";
import { div, el, text } from "./dom.ts";

// ---- header --------------------------------------------------------------
export function renderHeader(d: Dashboard): HTMLElement {
  const summary = `${num(d.graph.episodes)} episodes · ${num(d.stats.decisions)} decisions · ${pct(
    d.tokens.cache_saved_pct,
  )} cache-saved`;

  const crumbs = div("crumbs", []);
  crumbs.append(text("span", "", d.project));
  crumbs.append(text("span", "sep", "/"));
  crumbs.append(text("span", "now", `generated ${relativeTime(d.generated_at)}`));

  const h1 = el("h1");
  h1.append(text("span", "", d.project));
  h1.append(text("span", "repo", "dashboard"));

  return el("header", { class: "dash-head" }, [
    div("where", [crumbs, h1, text("p", "summary-line", summary)]),
  ]);
}

// ---- stat cards (.kpi) ---------------------------------------------------
interface Kpi {
  k: string;
  v: string;
  tone: string;
  note?: string;
}

function kpiCard(c: Kpi): HTMLElement {
  const head = div("k", [text("span", "", c.k)]);
  const value = text("div", "v", c.v);
  const children: HTMLElement[] = [head, value];
  if (c.note) children.push(text("div", "note", c.note));
  return div(`kpi ${c.tone}`, children);
}

export function renderStats(d: Dashboard): HTMLElement {
  const cards: Kpi[] = [
    { k: "decisions", v: num(d.stats.decisions), tone: "teal" },
    { k: "patterns", v: num(d.stats.patterns), tone: "sage" },
    { k: "failures", v: num(d.stats.failures), tone: "rust" },
    { k: "modules", v: num(d.stats.modules), tone: "lilac" },
    { k: "episodes", v: num(d.graph.episodes), tone: "amber", note: "graph nodes" },
    { k: "captured sessions", v: num(d.tokens.sessions), tone: "teal" },
    { k: "cache-saved", v: pct(d.tokens.cache_saved_pct), tone: "sage" },
  ];
  return div(
    "kpis kpis-7",
    cards.map(kpiCard),
  );
}

// ---- knowledge sources (the real centerpiece) ----------------------------
export function renderSources(sources: GraphSource[]): HTMLElement {
  const head = el("div", { class: "ph" }, []);
  head.append(text("span", "label", "graph"));
  head.append(text("span", "title", "Knowledge sources"));
  head.append(text("span", "sub", `${num(sources.length)} sources`));

  const body = div("pb tight", []);

  if (sources.length === 0) {
    body.append(text("p", "empty", "no graph sources recorded yet"));
    return div("panel amber", [head, body]);
  }

  const sorted = [...sources].sort((a, b) => b.count - a.count);
  const max = Math.max(...sorted.map((s) => s.count), 1);

  const list = div("src-list", []);
  for (const s of sorted) {
    const widthPct = Math.max(2, Math.round((s.count / max) * 100));

    const labelRow = div("src-top", [
      text("span", "src-name", s.source),
      text("span", "src-count", num(s.count)),
    ]);

    const bar = div("src-bar", []);
    const fill = div("src-fill", []);
    fill.style.width = `${widthPct}%`;
    bar.append(fill);

    list.append(div("src-row", [labelRow, bar]));
  }
  body.append(list);

  return div("panel amber", [head, body]);
}

// ---- recent decisions / failures -----------------------------------------
function recentRow(item: RecentItem, kind: "dec" | "fail"): HTMLElement {
  const bodyChildren: HTMLElement[] = [text("div", "name", item.title || item.id)];

  const meta = div("meta", []);
  meta.append(text("span", "ago", item.ago));
  const tags = item.tags ?? [];
  for (const t of tags) {
    meta.append(text("span", "ref", `#${t}`));
  }
  bodyChildren.push(meta);

  const badge = text("span", "badge", kind === "fail" ? "failure" : "decision");
  return div(`card-row ${kind}`, [div("body", bodyChildren), badge]);
}

export function renderRecent(
  title: string,
  label: string,
  items: RecentItem[],
  kind: "dec" | "fail",
  emptyText: string,
  tone: string,
): HTMLElement {
  const head = el("div", { class: "ph" }, []);
  head.append(text("span", "label", label));
  head.append(text("span", "title", title));
  head.append(text("span", "sub", `${num(items.length)}`));

  const body = div("pb tight", []);
  if (items.length === 0) {
    body.append(text("p", "empty", emptyText));
  } else {
    const list = div("card-list", items.map((it) => recentRow(it, kind)));
    body.append(list);
  }
  return div(`panel ${tone}`, [head, body]);
}

// ---- token savings -------------------------------------------------------
export function renderTokens(d: Dashboard): HTMLElement {
  const head = el("div", { class: "ph" }, []);
  head.append(text("span", "label", "tokens"));
  head.append(text("span", "title", "Token savings"));
  head.append(text("span", "sub", `${num(d.tokens.sessions)} sessions`));

  const rows: Array<[string, string, string]> = [
    ["cache-saved", pct(d.tokens.cache_saved_pct), "amber"],
    ["cache read", num(d.tokens.cache_read), "sage"],
    ["cache creation", num(d.tokens.cache_creation), ""],
    ["input", num(d.tokens.input), ""],
    ["output", num(d.tokens.output), ""],
    ["sessions", num(d.tokens.sessions), ""],
  ];

  const grid = div("breakdown tok-grid", []);
  for (const [k, v, tone] of rows) {
    grid.append(div(`row ${tone}`.trim(), [text("span", "k", k), text("span", "v", v)]));
  }

  const body = div("pb", [grid]);
  return div("panel sage", [head, body]);
}

// ---- states --------------------------------------------------------------
export function renderLoading(): HTMLElement {
  return div("state-panel", [
    div("state-spinner", []),
    text("p", "state-title", "loading dashboard…"),
    text("p", "state-sub", "fetching /api/dashboard"),
  ]);
}

export function renderError(message: string): HTMLElement {
  const panel = div("panel rust state-panel", []);
  panel.append(text("p", "state-title", "couldn’t reach /api/dashboard"));
  panel.append(text("p", "state-sub", "start the server with: illuminate wiki serve"));
  panel.append(text("p", "state-detail", message));
  return panel;
}

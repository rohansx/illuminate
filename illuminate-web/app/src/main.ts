// illuminate · data-driven dashboard entry point.
//
// On load it fetches the live GET /api/dashboard (relative -> same origin as
// the binary that serves this single-file build) and renders ONLY the real
// fields. There is no demo data anywhere: the loading and error paths render
// honest states, never fabricated rows.

import "./illuminate-v4.css";
import "./illuminate-dashboard.css";
import "./dashboard-app.css";

import type { Dashboard } from "./types.ts";
import { div, text } from "./dom.ts";
import {
  renderError,
  renderHeader,
  renderLoading,
  renderRecent,
  renderSources,
  renderStats,
  renderTokens,
} from "./render.ts";

const root = document.getElementById("app");

function mount(node: HTMLElement): void {
  if (!root) return;
  root.replaceChildren(node);
}

function mountAll(nodes: HTMLElement[]): void {
  if (!root) return;
  root.replaceChildren(...nodes);
}

function renderDashboard(d: Dashboard): void {
  const header = renderHeader(d);
  const stats = renderStats(d);

  // Sources is the centerpiece — pair it with token savings beside it.
  const topGrid = div("dash-grid", [renderSources(d.graph.sources ?? []), renderTokens(d)]);

  const decisions = renderRecent(
    "Recent decisions",
    "knowledge",
    d.recent_decisions ?? [],
    "dec",
    "no decisions recorded yet",
    "teal",
  );
  const failures = renderRecent(
    "Recent failures",
    "knowledge",
    d.recent_failures ?? [],
    "fail",
    "no failures recorded",
    "rust",
  );
  const recentGrid = div("dash-grid", [decisions, failures]);

  mountAll([
    header,
    text("p", "section-h", "overview"),
    stats,
    text("p", "section-h", "knowledge graph · token savings"),
    topGrid,
    text("p", "section-h", "recent activity"),
    recentGrid,
  ]);
}

async function load(): Promise<void> {
  mount(renderLoading());
  try {
    const resp = await fetch("/api/dashboard", { headers: { accept: "application/json" } });
    if (!resp.ok) {
      throw new Error(`HTTP ${resp.status} ${resp.statusText}`);
    }
    const data = (await resp.json()) as Dashboard;
    if (!data || typeof data !== "object" || !data.stats || !data.graph || !data.tokens) {
      throw new Error("unexpected response shape from /api/dashboard");
    }
    renderDashboard(data);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    mount(renderError(message));
  }
}

void load();

// illuminate · interactive, data-driven dashboard entry point.
//
// Shell: a fixed left rail (Overview / Knowledge / Sources / Tokens), a topbar
// with live search, and a view container. View switching is plain in-app state
// (show/hide) — no router. Every decision/failure/page row is clickable and
// opens a right slide-over that renders the page's markdown body via `marked`.
//
// Strictly data-driven: the Overview hydrates from GET /api/dashboard, Knowledge
// from /api/pages, search from /api/search, the detail panel from /api/page/<id>.
// Loading, fetch-error, and empty states are honest — there is NO demo data and
// no code path that can fabricate a row.

import "./illuminate-v4.css";
import "./illuminate-dashboard.css";
import "./dashboard-app.css";

import type { Dashboard } from "./types.ts";
import { div, el, text } from "./dom.ts";
import { fetchDashboard } from "./api.ts";
import { openDetail } from "./detail.ts";
import { createRail, type ViewId } from "./nav.ts";
import { createSearch } from "./search.ts";
import { mountKnowledge } from "./knowledge.ts";
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

// ---- shell scaffold (built once, populated on data load) -----------------
const view = div("view", []); // the swappable main content region
let currentView: ViewId = "overview";
let dashboard: Dashboard | null = null;

const { rail, setActive } = createRail((id) => selectView(id));
const { control: searchControl, results: searchResults } = createSearch((id) => openDetail(id));

const topbar = el("div", { class: "topbar app-topbar" }, [
  div("system", [el("span", { class: "dot" }), text("b", "", "illuminate"), text("span", "pipe", "/"), text("span", "", "wiki")]),
  searchControl,
]);

const shell = div("shell", [topbar, searchResults, view]);

function mountShell(): void {
  if (!root) return;
  root.replaceChildren(rail, shell);
}

// ---- view renderers --------------------------------------------------------
function overviewView(d: Dashboard): HTMLElement {
  const header = renderHeader(d);
  const stats = renderStats(d);

  const topGrid = div("dash-grid", [renderSources(d.graph.sources ?? []), renderTokens(d)]);

  const decisions = renderRecent(
    "Recent decisions",
    "knowledge",
    d.recent_decisions ?? [],
    "dec",
    "no decisions recorded yet",
    "teal",
    openDetail,
  );
  const failures = renderRecent(
    "Recent failures",
    "knowledge",
    d.recent_failures ?? [],
    "fail",
    "no failures recorded",
    "rust",
    openDetail,
  );
  const recentGrid = div("dash-grid", [decisions, failures]);

  return div("stack", [
    header,
    text("p", "section-h", "overview"),
    stats,
    text("p", "section-h", "knowledge graph · token savings"),
    topGrid,
    text("p", "section-h", "recent activity"),
    recentGrid,
  ]);
}

function sourcesView(d: Dashboard): HTMLElement {
  return div("stack", [
    text("p", "section-h", "knowledge sources"),
    renderSources(d.graph.sources ?? []),
  ]);
}

function tokensView(d: Dashboard): HTMLElement {
  return div("stack", [text("p", "section-h", "token savings"), renderTokens(d)]);
}

// ---- view switching --------------------------------------------------------
function selectView(id: ViewId): void {
  currentView = id;
  setActive(id);

  if (id === "knowledge") {
    // Knowledge fetches its own data (lazy) every time it is opened.
    mountKnowledge(view, openDetail);
    return;
  }

  if (!dashboard) return; // overview/sources/tokens need the dashboard envelope
  switch (id) {
    case "overview":
      view.replaceChildren(overviewView(dashboard));
      break;
    case "sources":
      view.replaceChildren(sourcesView(dashboard));
      break;
    case "tokens":
      view.replaceChildren(tokensView(dashboard));
      break;
  }
}

// ---- boot ------------------------------------------------------------------
async function load(): Promise<void> {
  if (!root) return;
  mountShell();
  setActive("overview");
  view.replaceChildren(renderLoading());

  try {
    dashboard = await fetchDashboard();
    selectView(currentView === "knowledge" ? "knowledge" : "overview");
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    view.replaceChildren(renderError(message));
  }
}

void load();

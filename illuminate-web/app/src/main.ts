// illuminate · interactive, data-driven dashboard entry point.
//
// Shell: a skip link, a fixed left rail (desktop) plus a horizontal tab strip
// (mobile), a topbar with live search, and a view container. View switching is
// plain in-app state (show/hide) — no router. Every decision/failure/page row
// is clickable and opens a right slide-over that renders the page's markdown
// body via `marked`. Source rows drill into their graph-episode lists, and
// each episode opens the same slide-over with its raw content.
//
// Strictly data-driven: the Overview hydrates from GET /api/dashboard,
// Knowledge from /api/pages, search from /api/search, the detail panel from
// /api/page/<id>, episodes from /api/episodes + /api/episode/<id>. Loading,
// fetch-error, and empty states are honest — there is NO demo data and no
// code path that can fabricate a row.

import "./illuminate-v4.css";
import "./illuminate-dashboard.css";
import "./dashboard-app.css";

import type { Dashboard } from "./types.ts";
import { div, el, text } from "./dom.ts";
import { fetchDashboard } from "./api.ts";
import { openDetail, openEpisode } from "./detail.ts";
import { createNav, type ViewId } from "./nav.ts";
import { createSearch } from "./search.ts";
import { mountKnowledge } from "./knowledge.ts";
import { mountEpisodes } from "./episodes.ts";
import {
  renderError,
  renderHeader,
  renderLoading,
  renderRecent,
  renderSources,
  renderStats,
  renderTokens,
} from "./render.ts";

/** Overview shows only the head of the source distribution; the Sources view
 *  has the full list. Keeps the long tail of count-1 sources off the
 *  Overview so the two-column grid stays balanced. */
const OVERVIEW_SOURCES_MAX = 8;

const root = document.getElementById("app");

// ---- shell scaffold (built once, populated on data load) -----------------
// The view container is the skip-link target, hence id + tabindex.
const view = el("div", { class: "view", id: "main", tabindex: "-1" });
let currentView: ViewId = "overview";
let dashboard: Dashboard | null = null;
let selectedSource: string | null = null; // Sources-view episode drill-down

const { rail, mobileNav, setActive } = createNav((id) => selectView(id));
const { control: searchControl, results: searchResults } = createSearch((id) => openDetail(id));

const topbar = el("div", { class: "topbar app-topbar" }, [
  div("system", [el("span", { class: "dot" }), text("b", "", "illuminate"), text("span", "pipe", "/"), text("span", "", "wiki")]),
  searchControl,
]);

const shell = div("shell", [topbar, mobileNav, searchResults, view]);

const skipLink = el("a", { class: "skip-link", href: "#main" });
skipLink.textContent = "skip to content";

function mountShell(): void {
  if (!root) return;
  root.replaceChildren(skipLink, rail, shell);
}

// ---- view renderers --------------------------------------------------------
/** Drill into one source's episode list (rendered inside the Sources view). */
function openSource(source: string): void {
  selectedSource = source;
  selectView("sources", { keepSource: true });
}

function overviewView(d: Dashboard): HTMLElement {
  const header = renderHeader(d);
  const stats = renderStats(d);

  const sourcesPanel = renderSources(d.graph.sources ?? [], {
    onOpenSource: openSource,
    maxRows: OVERVIEW_SOURCES_MAX,
    onViewAll: () => selectView("sources"),
  });
  const topGrid = div("dash-grid", [sourcesPanel, renderTokens(d)]);

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
    renderSources(d.graph.sources ?? [], { onOpenSource: openSource }),
  ]);
}

function tokensView(d: Dashboard): HTMLElement {
  return div("stack", [
    text("p", "section-h", "token savings"),
    renderTokens(d, { detailed: true }),
  ]);
}

// ---- view switching --------------------------------------------------------
function selectView(id: ViewId, opts: { keepSource?: boolean } = {}): void {
  currentView = id;
  setActive(id);
  if (!opts.keepSource) selectedSource = null;

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
      if (selectedSource) {
        mountEpisodes(view, selectedSource, openEpisode, () => selectView("sources"));
      } else {
        view.replaceChildren(sourcesView(dashboard));
      }
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

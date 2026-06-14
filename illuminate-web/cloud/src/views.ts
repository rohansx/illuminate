// The four workspace views, each a pure render function over the live
// /api/workspace snapshot. No fabricated rows: an empty array renders an honest
// empty panel.

import { div, el, text } from "./dom.ts";
import { cleanPreview, initials, num } from "./format.ts";
import type { FeedItem, Member, Repo, Strata, Workspace } from "./types.ts";

export type OpenRepo = (id: string) => void;

function statCard(k: string, v: number, sub: string, pulse = false): HTMLElement {
  const card = div("stat", [text("div", "stat-k", k), text("div", "stat-v", num(v))]);
  if (sub) card.append(text("div", "stat-d", sub));
  if (pulse) card.append(div("pulse-dot"));
  return card;
}

function statsRow(ws: Workspace): HTMLElement {
  const t = ws.totals;
  const reposSub =
    t.uninitialized > 0 ? `${num(t.scanned)} scanned · ${num(t.uninitialized)} uninitialized` : `${num(t.scanned)} scanned`;
  return div("stats", [
    statCard("Repositories", t.repos, reposSub),
    statCard("Episodes", t.episodes, "across the workspace", true),
    statCard("Entities", t.entities, `${num(t.edges)} edges`),
    statCard("Contributors", t.contributors, `${num(t.decisions)} decisions`),
  ]);
}

function healthCell(r: Repo): HTMLElement {
  return el("span", { class: `health ${r.health}` }, [el("i"), r.health]);
}

function repoRow(r: Repo, maxEp: number, open: OpenRepo): HTMLElement {
  const pct = maxEp > 0 ? Math.round((r.episodes / maxEp) * 100) : 0;
  const bar = div("bar", [el("i")]);
  (bar.firstChild as HTMLElement).setAttribute("style", `width:${pct}%`);

  const tr = el("tr", { class: "clickable", tabindex: "0", "data-repo": r.id }, [
    el("td", {}, [
      div("repo", [
        div("repo-glyph"),
        div("", [text("div", "repo-name", r.name), text("div", "repo-org", r.top_source || r.path)]),
      ]),
    ]),
    el("td", {}, [healthCell(r)]),
    el("td", { class: "num" }, [num(r.episodes)]),
    el("td", { class: "num" }, [num(r.entities)]),
    el("td", { class: "num" }, [num(r.decisions)]),
    el("td", {}, [el("span", { class: "pct" }, [bar])]),
    el("td", { class: "num" }, [r.ago || "—"]),
  ]);
  const go = () => open(r.id);
  tr.addEventListener("click", go);
  tr.addEventListener("keydown", (e) => {
    if ((e as KeyboardEvent).key === "Enter") go();
  });
  return tr;
}

function reposTable(repos: Repo[], open: OpenRepo): HTMLElement {
  const maxEp = repos.reduce((m, r) => Math.max(m, r.episodes), 0);
  const head = el("tr", {}, [
    el("th", {}, ["Repository"]),
    el("th", {}, ["Health"]),
    el("th", {}, ["Episodes"]),
    el("th", {}, ["Entities"]),
    el("th", {}, ["Decisions"]),
    el("th", {}, ["Density"]),
    el("th", {}, ["Active"]),
  ]);
  const body = repos.map((r) => repoRow(r, maxEp, open));
  return el("table", { class: "table" }, [el("thead", {}, [head]), el("tbody", {}, body)]);
}

function emptyPanelBody(msg: string): HTMLElement {
  return div("state", [text("p", "", msg)]);
}

function feedList(feed: FeedItem[]): HTMLElement {
  if (feed.length === 0) return emptyPanelBody("No activity captured yet.");
  const items = feed.map((f) =>
    el("li", {}, [
      text("div", "feed-time", f.ago),
      div("feed-body", [
        el("b", {}, [f.repo]),
        text("p", "", cleanPreview(f.preview).slice(0, 160) || "(no preview)"),
      ]),
      text("div", "feed-tag", f.source.split(":")[0]),
    ]),
  );
  return el("ul", { class: "feed" }, items);
}

function strataPanel(s: Strata): HTMLElement {
  const cells =
    s.levels.length > 0
      ? s.levels.map((lvl, i) =>
          el("i", {
            class: lvl > 0 ? `l${lvl}` : "",
            title: `${s.days[i] ?? ""}: ${s.counts[i] ?? 0} episodes`,
          }),
        )
      : [];
  const grid =
    cells.length > 0 ? div("strata", cells) : emptyPanelBody("No activity in the last 28 days.");
  const legend = div("strata-legend", [
    "less",
    el("i", { class: "l1" }),
    el("i", { class: "l2" }),
    el("i", { class: "l3" }),
    el("i", { class: "l4" }),
    el("i", { class: "l5" }),
    "more",
  ]);
  return div("panel", [
    div("panel-head", [
      text("h3", "panel-title", "Activity — last 28 days"),
      text("div", "panel-meta", `peak ${num(s.max)}/day`),
    ]),
    grid,
    cells.length > 0 ? legend : div(""),
  ]);
}

function panel(title: string, meta: string, body: HTMLElement): HTMLElement {
  return div("panel", [
    div("panel-head", [
      text("h3", "panel-title", title),
      meta ? text("div", "panel-meta", meta) : div(""),
    ]),
    body,
  ]);
}

function head(eyebrow: string, title: string, sub: string): HTMLElement {
  return div("view-head", [
    div("", [text("div", "view-eyebrow", eyebrow), text("h1", "view-title", title)]),
    sub ? text("p", "view-sub", sub) : div(""),
  ]);
}

export function renderOverview(ws: Workspace, open: OpenRepo): HTMLElement {
  const top = [...ws.repos].slice(0, 8);
  return div("", [
    head(
      "workspace overview",
      "Your knowledge graph, across every repo.",
      `Aggregated live from ${num(ws.totals.repos)} populated repositories (${num(ws.totals.scanned)} scanned) under ${ws.root}. Every number traces to a real graph.db — nothing here is mocked.`,
    ),
    statsRow(ws),
    div("grid-2", [
      panel("Recent activity", `${ws.feed.length} events`, feedList(ws.feed.slice(0, 12))),
      panel(
        "Top repositories",
        `${ws.totals.active_repos} active`,
        ws.repos.length > 0 ? reposTable(top, open) : emptyPanelBody("No repositories found."),
      ),
    ]),
    strataPanel(ws.strata),
  ]);
}

export function renderRepositories(ws: Workspace, open: OpenRepo): HTMLElement {
  return div("", [
    head(
      "repositories",
      "Every repo in the workspace.",
      "Click a repository to drill into its graph episodes, sources, and contributors.",
    ),
    panel(
      "Repositories",
      `${num(ws.totals.repos)} populated · ${num(ws.totals.uninitialized)} uninitialized`,
      ws.repos.length > 0
        ? reposTable(ws.repos, open)
        : emptyPanelBody("No populated .illuminate repos found under the scan root."),
    ),
  ]);
}

export function renderMembers(ws: Workspace): HTMLElement {
  const body =
    ws.members.length > 0
      ? el(
          "ul",
          { class: "members" },
          ws.members.map((m: Member) =>
            el("li", {}, [
              el("div", { class: "avatar" }, [initials(m.name || m.email)]),
              div("", [text("div", "me-name", m.name || m.email), text("div", "repo-org", m.email)]),
              text("div", "num", `${num(m.commits)} commits · ${num(m.repos)} repos`),
              text("div", `role ${m.role === "owner" ? "owner" : ""}`, m.role),
            ]),
          ),
        )
      : emptyPanelBody("No git contributors found across the workspace.");
  return div("", [
    head(
      "members",
      "Who built this.",
      "Contributors aggregated from git history across every repo. Roles are derived from commit volume — there is no separate org directory.",
    ),
    panel("Contributors", `${num(ws.members.length)} people`, body),
  ]);
}

export function renderActivity(ws: Workspace): HTMLElement {
  return div("", [
    head("activity", "Everything, newest first.", "The merged episode feed across every repo."),
    panel("Activity feed", `${ws.feed.length} events`, feedList(ws.feed)),
  ]);
}

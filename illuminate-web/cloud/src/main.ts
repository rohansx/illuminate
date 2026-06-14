// Illuminate Cloud — Teams: a data-driven, multi-repo workspace dashboard.
//
// Strictly live: the whole UI hydrates from ONE GET /api/workspace (a real
// aggregation of every .illuminate repo under the scan root). A fetch failure
// renders an honest error screen; an empty workspace renders empty panels.
// There is NO demo data and no code path that can fabricate a repo, episode, or
// contributor.

import "./cloud.css";

import { clear, div, el, text } from "./dom.ts";
import { num } from "./format.ts";
import { fetchWorkspace } from "./api.ts";
import { openRepoDetail } from "./detail.ts";
import { renderActivity, renderMembers, renderOverview, renderRepositories } from "./views.ts";
import type { Workspace } from "./types.ts";

type ViewId = "overview" | "repos" | "members" | "activity";

const NAV: { id: ViewId; label: string; badge: (ws: Workspace) => number }[] = [
  { id: "overview", label: "Overview", badge: (ws) => ws.totals.repos },
  { id: "repos", label: "Repositories", badge: (ws) => ws.totals.repos },
  { id: "members", label: "Members", badge: (ws) => ws.totals.contributors },
  { id: "activity", label: "Activity", badge: (ws) => ws.feed.length },
];

const root = document.getElementById("root")!;

function loadingScreen(): void {
  clear(root);
  root.append(
    div("state", [div("spinner"), text("p", "", "scanning the workspace…")]),
  );
}

function errorScreen(message: string): void {
  clear(root);
  root.append(
    div("state", [
      text("h2", "", "couldn't reach the workspace API"),
      el("p", {}, [
        "start it with ",
        el("code", {}, ["illuminate cloud serve --root <dir>"]),
        " and reload.",
      ]),
      text("p", "", message),
    ]),
  );
}

function viewBody(id: ViewId, ws: Workspace): HTMLElement {
  switch (id) {
    case "overview":
      return renderOverview(ws, openRepoDetail);
    case "repos":
      return renderRepositories(ws, openRepoDetail);
    case "members":
      return renderMembers(ws);
    case "activity":
      return renderActivity(ws);
  }
}

function shell(ws: Workspace): void {
  clear(root);
  let current: ViewId = "overview";

  const skip = el("a", { class: "skip", href: "#view" }, ["Skip to content"]);

  // sidebar
  const rootName = ws.root.split("/").filter(Boolean).pop() || ws.root;
  const org = div("org", [
    div("org-mark"),
    div("", [
      text("div", "org-name", "Workspace"),
      text("div", "org-meta", `${rootName} · ${num(ws.totals.repos)} repos`),
    ]),
  ]);

  const sideLinks: HTMLButtonElement[] = [];
  const mnavBtns: HTMLButtonElement[] = [];

  const side = el("aside", { class: "side" }, [org, text("div", "side-section", "Workspace")]);
  for (const item of NAV) {
    const b = el("button", { class: "side-link", type: "button" }, [
      item.label,
      el("span", { class: "badge" }, [num(item.badge(ws))]),
    ]) as HTMLButtonElement;
    b.addEventListener("click", () => select(item.id));
    sideLinks.push(b);
    side.append(b);
  }
  const topMember = ws.members[0];
  side.append(
    div("side-foot", [
      el("div", { class: "avatar" }, [topMember ? (topMember.name || "?").slice(0, 2).toUpperCase() : "··"]),
      div("", [
        text("div", "me-name", topMember ? topMember.name || topMember.email : "no contributors"),
        text("div", "me-role", topMember ? topMember.role : "—"),
      ]),
    ]),
  );

  // main
  const crumbs = div("crumbs", [
    el("span", {}, []),
    "cloud",
    el("span", {}, [" / "]),
    el("b", {}, ["workspace"]),
    el("span", {}, [" / "]),
  ]);
  const crumbView = el("b", {}, ["overview"]);
  crumbs.append(crumbView);
  const genAt = ws.generated_at ? new Date(ws.generated_at).toLocaleString() : "";
  const topbar = div("topbar", [crumbs, text("div", "topbar-meta", genAt ? `snapshot ${genAt}` : "")]);

  const mnav = div("mnav");
  for (const item of NAV) {
    const b = el("button", { type: "button" }, [item.label]) as HTMLButtonElement;
    b.addEventListener("click", () => select(item.id));
    mnavBtns.push(b);
    mnav.append(b);
  }

  const view = el("main", { class: "view", id: "view" });
  const main = div("main", [topbar, mnav, view]);

  root.append(skip, div("app", [side, main]));

  function select(id: ViewId): void {
    current = id;
    NAV.forEach((item, i) => {
      sideLinks[i].classList.toggle("on", item.id === id);
      mnavBtns[i].classList.toggle("on", item.id === id);
    });
    crumbView.textContent = NAV.find((n) => n.id === id)!.label.toLowerCase();
    clear(view);
    view.append(viewBody(id, ws));
    view.scrollTop = 0;
  }

  select(current);
}

async function boot(): Promise<void> {
  loadingScreen();
  try {
    const ws = await fetchWorkspace();
    shell(ws);
  } catch (err) {
    errorScreen(err instanceof Error ? err.message : String(err));
  }
}

boot();

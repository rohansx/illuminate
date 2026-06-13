// Knowledge view: fetches the full page list (GET /api/pages) and renders it
// grouped into tabbed sections — decisions / patterns / failures — every row
// clickable into the detail slide-over. Module pages are listed under their own
// tab too. Per-group empty states; honest loading + fetch-error states. No
// fabricated rows — only what /api/pages returns.

import type { PageListItem } from "./types.ts";
import { div, el, text } from "./dom.ts";
import { num, relativeTime } from "./format.ts";
import { fetchPages } from "./api.ts";
import type { OpenPage } from "./render.ts";

interface Group {
  key: string;
  label: string;
  tone: string; // dec | pat | fail | (none)
  empty: string;
}

// `key` matches the API's page `type` field, which is the DIR name (plural:
// "decisions"/"patterns"/"failures"/"modules") — not the singular form.
const GROUPS: Group[] = [
  { key: "decisions", label: "decisions", tone: "dec", empty: "no decisions recorded yet" },
  { key: "patterns", label: "patterns", tone: "pat", empty: "no patterns recorded yet" },
  { key: "failures", label: "failures", tone: "fail", empty: "no failures recorded yet" },
  { key: "modules", label: "modules", tone: "", empty: "no modules recorded yet" },
];

// No per-row type badge: the tab label and panel title already name the type
// for every row in these single-type panels — a repeated badge is pure noise.
function pageRow(item: PageListItem, group: Group, onOpen: OpenPage): HTMLElement {
  const bodyChildren: HTMLElement[] = [text("div", "name", item.title || item.id)];

  const meta = div("meta", []);
  const stamp = item.updated ?? item.created;
  if (stamp) meta.append(text("span", "ago", relativeTime(stamp)));
  if (item.status) meta.append(text("span", "ref", item.status));
  for (const t of item.tags ?? []) meta.append(text("span", "ref", `#${t}`));
  bodyChildren.push(meta);

  const row = el("button", { class: `card-row clickable ${group.tone}`.trim(), type: "button" }, [
    div("body", bodyChildren),
  ]);
  row.addEventListener("click", () => onOpen(item.id));
  return row;
}

function groupPanel(group: Group, items: PageListItem[], onOpen: OpenPage): HTMLElement {
  const head = el("div", { class: "ph" }, []);
  head.append(text("span", "label", "knowledge"));
  head.append(text("span", "title", group.label));
  head.append(text("span", "sub", num(items.length)));

  const body = div("pb tight", []);
  if (items.length === 0) {
    body.append(text("p", "empty", group.empty));
  } else {
    body.append(div("card-list", items.map((it) => pageRow(it, group, onOpen))));
  }

  const tonePanel = group.tone === "dec" ? "teal" : group.tone === "fail" ? "rust" : group.tone === "pat" ? "sage" : "lilac";
  return div(`panel ${tonePanel} kn-group`, [head, body]);
}

function renderGroups(host: HTMLElement, all: PageListItem[], onOpen: OpenPage): void {
  // Tabs across the top; one panel visible at a time. Default to the first
  // non-empty group so the view never opens on an empty tab when data exists.
  const counts = new Map<string, PageListItem[]>();
  for (const g of GROUPS) counts.set(g.key, []);
  for (const p of all) {
    const bucket = counts.get(p.type);
    if (bucket) bucket.push(p);
  }

  const tabs = div("kn-tabs seg", []);
  const panels = div("kn-panels", []);

  let active = GROUPS.find((g) => (counts.get(g.key)?.length ?? 0) > 0)?.key ?? GROUPS[0].key;

  const tabButtons = new Map<string, HTMLElement>();
  const panelNodes = new Map<string, HTMLElement>();

  for (const g of GROUPS) {
    const items = counts.get(g.key) ?? [];
    const btn = el("button", { type: "button" });
    btn.textContent = `${g.label} · ${items.length}`;
    btn.addEventListener("click", () => select(g.key));
    tabButtons.set(g.key, btn);
    tabs.append(btn);

    const panel = groupPanel(g, items, onOpen);
    panelNodes.set(g.key, panel);
    panels.append(panel);
  }

  function select(key: string): void {
    active = key;
    for (const [k, btn] of tabButtons) btn.classList.toggle("on", k === key);
    for (const [k, panel] of panelNodes) panel.toggleAttribute("hidden", k !== key);
  }

  host.replaceChildren(tabs, panels);
  select(active);
}

/**
 * Mount the Knowledge view into `host`. Fetches /api/pages and renders tabbed,
 * clickable lists. Replaces host content with loading -> groups | empty | error.
 */
export function mountKnowledge(host: HTMLElement, onOpen: OpenPage): void {
  host.replaceChildren(
    div("state-panel", [div("state-spinner", []), text("p", "state-title", "loading knowledge…")]),
  );

  void (async () => {
    try {
      const pages = await fetchPages();
      if (pages.length === 0) {
        host.replaceChildren(
          div("panel", [
            el("div", { class: "ph" }, [text("span", "title", "Knowledge")]),
            div("pb", [text("p", "empty", "no pages recorded yet")]),
          ]),
        );
        return;
      }
      const wrap = div("kn-view", []);
      host.replaceChildren(wrap);
      renderGroups(wrap, pages, onOpen);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      const panel = div("panel rust state-panel", [
        text("p", "state-title", "couldn’t reach /api/pages"),
        text("p", "state-sub", "start the server with: illuminate wiki serve"),
        text("p", "state-detail", message),
      ]);
      host.replaceChildren(panel);
    }
  })();
}

// Docs view: lists every markdown file under the repo's `docs/` directory
// (fetched from GET /api/docs) grouped by its top-level subdirectory, and opens
// each one in the shared right slide-over via `openDoc` (raw markdown rendered
// with `marked`, same path as wiki pages and episodes).
//
// Strictly data-driven: the list comes only from the live endpoint. Loading,
// fetch-error, and empty states are honest — there is no fallback doc list.

import type { DocItem } from "./types.ts";
import { div, el, text } from "./dom.ts";
import { fetchDocs } from "./api.ts";
import { num } from "./format.ts";
import { renderError, renderLoading } from "./render.ts";

/** Title-case a group dir name for the section header ("old" -> "Old"). */
function groupLabel(group: string): string {
  if (!group) return "overview";
  return group;
}

/** One clickable doc row — opens the doc in the slide-over. */
function docRow(doc: DocItem, onOpen: (path: string) => void): HTMLElement {
  const row = el("button", { class: "card-row clickable", type: "button" }, [
    div("body", [
      text("div", "name", doc.title),
      div("meta", [text("span", "ref", `docs/${doc.path}`)]),
    ]),
  ]);
  row.addEventListener("click", () => onOpen(doc.path));
  return row;
}

/** Render one group panel (a section header + its doc rows). */
function groupPanel(group: string, docs: DocItem[], onOpen: (path: string) => void): HTMLElement {
  const head = el("div", { class: "ph" }, []);
  head.append(text("span", "label", "docs"));
  head.append(text("span", "title", groupLabel(group)));
  head.append(text("span", "sub", `${num(docs.length)} file${docs.length === 1 ? "" : "s"}`));
  const list = div("card-list", docs.map((d) => docRow(d, onOpen)));
  return div("panel lilac", [head, div("pb tight", [list])]);
}

/** Mount the Docs view into `host`: loading -> grouped doc list | error. */
export function mountDocs(host: HTMLElement, onOpen: (path: string) => void): void {
  host.replaceChildren(renderLoading());
  void (async () => {
    try {
      const { docs } = await fetchDocs();
      if (docs.length === 0) {
        host.replaceChildren(
          div("stack", [
            text("p", "section-h", "docs"),
            div("panel lilac", [
              div("pb", [text("p", "empty", "no docs/ directory found in this repo")]),
            ]),
          ]),
        );
        return;
      }

      // Group by top-level dir, preserving the server's sort (root group first).
      const groups = new Map<string, DocItem[]>();
      for (const d of docs) {
        const arr = groups.get(d.group) ?? [];
        arr.push(d);
        groups.set(d.group, arr);
      }

      const panels: HTMLElement[] = [text("p", "section-h", `documentation · ${num(docs.length)} files`)];
      for (const [group, items] of groups) {
        panels.push(groupPanel(group, items, onOpen));
      }
      host.replaceChildren(div("stack", panels));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      host.replaceChildren(renderError(message));
    }
  })();
}

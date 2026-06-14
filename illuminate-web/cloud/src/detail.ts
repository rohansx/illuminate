// Repo drill-down slide-over: a real focus-trapped dialog that fetches
// /api/workspace/repo/<id> live and renders the repo's stats, recent episodes,
// and contributors. Honest loading/error states; no fabricated content.

import { clear, div, el, text } from "./dom.ts";
import { cleanPreview, num } from "./format.ts";
import { fetchRepo } from "./api.ts";
import type { RepoDetail } from "./types.ts";

let opener: HTMLElement | null = null;

function teardown(scrim: HTMLElement, drawer: HTMLElement, onKey: (e: KeyboardEvent) => void) {
  document.removeEventListener("keydown", onKey, true);
  scrim.remove();
  drawer.remove();
  if (opener && document.contains(opener)) opener.focus();
  opener = null;
}

function statBox(k: string, v: number): HTMLElement {
  return div("stat", [text("div", "stat-k", k), text("div", "stat-v", num(v))]);
}

function renderBody(d: RepoDetail): HTMLElement {
  const body = div("drawer-body");
  body.append(
    div("drawer-stats", [
      statBox("Episodes", d.stats.episodes),
      statBox("Entities", d.stats.entities),
      statBox("Edges", d.stats.edges),
    ]),
  );

  body.append(text("div", "drawer-section-title", `Sources (${d.stats.sources.length})`));
  if (d.stats.sources.length > 0) {
    for (const s of d.stats.sources.slice(0, 12)) {
      body.append(
        el("div", { class: "ep-row" }, [
          el("span", { class: "ep-src" }, [s.source]),
          text("span", "num", `  ${num(s.count)}`),
        ]),
      );
    }
  } else {
    body.append(text("p", "view-sub", "No sources yet."));
  }

  body.append(text("div", "drawer-section-title", `Recent episodes (${d.episodes.length})`));
  if (d.episodes.length > 0) {
    for (const e of d.episodes.slice(0, 30)) {
      body.append(
        div("ep-row", [
          el("div", { class: "ep-src" }, [e.source]),
          text("div", "ep-prev", cleanPreview(e.preview) || "(no preview)"),
        ]),
      );
    }
  } else {
    body.append(text("p", "view-sub", "This repo's graph has no episodes yet."));
  }

  if (d.contributors.length > 0) {
    body.append(text("div", "drawer-section-title", `Contributors (${d.contributors.length})`));
    for (const c of d.contributors.slice(0, 12)) {
      body.append(
        div("ep-row", [text("span", "num", `${num(c.commits)}`), el("span", {}, [`  ${c.name}`])]),
      );
    }
  }
  return body;
}

/** Open the slide-over for a repo id. */
export function openRepoDetail(id: string): void {
  opener = (document.activeElement as HTMLElement) ?? null;

  const scrim = div("scrim");
  const closeBtn = el("button", { class: "drawer-close", "aria-label": "Close" }, ["esc ✕"]);
  const titleEl = text("h2", "drawer-title", id);
  const headEl = div("drawer-head", [
    div("", [titleEl, text("div", "drawer-path", "loading…")]),
    closeBtn,
  ]);
  const drawer = el("aside", { class: "drawer", role: "dialog", "aria-modal": "true", "aria-label": `repository ${id}` }, [
    headEl,
    div("drawer-body", [div("state", [div("spinner"), text("p", "", "loading repository…")])]),
  ]);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      teardown(scrim, drawer, onKey);
      return;
    }
    if (e.key === "Tab") {
      const f = drawer.querySelectorAll<HTMLElement>(
        'button, [href], [tabindex]:not([tabindex="-1"])',
      );
      if (f.length === 0) return;
      const first = f[0];
      const last = f[f.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  };

  closeBtn.addEventListener("click", () => teardown(scrim, drawer, onKey));
  scrim.addEventListener("click", () => teardown(scrim, drawer, onKey));
  document.addEventListener("keydown", onKey, true);

  document.body.append(scrim, drawer);
  closeBtn.focus();

  fetchRepo(id)
    .then((d) => {
      (headEl.querySelector(".drawer-path") as HTMLElement).textContent = d.path;
      const old = drawer.querySelector(".drawer-body") as HTMLElement;
      const fresh = renderBody(d);
      old.replaceWith(fresh);
    })
    .catch((err) => {
      const body = drawer.querySelector(".drawer-body") as HTMLElement;
      clear(body);
      body.append(
        div("state", [
          text("h2", "", "couldn't load this repo"),
          text("p", "", String(err?.message ?? err)),
        ]),
      );
    });
}

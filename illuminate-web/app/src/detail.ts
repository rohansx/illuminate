// Right-side slide-over detail panel. Two entry points share one overlay:
//   openDetail(id)  — fetches /api/page/<id> and renders the FULL wiki page
//   openEpisode(id) — fetches /api/episode/<id> and renders the raw episode
//                     content (markdown via `marked`), with source + created.
//
// Safety: every field except the markdown body is set via textContent (see
// dom.ts). The body is the ONE place HTML is injected, and only after passing
// through `marked` — an accepted, intentional render path per the task. The
// chip/tag/time helpers never trust raw API strings as markup.
//
// A11y: the sheet is a real modal — on open, focus moves to the close button;
// Tab is trapped inside the sheet while open; Escape (captured before any
// other handler, so e.g. the search input never sees it) closes; and focus is
// restored to the element that opened the sheet.

import { marked } from "marked";

import type { Episode, Page } from "./types.ts";
import { div, el, text } from "./dom.ts";
import { fetchEpisode, fetchPage } from "./api.ts";
import { relativeTime } from "./format.ts";

// Render markdown -> HTML synchronously. `marked.parse` can be async when async
// extensions are configured; we use the default sync pipeline and guard the
// return type so the body is always a string before it touches the DOM.
function markdownToHtml(md: string): string {
  const out = marked.parse(md ?? "", { async: false });
  return typeof out === "string" ? out : String(out);
}

// The API reports the page type as its DIR name (plural: "decisions"), while
// older payloads used the singular. Accept both: normalize, then map.
function singularize(type: string): string {
  return type.endsWith("s") ? type.slice(0, -1) : type;
}

function toneFor(type: string): string {
  switch (singularize(type)) {
    case "decision":
      return "teal";
    case "pattern":
      return "sage";
    case "failure":
      return "rust";
    case "module":
      return "lilac";
    default:
      return "";
  }
}

// ---- singleton overlay scaffold ------------------------------------------
let overlay: HTMLElement | null = null;
let sheet: HTMLElement | null = null;
let closeBtn: HTMLButtonElement | null = null;
let bodyHost: HTMLElement | null = null;
let keysBound = false;
let requestSeq = 0; // guards against out-of-order fetch results
let opener: HTMLElement | null = null; // focus is restored here on close

function isOpen(): boolean {
  return !!overlay && !overlay.hasAttribute("hidden");
}

/** Keep Tab cycling inside the sheet while the modal is open. */
function trapTab(e: KeyboardEvent): void {
  if (!isOpen() || !sheet) return;
  const tabbables = Array.from(
    sheet.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    ),
  ).filter((n) => !n.hasAttribute("disabled"));
  if (tabbables.length === 0) return;
  const first = tabbables[0];
  const last = tabbables[tabbables.length - 1];
  const active = document.activeElement;
  const inside = active instanceof Node && sheet.contains(active);
  if (e.shiftKey) {
    if (!inside || active === first) {
      e.preventDefault();
      last.focus();
    }
  } else if (!inside || active === last) {
    e.preventDefault();
    first.focus();
  }
}

function ensureScaffold(): void {
  if (overlay) return;

  overlay = div("detail-overlay", []);
  overlay.setAttribute("hidden", "");
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) closeDetail();
  });

  sheet = el("aside", { class: "detail-sheet", role: "dialog", "aria-modal": "true" });

  closeBtn = el("button", {
    class: "detail-close",
    type: "button",
    "aria-label": "close",
  }) as HTMLButtonElement;
  closeBtn.textContent = "✕ close";
  closeBtn.addEventListener("click", () => closeDetail());

  bodyHost = div("detail-body", []);

  sheet.append(div("detail-bar", [closeBtn]), bodyHost);
  overlay.append(sheet);
  document.body.append(overlay);

  if (!keysBound) {
    // Capture phase: the modal consumes Escape before any other handler
    // (e.g. the search input's clear-on-Escape) can react to it.
    document.addEventListener(
      "keydown",
      (e) => {
        if (!isOpen()) return;
        if (e.key === "Escape") {
          e.stopPropagation();
          e.preventDefault();
          closeDetail();
        } else if (e.key === "Tab") {
          trapTab(e);
        }
      },
      true,
    );
    keysBound = true;
  }
}

function showState(title: string, sub: string, tone = ""): void {
  if (!bodyHost) return;
  const panel = div(`state-panel ${tone}`.trim(), [
    text("p", "state-title", title),
    text("p", "state-sub", sub),
  ]);
  bodyHost.replaceChildren(panel);
}

function showLoading(what: string): void {
  if (!bodyHost) return;
  bodyHost.replaceChildren(
    div("state-panel", [
      div("state-spinner", []),
      text("p", "state-title", `loading ${what}…`),
    ]),
  );
}

function renderPage(p: Page): void {
  if (!bodyHost) return;
  const tone = toneFor(p.type);

  // header: type/status chip + title — the chip shows the singular form.
  const chips = div("detail-chips", []);
  chips.append(text("span", `chip ${tone}`.trim(), p.type ? singularize(p.type) : "page"));
  if (p.status) chips.append(text("span", "chip ghost", p.status));

  const header = div("detail-head", [chips, text("h2", "detail-title", p.title || p.id)]);

  // meta: created / updated relative times
  const meta = div("detail-meta", []);
  if (p.created) meta.append(text("span", "", `created ${relativeTime(p.created)}`));
  if (p.updated) meta.append(text("span", "", `updated ${relativeTime(p.updated)}`));
  if (meta.childNodes.length > 0) header.append(meta);

  // tags
  const tags = p.tags ?? [];
  if (tags.length > 0) {
    const tagRow = div("detail-tags", tags.map((t) => text("span", "ref", `#${t}`)));
    header.append(tagRow);
  }

  // markdown body — the single intentional HTML-injection point, via marked.
  const article = el("article", { class: "detail-md md" });
  article.innerHTML = markdownToHtml(p.body);

  bodyHost.replaceChildren(header, article);
}

function renderEpisode(ep: Episode): void {
  if (!bodyHost) return;

  const chips = div("detail-chips", []);
  chips.append(text("span", "chip amber", "episode"));

  const header = div("detail-head", [chips, text("h2", "detail-title", ep.source || ep.id)]);

  const meta = div("detail-meta", []);
  if (ep.created) meta.append(text("span", "", `created ${relativeTime(ep.created)}`));
  meta.append(text("span", "", ep.id));
  header.append(meta);

  // Episode content is raw text/markdown — rendered via the same intentional
  // marked pipeline as page bodies.
  const article = el("article", { class: "detail-md md" });
  article.innerHTML = markdownToHtml(ep.content);

  bodyHost.replaceChildren(header, article);
}

export function closeDetail(): void {
  if (!overlay) return;
  requestSeq += 1; // invalidate any in-flight fetch
  overlay.setAttribute("hidden", "");
  document.body.classList.remove("detail-open");
  // Restore focus to whatever opened the sheet, if it is still in the page.
  if (opener && opener.isConnected) opener.focus();
  opener = null;
}

/** Shared open routine: show overlay + loading, run the loader, render. */
function open(loadingWhat: string, load: (seq: number) => Promise<void>): void {
  ensureScaffold();
  if (!overlay) return;

  const active = document.activeElement;
  opener = active instanceof HTMLElement ? active : null;

  const seq = ++requestSeq;
  overlay.removeAttribute("hidden");
  document.body.classList.add("detail-open");
  if (sheet) sheet.scrollTop = 0;
  closeBtn?.focus();
  showLoading(loadingWhat);

  void load(seq);
}

/** Open the slide-over for a page id: loading -> page | not-found | error. */
export function openDetail(id: string): void {
  open("page", async (seq) => {
    try {
      const page = await fetchPage(id);
      if (seq !== requestSeq) return; // superseded by a newer open/close
      renderPage(page);
    } catch (err) {
      if (seq !== requestSeq) return;
      const message = err instanceof Error ? err.message : String(err);
      if (message === "page not found") {
        showState("page not found", `no page with id “${id}”`, "rust");
      } else {
        showState("couldn’t load page", message, "rust");
      }
    }
  });
}

/** Open the slide-over for a graph episode id (the Sources drill-down). */
export function openEpisode(id: string): void {
  open("episode", async (seq) => {
    try {
      const ep = await fetchEpisode(id);
      if (seq !== requestSeq) return; // superseded by a newer open/close
      renderEpisode(ep);
    } catch (err) {
      if (seq !== requestSeq) return;
      const message = err instanceof Error ? err.message : String(err);
      if (message === "episode not found") {
        showState("episode not found", `no episode with id “${id}”`, "rust");
      } else {
        showState("couldn’t load episode", message, "rust");
      }
    }
  });
}

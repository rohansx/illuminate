// Right-side slide-over detail panel. Given a page id it fetches
// /api/page/<id> and renders the FULL page: title, type/status chip, tags,
// created/updated, and the markdown `body` rendered to HTML via `marked`.
//
// Safety: every field except the markdown body is set via textContent (see
// dom.ts). The body is the ONE place HTML is injected, and only after passing
// through `marked` — an accepted, intentional render path per the task. The
// chip/tag/time helpers never trust raw API strings as markup.

import { marked } from "marked";

import type { Page } from "./types.ts";
import { div, el, text } from "./dom.ts";
import { fetchPage } from "./api.ts";
import { relativeTime } from "./format.ts";

// Render markdown -> HTML synchronously. `marked.parse` can be async when async
// extensions are configured; we use the default sync pipeline and guard the
// return type so the body is always a string before it touches the DOM.
function markdownToHtml(md: string): string {
  const out = marked.parse(md ?? "", { async: false });
  return typeof out === "string" ? out : String(out);
}

function toneFor(type: string): string {
  switch (type) {
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
let bodyHost: HTMLElement | null = null;
let escBound = false;
let requestSeq = 0; // guards against out-of-order fetch results

function ensureScaffold(): void {
  if (overlay) return;

  overlay = div("detail-overlay", []);
  overlay.setAttribute("hidden", "");
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) closeDetail();
  });

  sheet = el("aside", { class: "detail-sheet", role: "dialog", "aria-modal": "true" });

  const close = el("button", { class: "detail-close", type: "button", "aria-label": "close" });
  close.textContent = "✕ close";
  close.addEventListener("click", () => closeDetail());

  bodyHost = div("detail-body", []);

  sheet.append(div("detail-bar", [close]), bodyHost);
  overlay.append(sheet);
  document.body.append(overlay);

  if (!escBound) {
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && overlay && !overlay.hasAttribute("hidden")) closeDetail();
    });
    escBound = true;
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

function showLoading(): void {
  if (!bodyHost) return;
  bodyHost.replaceChildren(
    div("state-panel", [
      div("state-spinner", []),
      text("p", "state-title", "loading page…"),
    ]),
  );
}

function renderPage(p: Page): void {
  if (!bodyHost) return;
  const tone = toneFor(p.type);

  // header: type/status chip + title
  const chips = div("detail-chips", []);
  chips.append(text("span", `chip ${tone}`.trim(), p.type || "page"));
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

export function closeDetail(): void {
  if (!overlay) return;
  requestSeq += 1; // invalidate any in-flight fetch
  overlay.setAttribute("hidden", "");
  document.body.classList.remove("detail-open");
}

/** Open the slide-over for a page id: loading -> page | not-found | error. */
export function openDetail(id: string): void {
  ensureScaffold();
  if (!overlay) return;

  const seq = ++requestSeq;
  overlay.removeAttribute("hidden");
  document.body.classList.add("detail-open");
  if (sheet) sheet.scrollTop = 0;
  showLoading();

  void (async () => {
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
  })();
}

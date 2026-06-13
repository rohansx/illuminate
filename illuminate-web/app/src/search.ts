// Topbar search. On Enter it fetches /api/search?q=<text> and renders a result
// list (title + type chip + snippet) into a panel below the input; each result
// is clickable into the detail slide-over. Empty query clears the panel; a
// no-results query shows an honest empty state. No fabricated hits.

import type { SearchResult } from "./types.ts";
import { div, el, text } from "./dom.ts";
import { cleanSnippet } from "./format.ts";
import { fetchSearch } from "./api.ts";
import type { OpenPage } from "./render.ts";

// The API reports the page type as its DIR name (plural: "decisions"); older
// payloads used the singular. Accept both: normalize, then map (same contract
// as detail.ts).
function singularize(type: string): string {
  return type.endsWith("s") ? type.slice(0, -1) : type;
}

function toneFor(type: string): string {
  switch (singularize(type)) {
    case "decision":
      return "dec";
    case "pattern":
      return "pat";
    case "failure":
      return "fail";
    default:
      return "";
  }
}

function resultRow(r: SearchResult, onOpen: OpenPage): HTMLElement {
  const top = div("sr-top", [
    text("span", "sr-title", r.title || r.id),
    text("span", `ctag ${toneFor(r.type)}`.trim(), r.type ? singularize(r.type) : "page"),
  ]);
  const children: HTMLElement[] = [top];
  // Snippets are raw markdown — strip the syntax noise before display.
  const snippet = cleanSnippet(r.snippet);
  if (snippet) children.push(text("p", "sr-snippet", snippet));

  const row = el("button", { class: "sr-row", type: "button" }, children);
  row.addEventListener("click", () => onOpen(r.id));
  return row;
}

/** Build the topbar search control + a results panel. Returns both nodes. */
export function createSearch(onOpen: OpenPage): { control: HTMLElement; results: HTMLElement } {
  const results = div("search-results", []);
  results.setAttribute("hidden", "");

  const input = el("input", {
    class: "search-input",
    type: "search",
    placeholder: "search the graph…  ↵",
    "aria-label": "search knowledge",
  }) as HTMLInputElement;

  let seq = 0;

  function clear(): void {
    seq += 1;
    input.value = "";
    results.replaceChildren();
    results.setAttribute("hidden", "");
  }

  /** Head row for the results panel — always carries its own ✕ dismiss, so
   *  the panel can be closed without Esc or manually emptying the input. */
  function headRow(children: HTMLElement[]): HTMLElement {
    const dismiss = el("button", {
      class: "search-clear",
      type: "button",
      "aria-label": "clear search results",
    });
    dismiss.textContent = "✕";
    dismiss.addEventListener("click", () => clear());
    return div("search-head", [...children, dismiss]);
  }

  function showState(title: string): void {
    results.replaceChildren(
      headRow([text("span", "label", "search"), text("span", "title", title)]),
    );
    results.removeAttribute("hidden");
  }

  async function run(): Promise<void> {
    const q = input.value.trim();
    if (!q) {
      clear();
      return;
    }
    const mine = ++seq;
    showState("searching…");
    try {
      const hits = await fetchSearch(q);
      if (mine !== seq) return;
      const head = headRow([
        text("span", "label", "search"),
        text("span", "title", `“${q}”`),
        text("span", "sub", `${hits.length} result${hits.length === 1 ? "" : "s"}`),
      ]);
      if (hits.length === 0) {
        results.replaceChildren(head, text("p", "empty", "no matches"));
      } else {
        results.replaceChildren(head, div("sr-list", hits.map((h) => resultRow(h, onOpen))));
      }
      results.removeAttribute("hidden");
    } catch (err) {
      if (mine !== seq) return;
      const message = err instanceof Error ? err.message : String(err);
      results.replaceChildren(
        headRow([text("span", "label", "search"), text("span", "title", "search failed")]),
        text("p", "state-detail", message),
      );
      results.removeAttribute("hidden");
    }
  }

  // NOTE: while the detail slide-over is open, detail.ts consumes Escape in a
  // document-level capture handler (stopPropagation), so a single Esc closes
  // ONLY the detail — it can never also wipe the query/results below.
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      void run();
    } else if (e.key === "Escape") {
      clear();
    }
  });
  input.addEventListener("input", () => {
    if (input.value.trim() === "") clear();
  });

  const control = div("search-box", [input]);
  return { control, results };
}

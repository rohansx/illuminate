// Fixed left rail (reusing the dark .rail / .vnav aesthetic) plus the in-app
// view-state switcher. No router library — switching a view simply toggles the
// active rail item and asks the host to render that view. Overview is default.

import { el, text } from "./dom.ts";

export type ViewId = "overview" | "knowledge" | "sources" | "tokens";

interface NavItem {
  id: ViewId;
  short: string; // vertical rail label (keep short for the 60px rail)
}

const ITEMS: NavItem[] = [
  { id: "overview", short: "over" },
  { id: "knowledge", short: "know" },
  { id: "sources", short: "src" },
  { id: "tokens", short: "tok" },
];

/**
 * Build the fixed left rail. `onSelect` is called with the chosen view id; the
 * caller drives the actual render and then calls the returned `setActive` to
 * sync the highlight (so external/default selection stays consistent).
 */
export function createRail(onSelect: (id: ViewId) => void): {
  rail: HTMLElement;
  setActive: (id: ViewId) => void;
} {
  const mark = el("div", { class: "mark" });

  const vnav = el("nav", { class: "vnav", "aria-label": "views" });
  const links = new Map<ViewId, HTMLElement>();

  for (const item of ITEMS) {
    const a = el("a", { href: "#", role: "button", "aria-label": item.id });
    a.textContent = item.short;
    a.addEventListener("click", (e) => {
      e.preventDefault();
      onSelect(item.id);
    });
    links.set(item.id, a);
    vnav.append(a);
  }

  const footer = el("div", { class: "footer" });
  footer.append(text("b", "", "illuminate"));

  const rail = el("aside", { class: "rail" }, [mark, el("div", { class: "sep" }), vnav, footer]);

  function setActive(id: ViewId): void {
    for (const [key, a] of links) a.classList.toggle("active", key === id);
  }

  return { rail, setActive };
}

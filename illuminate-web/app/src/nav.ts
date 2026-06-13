// Navigation: the fixed left rail (desktop) plus a compact horizontal tab
// strip (mobile, <=720px — where illuminate-v4.css hides the rail entirely).
// Both are built here and kept in sync by the shared `setActive`. No router
// library — switching a view simply toggles the active item and asks the host
// to render that view. Overview is default.

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
 * Build the fixed left rail AND the mobile tab strip. `onSelect` is called
 * with the chosen view id; the caller drives the actual render and then calls
 * the returned `setActive` to sync the highlight on both navs (so external /
 * default selection stays consistent).
 */
export function createNav(onSelect: (id: ViewId) => void): {
  rail: HTMLElement;
  mobileNav: HTMLElement;
  setActive: (id: ViewId) => void;
} {
  const mark = el("div", { class: "mark" });

  const vnav = el("nav", { class: "vnav", "aria-label": "views" });
  const railLinks = new Map<ViewId, HTMLElement>();

  for (const item of ITEMS) {
    // `title` gives sighted users the full word behind the rail abbreviation.
    const a = el("a", { href: "#", role: "button", "aria-label": item.id, title: item.id });
    a.textContent = item.short;
    a.addEventListener("click", (e) => {
      e.preventDefault();
      onSelect(item.id);
    });
    railLinks.set(item.id, a);
    vnav.append(a);
  }

  const footer = el("div", { class: "footer" });
  footer.append(text("b", "", "illuminate"));

  const rail = el("aside", { class: "rail" }, [mark, el("div", { class: "sep" }), vnav, footer]);

  // Mobile: a horizontal tab strip under the topbar with FULL labels —
  // display is toggled purely in CSS (shown only at <=720px).
  const mobileNav = el("nav", { class: "mnav", "aria-label": "views" });
  const mobileButtons = new Map<ViewId, HTMLElement>();
  for (const item of ITEMS) {
    const b = el("button", { type: "button" });
    b.textContent = item.id;
    b.addEventListener("click", () => onSelect(item.id));
    mobileButtons.set(item.id, b);
    mobileNav.append(b);
  }

  function setActive(id: ViewId): void {
    for (const [key, a] of railLinks) a.classList.toggle("active", key === id);
    for (const [key, b] of mobileButtons) b.classList.toggle("active", key === id);
  }

  return { rail, mobileNav, setActive };
}

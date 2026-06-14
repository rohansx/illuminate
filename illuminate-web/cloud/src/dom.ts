// Tiny DOM builder. Every text node goes through textContent, so repo names,
// sources, and previews from the API can never inject markup — no innerHTML of
// raw API strings anywhere in the app.

type Attrs = Record<string, string>;
type Child = Node | string;

export function el(tag: string, attrs: Attrs = {}, children: Child[] = []): HTMLElement {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    node.setAttribute(k, v);
  }
  for (const c of children) {
    node.append(typeof c === "string" ? document.createTextNode(c) : c);
  }
  return node;
}

/** Convenience: a <div class="..."> with text or element children. */
export function div(className: string, children: Child[] = []): HTMLElement {
  return el("div", { class: className }, children);
}

/** Convenience: a text element whose content is set safely via textContent. */
export function text(tag: string, className: string, value: string): HTMLElement {
  const node = el(tag, className ? { class: className } : {});
  node.textContent = value;
  return node;
}

/** Remove all children of a node. */
export function clear(node: HTMLElement): void {
  while (node.firstChild) node.removeChild(node.firstChild);
}

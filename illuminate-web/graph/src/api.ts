import type { GraphData, Layer } from "./types";

/** GET /api/layout?layer=&max_nodes= — illuminate's laid-out graph. Throws on
 * any non-OK response or shape mismatch so the UI shows an honest error. */
export async function fetchLayout(layer: Layer, maxNodes = 6000): Promise<GraphData> {
  const resp = await fetch(`/api/layout?layer=${layer}&max_nodes=${maxNodes}`, {
    headers: { Accept: "application/json" },
  });
  if (!resp.ok) throw new Error(`/api/layout returned ${resp.status}`);
  const data = (await resp.json()) as GraphData;
  if (!data || !Array.isArray(data.nodes) || !Array.isArray(data.edges)) {
    throw new Error("unexpected response shape from /api/layout");
  }
  return data;
}

// Edge renderer: one lineSegments BufferGeometry for every edge, additive
// blending so dense regions glow. Color is by edge type; intra-cluster edges
// are brighter than cross-cluster.
//
// Technique adapted from codebase-memory-mcp's graph-ui/src/components/EdgeLines.tsx
// (MIT, © 2025 DeusData).

import { useMemo } from "react";
import * as THREE from "three";
import type { GraphEdge, GraphNode } from "./types";

interface Props {
  nodes: GraphNode[];
  edges: GraphEdge[];
  highlightedIds: Set<number> | null;
}

// illuminate edge types (capitalized): code containment/calls/imports +
// decision-graph relations.
const EDGE_TYPE_COLORS: Record<string, string> = {
  CONTAINS: "#3d6a55",
  CALLS: "#1DA27E",
  IMPORTS: "#3b82f6",
  INHERITS: "#f97316",
  REFERENCES: "#a855f7",
  SUPERSEDES: "#b6573a",
  CONTRADICTS: "#9c3d20",
  RELATED: "#c89a3a",
};
const DEFAULT_EDGE_COLOR = "#1C8585";

function clusterKey(fp?: string): string {
  if (!fp) return "";
  return fp.split("/").slice(0, 2).join("/");
}

export function EdgeLines({ nodes, edges, highlightedIds }: Props) {
  const geometry = useMemo(() => {
    const idMap = new Map<number, number>();
    for (let i = 0; i < nodes.length; i++) idMap.set(nodes[i].id, i);

    const hasHighlight = highlightedIds && highlightedIds.size > 0;
    const positions = new Float32Array(edges.length * 6);
    const colors = new Float32Array(edges.length * 6);
    let valid = 0;

    for (const edge of edges) {
      const si = idMap.get(edge.source);
      const ti = idMap.get(edge.target);
      if (si === undefined || ti === undefined) continue;
      const s = nodes[si];
      const t = nodes[ti];

      const sHL = !hasHighlight || highlightedIds!.has(s.id);
      const tHL = !hasHighlight || highlightedIds!.has(t.id);
      if (hasHighlight && !sHL && !tHL) continue;

      const same = clusterKey(s.file_path) === clusterKey(t.file_path);
      let intensity = same ? 0.28 : 0.08;
      if (hasHighlight) intensity = sHL && tHL ? 0.6 : 0.04;

      const off = valid * 6;
      positions[off] = s.x;
      positions[off + 1] = s.y;
      positions[off + 2] = s.z;
      positions[off + 3] = t.x;
      positions[off + 4] = t.y;
      positions[off + 5] = t.z;

      const c = new THREE.Color(EDGE_TYPE_COLORS[edge.type.toUpperCase()] ?? DEFAULT_EDGE_COLOR);
      for (let k = 0; k < 2; k++) {
        colors[off + k * 3] = c.r * intensity;
        colors[off + k * 3 + 1] = c.g * intensity;
        colors[off + k * 3 + 2] = c.b * intensity;
      }
      valid++;
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(positions.slice(0, valid * 6), 3));
    geo.setAttribute("color", new THREE.BufferAttribute(colors.slice(0, valid * 6), 3));
    return geo;
  }, [nodes, edges, highlightedIds]);

  return (
    <lineSegments geometry={geometry}>
      <lineBasicMaterial
        vertexColors
        transparent
        opacity={1}
        blending={THREE.AdditiveBlending}
        depthWrite={false}
        toneMapped={false}
      />
    </lineSegments>
  );
}

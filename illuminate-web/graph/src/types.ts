// Matches the GraphData JSON emitted by illuminate's /api/layout
// (crates/illuminate-layout) — itself a port of codebase-memory-mcp's
// layout3d.c output contract (MIT).

export interface GraphNode {
  id: number;
  x: number;
  y: number;
  z: number;
  label: string;
  name: string;
  file_path?: string;
  size: number;
  color: string;
}

export interface GraphEdge {
  source: number;
  target: number;
  type: string;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
  total_nodes: number;
}

export type Layer = "code" | "decisions" | "both";

// Illuminate graph — a 3D galaxy of illuminate's own code + decision graphs,
// rendered from /api/layout. Strictly live data: a fetch failure shows an
// honest error, an empty graph an honest empty state. Renderer technique
// adapted from codebase-memory-mcp graph-ui (MIT, © 2025 DeusData); this app,
// its UI, and the illuminate wiring are original.

import { useEffect, useMemo, useRef, useState } from "react";
import { Canvas } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import { EffectComposer, Bloom } from "@react-three/postprocessing";
import { NodeCloud } from "./NodeCloud";
import { EdgeLines } from "./EdgeLines";
import { fetchLayout } from "./api";
import type { GraphData, GraphNode, Layer } from "./types";

const LAYERS: { id: Layer; label: string }[] = [
  { id: "both", label: "All" },
  { id: "code", label: "Code" },
  { id: "decisions", label: "Decisions" },
];

export default function App() {
  const [layer, setLayer] = useState<Layer>("both");
  const [data, setData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hovered, setHovered] = useState<GraphNode | null>(null);
  const [selected, setSelected] = useState<GraphNode | null>(null);
  const [pointer, setPointer] = useState({ x: 0, y: 0 });

  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    setSelected(null);
    fetchLayout(layer)
      .then((d) => {
        if (alive) {
          setData(d);
          setLoading(false);
        }
      })
      .catch((e) => {
        if (alive) {
          setError(e instanceof Error ? e.message : String(e));
          setLoading(false);
        }
      });
    return () => {
      alive = false;
    };
  }, [layer]);

  // Neighbors of the selected node — highlight it + its edges.
  const highlightedIds = useMemo(() => {
    if (!selected || !data) return null;
    const set = new Set<number>([selected.id]);
    for (const e of data.edges) {
      if (e.source === selected.id) set.add(e.target);
      else if (e.target === selected.id) set.add(e.source);
    }
    return set;
  }, [selected, data]);

  const nodes = data?.nodes ?? [];
  const edges = data?.edges ?? [];

  return (
    <div
      className="wrap"
      onMouseMove={(e) => setPointer({ x: e.clientX, y: e.clientY })}
    >
      <Canvas
        camera={{ position: [0, 0, 1500], fov: 60, near: 1, far: 20000 }}
        dpr={[1, 2]}
        gl={{ antialias: true }}
        onPointerMissed={() => setSelected(null)}
        style={{ background: "#0a0a0f" }}
      >
        {nodes.length > 0 && (
          <>
            <EdgeLines nodes={nodes} edges={edges} highlightedIds={highlightedIds} />
            <NodeCloud
              nodes={nodes}
              highlightedIds={highlightedIds}
              onHover={setHovered}
              onClick={setSelected}
            />
            <EffectComposer>
              <Bloom intensity={1.1} luminanceThreshold={0.55} luminanceSmoothing={0.3} mipmapBlur />
            </EffectComposer>
          </>
        )}
        <OrbitControls
          enableDamping
          dampingFactor={0.08}
          autoRotate={!selected}
          autoRotateSpeed={0.35}
          maxDistance={9000}
        />
      </Canvas>

      <Overlay
        layer={layer}
        setLayer={setLayer}
        data={data}
        loading={loading}
        error={error}
      />

      {hovered && (
        <div className="tooltip" style={{ left: pointer.x + 14, top: pointer.y + 14 }}>
          <span className="tt-label">{hovered.label}</span>
          <span className="tt-name">{hovered.name}</span>
          {hovered.file_path && <span className="tt-path">{hovered.file_path}</span>}
        </div>
      )}

      {selected && <DetailPanel node={selected} data={data} onClose={() => setSelected(null)} />}
    </div>
  );
}

function Overlay({
  layer,
  setLayer,
  data,
  loading,
  error,
}: {
  layer: Layer;
  setLayer: (l: Layer) => void;
  data: GraphData | null;
  loading: boolean;
  error: string | null;
}) {
  return (
    <>
      <header className="hud">
        <div>
          <div className="eyebrow">illuminate · knowledge graph</div>
          <h1 className="title">
            Your codebase, as a <em>galaxy</em>.
          </h1>
        </div>
        <div className="layers" role="tablist" aria-label="graph layer">
          {LAYERS.map((l) => (
            <button
              key={l.id}
              className={l.id === layer ? "on" : ""}
              aria-pressed={l.id === layer}
              onClick={() => setLayer(l.id)}
            >
              {l.label}
            </button>
          ))}
        </div>
      </header>

      <footer className="stats">
        {loading && <span className="dim">loading layout…</span>}
        {error && <span className="err">graph API error: {error}</span>}
        {!loading && !error && data && (
          <>
            <b>{data.nodes.length.toLocaleString()}</b> nodes ·{" "}
            <b>{data.edges.length.toLocaleString()}</b> edges
            {data.total_nodes > data.nodes.length && (
              <span className="dim"> · of {data.total_nodes.toLocaleString()} total</span>
            )}
            {data.nodes.length === 0 && (
              <span className="dim">
                {" "}
                — empty. run <code>illuminate index</code> + <code>illuminate bootstrap</code>.
              </span>
            )}
          </>
        )}
      </footer>
    </>
  );
}

function DetailPanel({
  node,
  data,
  onClose,
}: {
  node: GraphNode;
  data: GraphData | null;
  onClose: () => void;
}) {
  const degree = useRef(0);
  degree.current = data ? data.edges.filter((e) => e.source === node.id || e.target === node.id).length : 0;
  return (
    <aside className="detail">
      <button className="close" onClick={onClose} aria-label="Close">
        ✕
      </button>
      <div className="d-label" style={{ color: node.color }}>
        {node.label}
      </div>
      <div className="d-name">{node.name}</div>
      {node.file_path && <div className="d-path">{node.file_path}</div>}
      <dl className="d-meta">
        <div>
          <dt>connections</dt>
          <dd>{degree.current}</dd>
        </div>
        <div>
          <dt>id</dt>
          <dd>{node.id}</dd>
        </div>
      </dl>
    </aside>
  );
}

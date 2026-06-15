//! Deterministic 3D force-directed graph layout for illuminate's graph
//! visualization.
//!
//! A faithful Rust port of codebase-memory-mcp's `src/ui/layout3d.c` (MIT,
//! © 2025 DeusData). Strategy, unchanged from the original:
//!   1. seed each node on a ring by directory-cluster key (clean structure),
//!   2. assign z from call depth (entry points high, callees below),
//!   3. run 40 iterations of anchored ForceAtlas2 — Barnes-Hut repulsion +
//!      edge attraction + a spring back to the seed position — so clusters stay
//!      separated while overlaps untangle.
//!
//! Pure and deterministic: no clock, no RNG except a seeded LCG keyed on each
//! node's qualified name, so the same graph always lays out the same way. The
//! browser never lays out — it just renders the `{x,y,z,size,color}` this emits.
//!
//! The layout is graph-agnostic (it works on any node/edge set), so it serves
//! both illuminate's code graph and its decision graph.

use serde::Serialize;

const BH_THETA: f32 = 1.2;
const LOCAL_REPULSION: f32 = 8.0;
const LOCAL_ATTRACTION: f32 = 1.0;
const LOCAL_ANCHOR_K: f32 = 0.25;
const LOCAL_ITERATIONS: usize = 40;
const Z_DEPTH_SPACING: f32 = 50.0;

/// Default node cap (matches cbm's `DEFAULT_MAX_NODES`). Callers should truncate
/// to this before calling [`compute`] and pass the true total.
pub const DEFAULT_MAX_NODES: usize = 50_000;

/// An input node. `label` drives size + entry-point detection; `file_path`
/// drives the directory-cluster ring; `qualified_name` seeds deterministic
/// jitter.
#[derive(Debug, Clone)]
pub struct Node {
    pub label: String,
    pub name: String,
    pub file_path: Option<String>,
    pub qualified_name: String,
}

/// An input edge, referencing node positions by index into the `nodes` slice
/// passed to [`compute`]. Out-of-range indices are ignored.
#[derive(Debug, Clone)]
pub struct Edge {
    pub source: usize,
    pub target: usize,
    pub edge_type: String,
}

/// A laid-out node — matches the `GraphNode` contract the React renderer
/// expects (`graph-ui/src/lib/types.ts`).
#[derive(Debug, Clone, Serialize)]
pub struct OutNode {
    pub id: usize,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub label: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    pub size: f32,
    pub color: String,
}

/// A laid-out edge (serializes its `edge_type` as `type`, matching the
/// frontend `GraphEdge`).
#[derive(Debug, Clone, Serialize)]
pub struct OutEdge {
    pub source: usize,
    pub target: usize,
    #[serde(rename = "type")]
    pub edge_type: String,
}

/// The full layout payload — the `GraphData` the `/api/layout` route serves.
#[derive(Debug, Clone, Serialize)]
pub struct GraphData {
    pub nodes: Vec<OutNode>,
    pub edges: Vec<OutEdge>,
    pub total_nodes: usize,
}

// ── color + size ───────────────────────────────────────────────────────────

/// Stellar spectral-type color by node degree (Hertzsprung-Russell: low-degree
/// leaves are red dwarfs, mega-hubs are blue giants). Returned as `#rrggbb`.
fn stellar_color(degree: usize) -> String {
    let rgb: u32 = match degree {
        0..=1 => 0xff6050,
        2..=3 => 0xff8855,
        4..=5 => 0xffa060,
        6..=8 => 0xffc070,
        9..=12 => 0xffe080,
        13..=18 => 0xfff0c0,
        19..=25 => 0xfff8e8,
        26..=35 => 0xe8e8ff,
        36..=50 => 0xc0d0ff,
        _ => 0x80a0ff,
    };
    format!("#{rgb:06x}")
}

/// Base node size by label (containers are bigger than leaves). Covers both
/// code-graph labels (File/Class/Function/…) and illuminate's decision-graph
/// labels (Decision/Pattern/Failure/Module/Entity).
fn size_for_label(label: &str) -> f32 {
    match label {
        "Project" => 20.0,
        "Package" | "Module" => 15.0,
        "Folder" => 12.0,
        "File" => 8.0,
        "Decision" => 9.0,
        "Pattern" | "Failure" => 7.0,
        "Class" | "Interface" | "Struct" | "Enum" | "Trait" | "Entity" => 6.0,
        _ => 4.0, // Function / Method / unknown
    }
}

/// Is this label an entry point (placed at call depth 0)?
fn is_entry_label(label: &str) -> bool {
    matches!(label, "Route" | "File" | "Module" | "Package")
}

// ── deterministic hashing / jitter ──────────────────────────────────────────

fn fnv1a(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

/// Seeded LCG, range [-0.5, 0.5). Deterministic given the seed.
fn rand_float(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
    ((*seed >> 16) & 0x7FFF) as f32 / 32768.0 - 0.5
}

// ── call depth (BFS) ────────────────────────────────────────────────────────

fn compute_call_depth(n: usize, es: &[usize], ed: &[usize], labels: &[&str]) -> Vec<i32> {
    let mut depth = vec![-1i32; n];
    let mut q: Vec<usize> = Vec::new();

    for (i, lbl) in labels.iter().enumerate().take(n) {
        if is_entry_label(lbl) {
            depth[i] = 0;
            q.push(i);
        }
    }
    // No labelled entry points → use in-degree-0 nodes as roots.
    if q.is_empty() {
        let mut indeg = vec![0u32; n];
        for &t in ed {
            if t < n {
                indeg[t] += 1;
            }
        }
        for (i, &d) in indeg.iter().enumerate() {
            if d == 0 {
                depth[i] = 0;
                q.push(i);
            }
        }
    }

    let mut head = 0;
    while head < q.len() {
        let c = q[head];
        head += 1;
        let cd = depth[c];
        for (e, &s) in es.iter().enumerate() {
            if s == c {
                let t = ed[e];
                if t < n && depth[t] == -1 {
                    depth[t] = cd + 1;
                    q.push(t);
                }
            }
        }
    }
    for d in depth.iter_mut() {
        if *d == -1 {
            *d = 0;
        }
    }
    depth
}

// ── Barnes-Hut octree (arena-backed) ────────────────────────────────────────

#[derive(Clone)]
struct Oct {
    cx: f32,
    cy: f32,
    cz: f32,
    total_mass: f32,
    half: f32,
    ox: f32,
    oy: f32,
    oz: f32,
    body: i32, // node index of a single contained body, else -1
    body_mass: f32,
    children: [i32; 8], // arena indices, -1 = none
}

impl Oct {
    fn new(ox: f32, oy: f32, oz: f32, half: f32) -> Self {
        Oct {
            cx: 0.0,
            cy: 0.0,
            cz: 0.0,
            total_mass: 0.0,
            half,
            ox,
            oy,
            oz,
            body: -1,
            body_mass: 0.0,
            children: [-1; 8],
        }
    }
}

fn octant(o: &Oct, x: f32, y: f32, z: f32) -> usize {
    (if x >= o.ox { 1 } else { 0 })
        | (if y >= o.oy { 2 } else { 0 })
        | (if z >= o.oz { 4 } else { 0 })
}

fn child_center(o: &Oct, oi: usize) -> (f32, f32, f32) {
    let q = o.half * 0.5;
    (
        o.ox + if oi & 1 != 0 { q } else { -q },
        o.oy + if oi & 2 != 0 { q } else { -q },
        o.oz + if oi & 4 != 0 { q } else { -q },
    )
}

fn oct_insert(arena: &mut Vec<Oct>, ni: usize, idx: usize, x: f32, y: f32, z: f32, mass: f32) {
    // Empty leaf → place the body here.
    if arena[ni].total_mass == 0.0 && arena[ni].body == -1 {
        let o = &mut arena[ni];
        o.body = idx as i32;
        o.body_mass = mass;
        o.cx = x;
        o.cy = y;
        o.cz = z;
        o.total_mass = mass;
        return;
    }
    // Occupied leaf → push its existing body down first.
    if arena[ni].body >= 0 {
        let (oi_idx, ox, oy, oz, om) = {
            let o = &arena[ni];
            (o.body as usize, o.cx, o.cy, o.cz, o.body_mass)
        };
        arena[ni].body = -1;
        let oo = octant(&arena[ni], ox, oy, oz);
        if arena[ni].children[oo] < 0 {
            let (a, b, c) = child_center(&arena[ni], oo);
            let half = arena[ni].half * 0.5;
            arena.push(Oct::new(a, b, c, half));
            let ci = arena.len() - 1;
            arena[ni].children[oo] = ci as i32;
        }
        let ci = arena[ni].children[oo] as usize;
        oct_insert(arena, ci, oi_idx, ox, oy, oz, om);
    }
    // Update center of mass.
    {
        let o = &mut arena[ni];
        let nm = o.total_mass + mass;
        o.cx = (o.cx * o.total_mass + x * mass) / nm;
        o.cy = (o.cy * o.total_mass + y * mass) / nm;
        o.cz = (o.cz * o.total_mass + z * mass) / nm;
        o.total_mass = nm;
    }
    let oo = octant(&arena[ni], x, y, z);
    if arena[ni].children[oo] < 0 {
        let (a, b, c) = child_center(&arena[ni], oo);
        let half = arena[ni].half * 0.5;
        arena.push(Oct::new(a, b, c, half));
        let ci = arena.len() - 1;
        arena[ni].children[oo] = ci as i32;
    }
    let ci = arena[ni].children[oo] as usize;
    oct_insert(arena, ci, idx, x, y, z, mass);
}

#[allow(clippy::too_many_arguments)]
fn oct_repulse(
    arena: &[Oct],
    ni: i32,
    px: f32,
    py: f32,
    pz: f32,
    mm: f32,
    si: usize,
    f: &mut (f32, f32, f32),
) {
    if ni < 0 {
        return;
    }
    let o = &arena[ni as usize];
    if o.total_mass == 0.0 || o.body == si as i32 {
        return;
    }
    let dx = px - o.cx;
    let dy = py - o.cy;
    let dz = pz - o.cz;
    let mut d = (dx * dx + dy * dy + dz * dz).sqrt();
    if o.body >= 0 || (o.half * 2.0 / (d + 0.001)) < BH_THETA {
        if d < 0.01 {
            d = 0.01;
        }
        let force = LOCAL_REPULSION * mm * o.total_mass / d;
        f.0 += force * dx / d;
        f.1 += force * dy / d;
        f.2 += force * dz / d;
        return;
    }
    for &c in &o.children {
        oct_repulse(arena, c, px, py, pz, mm, si, f);
    }
}

// ── one body during optimization ────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Body {
    x: f32,
    y: f32,
    z: f32,
    ax: f32,
    ay: f32,
    az: f32,
    fx: f32,
    fy: f32,
    fz: f32,
    mass: f32,
}

fn local_optimize(b: &mut [Body], es: &[usize], ed: &[usize]) {
    let n = b.len();
    for _ in 0..LOCAL_ITERATIONS {
        for body in b.iter_mut() {
            body.fx = 0.0;
            body.fy = 0.0;
            body.fz = 0.0;
        }

        // Bounding box.
        let (mut mnx, mut mny, mut mnz) = (1e9f32, 1e9f32, 1e9f32);
        let (mut mxx, mut mxy, mut mxz) = (-1e9f32, -1e9f32, -1e9f32);
        for body in b.iter() {
            mnx = mnx.min(body.x);
            mny = mny.min(body.y);
            mnz = mnz.min(body.z);
            mxx = mxx.max(body.x);
            mxy = mxy.max(body.y);
            mxz = mxz.max(body.z);
        }
        let half = (mxx - mnx).max(mxy - mny).max(mxz - mnz) * 0.5 + 1.0;

        // Repulsion via Barnes-Hut.
        let mut arena: Vec<Oct> = Vec::with_capacity(n * 2);
        arena.push(Oct::new(
            (mnx + mxx) * 0.5,
            (mny + mxy) * 0.5,
            (mnz + mxz) * 0.5,
            half,
        ));
        for (i, body) in b.iter().enumerate() {
            oct_insert(&mut arena, 0, i, body.x, body.y, body.z, body.mass);
        }
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            let (px, py, pz, mm) = (b[i].x, b[i].y, b[i].z, b[i].mass);
            let mut f = (0.0f32, 0.0f32, 0.0f32);
            oct_repulse(&arena, 0, px, py, pz, mm, i, &mut f);
            b[i].fx += f.0;
            b[i].fy += f.1;
            b[i].fz += f.2;
        }

        // Attraction along edges.
        for (e, &s) in es.iter().enumerate() {
            let t = ed[e];
            if s >= n || t >= n {
                continue;
            }
            let dx = b[t].x - b[s].x;
            let dy = b[t].y - b[s].y;
            let dz = b[t].z - b[s].z;
            b[s].fx += dx * LOCAL_ATTRACTION;
            b[s].fy += dy * LOCAL_ATTRACTION;
            b[s].fz += dz * LOCAL_ATTRACTION;
            b[t].fx -= dx * LOCAL_ATTRACTION;
            b[t].fy -= dy * LOCAL_ATTRACTION;
            b[t].fz -= dz * LOCAL_ATTRACTION;
        }

        // Anchor spring back to the seed (ring) position.
        for body in b.iter_mut() {
            body.fx += (body.ax - body.x) * LOCAL_ANCHOR_K * body.mass;
            body.fy += (body.ay - body.y) * LOCAL_ANCHOR_K * body.mass;
            body.fz += (body.az - body.z) * LOCAL_ANCHOR_K * body.mass;
        }

        // Apply with capped displacement.
        for body in b.iter_mut() {
            let fm = (body.fx * body.fx + body.fy * body.fy + body.fz * body.fz).sqrt();
            let mut speed = 1.0f32;
            if speed * fm > 8.0 {
                speed = 8.0 / (fm + 0.001);
            }
            body.x += body.fx * speed;
            body.y += body.fy * speed;
            body.z += body.fz * speed;
        }
    }
}

/// First-3-directory-components cluster key for a file path (the ring grouping).
fn cluster_key(file_path: &str) -> String {
    let mut out = String::new();
    let mut slashes = 0;
    for ch in file_path.chars() {
        if out.len() >= 255 {
            break;
        }
        if ch == '/' {
            slashes += 1;
            if slashes >= 3 {
                break;
            }
        }
        out.push(ch);
    }
    out
}

/// Lay out `nodes` (already capped to at most `total_nodes`) + `edges` into 3D
/// positions, colors, and sizes. `total_nodes` is the untruncated count so the
/// UI can show "showing N of total".
pub fn compute(nodes: &[Node], edges: &[Edge], total_nodes: usize) -> GraphData {
    let n = nodes.len();
    if n == 0 {
        return GraphData {
            nodes: Vec::new(),
            edges: Vec::new(),
            total_nodes,
        };
    }

    // Resolve valid edges to index pairs + accumulate degree.
    let mut es: Vec<usize> = Vec::with_capacity(edges.len());
    let mut ed: Vec<usize> = Vec::with_capacity(edges.len());
    let mut etype: Vec<String> = Vec::with_capacity(edges.len());
    let mut deg = vec![0usize; n];
    for e in edges {
        if e.source < n && e.target < n {
            deg[e.source] += 1;
            deg[e.target] += 1;
            es.push(e.source);
            ed.push(e.target);
            etype.push(e.edge_type.clone());
        }
    }

    let labels: Vec<&str> = nodes.iter().map(|nd| nd.label.as_str()).collect();
    let cdepth = compute_call_depth(n, &es, &ed, &labels);

    // Seed positions: ring by cluster key + z from call depth + jittered.
    let mut bodies: Vec<Body> = Vec::with_capacity(n);
    for (i, nd) in nodes.iter().enumerate() {
        let fp = nd.file_path.as_deref().unwrap_or("");
        let h = fnv1a(&cluster_key(fp));
        let angle = (h & 0xFFFF) as f32 / 65535.0 * std::f32::consts::TAU;
        let r = 500.0 + ((h >> 16) & 0xFF) as f32 / 255.0 * 250.0;
        let mut seed = fnv1a(&nd.qualified_name);
        let jitter = 40.0;
        let px = r * angle.cos() + rand_float(&mut seed) * jitter;
        let py = r * angle.sin() + rand_float(&mut seed) * jitter;
        let pz = -(cdepth[i] as f32) * Z_DEPTH_SPACING;
        bodies.push(Body {
            x: px,
            y: py,
            z: pz,
            ax: px,
            ay: py,
            az: pz,
            fx: 0.0,
            fy: 0.0,
            fz: 0.0,
            mass: (deg[i] + 1) as f32,
        });
    }

    local_optimize(&mut bodies, &es, &ed);

    let out_nodes: Vec<OutNode> = nodes
        .iter()
        .enumerate()
        .map(|(i, nd)| {
            let deg_boost = if deg[i] > 5 {
                (deg[i] as f32 * 0.3).min(10.0)
            } else {
                0.0
            };
            OutNode {
                id: i,
                x: finite(bodies[i].x),
                y: finite(bodies[i].y),
                z: finite(bodies[i].z),
                label: nd.label.clone(),
                name: nd.name.clone(),
                file_path: nd.file_path.clone(),
                size: size_for_label(&nd.label) + deg_boost,
                color: stellar_color(deg[i]),
            }
        })
        .collect();

    let out_edges: Vec<OutEdge> = (0..es.len())
        .map(|e| OutEdge {
            source: es[e],
            target: ed[e],
            edge_type: etype[e].clone(),
        })
        .collect();

    GraphData {
        nodes: out_nodes,
        edges: out_edges,
        total_nodes,
    }
}

fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(label: &str, qn: &str, fp: Option<&str>) -> Node {
        Node {
            label: label.into(),
            name: qn.into(),
            file_path: fp.map(|s| s.into()),
            qualified_name: qn.into(),
        }
    }

    fn edge(s: usize, t: usize) -> Edge {
        Edge {
            source: s,
            target: t,
            edge_type: "Calls".into(),
        }
    }

    #[test]
    fn empty_graph_is_empty_but_reports_total() {
        let g = compute(&[], &[], 7);
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
        assert_eq!(g.total_nodes, 7);
    }

    #[test]
    fn every_node_gets_a_finite_position_and_sequential_id() {
        let nodes = vec![
            node("File", "src/a.rs", Some("src/a.rs")),
            node("Function", "src/a.rs::foo", Some("src/a.rs")),
            node("Function", "src/b.rs::bar", Some("src/b.rs")),
        ];
        let edges = vec![edge(0, 1), edge(1, 2)];
        let g = compute(&nodes, &edges, 3);
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 2);
        for (i, nd) in g.nodes.iter().enumerate() {
            assert_eq!(nd.id, i);
            assert!(nd.x.is_finite() && nd.y.is_finite() && nd.z.is_finite());
            assert!(nd.color.starts_with('#') && nd.color.len() == 7);
            assert!(nd.size > 0.0);
        }
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let nodes = vec![
            node("File", "a", Some("x/y/a.rs")),
            node("Function", "a::f", Some("x/y/a.rs")),
            node("Class", "a::C", Some("x/z/a.rs")),
        ];
        let edges = vec![edge(0, 1), edge(0, 2), edge(1, 2)];
        let g1 = compute(&nodes, &edges, 3);
        let g2 = compute(&nodes, &edges, 3);
        for (a, b) in g1.nodes.iter().zip(g2.nodes.iter()) {
            assert_eq!(a.x.to_bits(), b.x.to_bits());
            assert_eq!(a.y.to_bits(), b.y.to_bits());
            assert_eq!(a.z.to_bits(), b.z.to_bits());
        }
    }

    #[test]
    fn higher_degree_gets_hotter_color_and_bigger_size() {
        // hub (idx 0) connected to 8 leaves
        let mut nodes = vec![node("Function", "hub", Some("h.rs"))];
        let mut edges = Vec::new();
        for i in 1..=8 {
            nodes.push(node("Function", &format!("leaf{i}"), Some("h.rs")));
            edges.push(edge(0, i));
        }
        let g = compute(&nodes, &edges, nodes.len());
        let hub = &g.nodes[0];
        let leaf = &g.nodes[1];
        // hub degree 8 → warmer-than-red color + degree boost on size
        assert_ne!(hub.color, "#ff6050");
        assert!(hub.size > leaf.size);
        assert_eq!(leaf.color, "#ff6050"); // degree 1 → red dwarf
    }

    #[test]
    fn out_of_range_edges_are_ignored() {
        let nodes = vec![node("File", "a", Some("a.rs"))];
        let edges = vec![edge(0, 99), edge(5, 0)];
        let g = compute(&nodes, &edges, 1);
        assert_eq!(g.nodes.len(), 1);
        assert!(g.edges.is_empty());
    }

    #[test]
    fn edge_serializes_type_field() {
        let v = serde_json::to_value(OutEdge {
            source: 0,
            target: 1,
            edge_type: "Imports".into(),
        })
        .unwrap();
        assert_eq!(v["type"], "Imports");
    }
}

// Node renderer: one InstancedMesh of spheres for the whole graph (scales to
// tens of thousands of nodes). Color comes from the server-computed stellar
// palette; bloom turns bright stars into glowing coronas. Hover/click resolve
// via the picked instanceId.
//
// Technique adapted from codebase-memory-mcp's graph-ui/src/components/NodeCloud.tsx
// (MIT, © 2025 DeusData).

import { useMemo, useRef } from "react";
import { useFrame } from "@react-three/fiber";
import * as THREE from "three";
import type { GraphNode } from "./types";

interface Props {
  nodes: GraphNode[];
  highlightedIds: Set<number> | null;
  onHover: (node: GraphNode | null) => void;
  onClick: (node: GraphNode) => void;
}

export function NodeCloud({ nodes, highlightedIds, onHover, onClick }: Props) {
  const meshRef = useRef<THREE.InstancedMesh>(null);
  const tempObj = useMemo(() => new THREE.Object3D(), []);
  const tempColor = useMemo(() => new THREE.Color(), []);

  // Per-instance colors — dim non-highlighted nodes, boost bright stars so
  // bloom picks up the excess as a halo.
  const colors = useMemo(() => {
    const arr = new Float32Array(nodes.length * 3);
    const hasHighlight = highlightedIds && highlightedIds.size > 0;
    for (let i = 0; i < nodes.length; i++) {
      tempColor.set(nodes[i].color);
      if (hasHighlight && !highlightedIds!.has(nodes[i].id)) {
        tempColor.multiplyScalar(0.12);
      } else {
        const brightness = (tempColor.r + tempColor.g + tempColor.b) / 3;
        tempColor.multiplyScalar(1.2 + brightness * 0.8);
      }
      arr[i * 3] = tempColor.r;
      arr[i * 3 + 1] = tempColor.g;
      arr[i * 3 + 2] = tempColor.b;
    }
    return arr;
  }, [nodes, highlightedIds, tempColor]);

  useFrame(() => {
    const mesh = meshRef.current;
    if (!mesh) return;
    const hasHighlight = highlightedIds && highlightedIds.size > 0;
    for (let i = 0; i < nodes.length; i++) {
      const n = nodes[i];
      tempObj.position.set(n.x, n.y, n.z);
      const isHi = !hasHighlight || highlightedIds!.has(n.id);
      const s = n.size * (isHi ? 0.5 : 0.2);
      tempObj.scale.set(s, s, s);
      tempObj.updateMatrix();
      mesh.setMatrixAt(i, tempObj.matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
    mesh.computeBoundingSphere();
  });

  return (
    <instancedMesh
      ref={meshRef}
      args={[undefined, undefined, nodes.length]}
      frustumCulled={false}
      onPointerOver={(e) => {
        e.stopPropagation();
        if (e.instanceId !== undefined && e.instanceId < nodes.length) onHover(nodes[e.instanceId]);
      }}
      onPointerOut={() => onHover(null)}
      onClick={(e) => {
        e.stopPropagation();
        if (e.instanceId !== undefined && e.instanceId < nodes.length) onClick(nodes[e.instanceId]);
      }}
    >
      <sphereGeometry args={[1, 16, 12]} />
      <meshBasicMaterial vertexColors toneMapped={false} />
      <instancedBufferAttribute attach="geometry-attributes-color" args={[colors, 3]} />
    </instancedMesh>
  );
}

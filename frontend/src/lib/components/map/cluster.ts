/**
 * Cluster computation for genuinely overlapping map markers.
 *
 * The results map should stay readable and map-like, so grouping is based on
 * actual pin proximity in container-pixel space rather than label-box
 * collisions. This keeps clustering focused on true marker overlap instead of
 * hiding nearby-but-distinct guesses behind a synthetic centroid.
 */
import type L from 'leaflet';

export interface ClusterGuess {
  userId: string;
  lat: number;
  lng: number;
  displayName: string;
  distanceMeters: number;
  color?: string;
  isCurrentUser?: boolean;
}

export interface Cluster {
  /** Stable id derived from sorted member user ids. */
  id: string;
  /** Cluster centroid as a Leaflet LatLng. */
  centroidLatLng: L.LatLng;
  /** Centroid in container-pixel space (reference frame at cluster time). */
  centroidPoint: { x: number; y: number };
  /** Cluster members. Current user first, otherwise input order. */
  members: ClusterGuess[];
  /** True if the current user is part of this cluster. */
  containsCurrentUser: boolean;
}

export interface ClusterResult {
  clusters: Cluster[];
  /** Guesses that did not collide with any other marker. */
  singletons: ClusterGuess[];
}

export interface ClusterOptions {
  pinRadiusPx: number;
}

// ────────────────────────────────────────────────────────────────────────
// Collision helpers
// ────────────────────────────────────────────────────────────────────────

interface ProjectedPoint extends ClusterGuess {
  x: number;
  y: number;
  sourceIndex: number;
}

function project(
  guesses: ClusterGuess[],
  map: L.Map,
): ProjectedPoint[] {
  const out: ProjectedPoint[] = [];
  guesses.forEach((g, sourceIndex) => {
    const pt = map.latLngToContainerPoint([g.lat, g.lng]);
    out.push({ ...g, x: pt.x, y: pt.y, sourceIndex });
  });
  return out;
}

function pinsOverlap(a: ProjectedPoint, b: ProjectedPoint, radius: number): boolean {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  const rr = (radius + radius) * (radius + radius);
  return dx * dx + dy * dy <= rr;
}

function shouldCluster(
  a: ProjectedPoint,
  b: ProjectedPoint,
  pinRadius: number,
): boolean {
  return pinsOverlap(a, b, pinRadius);
}

// ────────────────────────────────────────────────────────────────────────
// Union-find
// ────────────────────────────────────────────────────────────────────────

class UnionFind {
  private parent: number[];
  private rank: number[];

  constructor(size: number) {
    this.parent = Array.from({ length: size }, (_, i) => i);
    this.rank = new Array(size).fill(0);
  }

  find(x: number): number {
    while (this.parent[x] !== x) {
      this.parent[x] = this.parent[this.parent[x]]; // path compression
      x = this.parent[x];
    }
    return x;
  }

  union(a: number, b: number): void {
    const ra = this.find(a);
    const rb = this.find(b);
    if (ra === rb) return;
    if (this.rank[ra] < this.rank[rb]) {
      this.parent[ra] = rb;
    } else if (this.rank[ra] > this.rank[rb]) {
      this.parent[rb] = ra;
    } else {
      this.parent[rb] = ra;
      this.rank[ra] += 1;
    }
  }
}

// ────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────

/**
 * Build clusters from a list of guesses using the provided Leaflet map for
 * container-pixel projection.
 */
export function buildClusters(
  guesses: ClusterGuess[],
  map: L.Map,
  options: ClusterOptions,
): ClusterResult {
  if (guesses.length === 0) {
    return { clusters: [], singletons: [] };
  }

  const projected = project(guesses, map);
  const uf = new UnionFind(projected.length);

  // Build collision graph. O(n^2) over at most ~16 players — trivial.
  for (let i = 0; i < projected.length; i += 1) {
    for (let j = i + 1; j < projected.length; j += 1) {
      if (shouldCluster(projected[i], projected[j], options.pinRadiusPx)) {
        uf.union(i, j);
      }
    }
  }

  // Bucket by component root.
  const buckets = new Map<number, number[]>();
  for (let i = 0; i < projected.length; i += 1) {
    const root = uf.find(i);
    const arr = buckets.get(root) ?? [];
    arr.push(i);
    buckets.set(root, arr);
  }

  const clusters: Cluster[] = [];
  const singletons: ClusterGuess[] = [];

  for (const indices of buckets.values()) {
    if (indices.length === 1) {
      const p = projected[indices[0]];
      // Strip the projected fields before exposing.
      singletons.push(stripProjection(p));
      continue;
    }

    // Centroid in container-pixel space, then convert back to LatLng.
    let sx = 0;
    let sy = 0;
    for (const i of indices) {
      sx += projected[i].x;
      sy += projected[i].y;
    }
    const cx = sx / indices.length;
    const cy = sy / indices.length;
    const centroidLatLng = map.containerPointToLatLng([cx, cy]);

    // Order members: current user first, then by original input order for stability.
    const members = indices
      .map((i) => projected[i])
      .sort((a, b) => {
        if (a.isCurrentUser && !b.isCurrentUser) return -1;
        if (!a.isCurrentUser && b.isCurrentUser) return 1;
        return a.sourceIndex - b.sourceIndex;
      })
      .map(stripProjection);

    const id = members
      .map((m) => m.userId || `lat${m.lat}lng${m.lng}`)
      .slice()
      .sort()
      .join('|');

    const containsCurrentUser = members.some((m) => m.isCurrentUser === true);

    clusters.push({
      id,
      centroidLatLng,
      centroidPoint: { x: cx, y: cy },
      members,
      containsCurrentUser,
    });
  }

  return { clusters, singletons };
}

function stripProjection(p: ProjectedPoint): ClusterGuess {
  const { x: _x, y: _y, sourceIndex: _sourceIndex, ...rest } = p;
  return rest;
}

// ────────────────────────────────────────────────────────────────────────
// Spread geometry
// ────────────────────────────────────────────────────────────────────────

export interface SpreadPosition {
  /** Member index within the cluster. */
  index: number;
  /** Offset from centroid, in container pixels. */
  dx: number;
  dy: number;
  /** Absolute container-pixel position. */
  x: number;
  y: number;
  /** Tooltip placement hint for this position. */
  tooltipDir: 'left' | 'right' | 'top' | 'bottom';
}

export interface SpreadOptions {
  minRadiusPx: number;
  perMemberPx: number;
  maxRadiusPx: number;
  edgeMarginPx: number;
  mapSize: { width: number; height: number };
}

/**
 * Compute radial spread positions for a cluster's members.
 *
 * The first position is placed at -90° (above the centroid). Angles are
 * evenly distributed. The entire arc is rotated to find an orientation that
 * keeps all positions inside the map viewport with edge margin; if none fit,
 * the radius is reduced in steps down to the minimum.
 *
 * Each position is assigned a tooltip direction (`left` / `right`) so labels
 * point away from the centroid and don't overrun the viewport edge.
 */
export function computeSpreadPositions(
  centroid: { x: number; y: number },
  count: number,
  options: SpreadOptions,
): SpreadPosition[] {
  if (count <= 0) return [];

  const { minRadiusPx, perMemberPx, maxRadiusPx, edgeMarginPx, mapSize } = options;

  // Start radius scales with member count but is clamped.
  const idealRadius = Math.min(
    maxRadiusPx,
    Math.max(minRadiusPx, minRadiusPx + perMemberPx * (count - 1)),
  );

  // Try rotations first, then progressively smaller radii.
  const rotationSteps = 12; // 30° increments
  const radiusSteps = 5;

  for (let rs = 0; rs < radiusSteps; rs += 1) {
    const radius = Math.max(
      minRadiusPx,
      idealRadius - rs * ((idealRadius - minRadiusPx) / radiusSteps),
    );

    for (let rot = 0; rot < rotationSteps; rot += 1) {
      const startAngle = -Math.PI / 2 + (rot * 2 * Math.PI) / rotationSteps;
      const positions = placeOnArc(centroid, count, radius, startAngle, mapSize, edgeMarginPx);
      if (positions) return positions;
    }
  }

  // Fallback: use minimum radius with -90° start even if slightly off-screen.
  return placeOnArc(
    centroid,
    count,
    minRadiusPx,
    -Math.PI / 2,
    mapSize,
    edgeMarginPx,
    /* allowOffscreen */ true,
  )!;
}

function placeOnArc(
  centroid: { x: number; y: number },
  count: number,
  radius: number,
  startAngle: number,
  mapSize: { width: number; height: number },
  edgeMargin: number,
  allowOffscreen = false,
): SpreadPosition[] | null {
  const result: SpreadPosition[] = [];
  const angleStep = (2 * Math.PI) / count;
  for (let i = 0; i < count; i += 1) {
    const angle = startAngle + i * angleStep;
    const dx = Math.cos(angle) * radius;
    const dy = Math.sin(angle) * radius;
    const x = centroid.x + dx;
    const y = centroid.y + dy;

    if (!allowOffscreen) {
      if (
        x < edgeMargin ||
        x > mapSize.width - edgeMargin ||
        y < edgeMargin ||
        y > mapSize.height - edgeMargin
      ) {
        return null;
      }
    }

    // Choose tooltip direction: favour the side that has more room along x;
    // for positions near the top/bottom edge, fall back to top/bottom.
    let tooltipDir: SpreadPosition['tooltipDir'] = dx >= 0 ? 'right' : 'left';
    if (y < edgeMargin * 2) tooltipDir = 'bottom';
    else if (y > mapSize.height - edgeMargin * 2) tooltipDir = 'top';

    result.push({ index: i, dx, dy, x, y, tooltipDir });
  }
  return result;
}

// ────────────────────────────────────────────────────────────────────────
// Label measurement cache
// ────────────────────────────────────────────────────────────────────────

/**
 * Caches text-width measurements for nametag labels. Uses a single off-DOM
 * canvas context configured with the tooltip font.
 */
export class LabelMeasurer {
  private ctx: CanvasRenderingContext2D | null;
  private cache = new Map<string, number>();
  private readonly font: string;
  private readonly lineHeight: number;

  constructor(font: string, lineHeight: number) {
    this.font = font;
    this.lineHeight = lineHeight;
    if (typeof document !== 'undefined') {
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      if (ctx) ctx.font = font;
      this.ctx = ctx;
    } else {
      this.ctx = null;
    }
  }

  measure(text: string): { width: number; height: number } {
    if (!this.ctx) {
      // SSR/fallback estimate: ~7px per char.
      return { width: text.length * 7, height: this.lineHeight };
    }
    let width = this.cache.get(text);
    if (width === undefined) {
      this.ctx.font = this.font;
      width = this.ctx.measureText(text).width;
      this.cache.set(text, width);
    }
    return { width, height: this.lineHeight };
  }
}

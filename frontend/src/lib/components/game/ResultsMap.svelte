<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';
  import {
    MAP_TILES,
    MAP_DEFAULTS,
    MARKER_CONFIG,
    RESULTS_ANIMATION,
    CLUSTER_CONFIG,
  } from '$lib/config/map';
  import {
    createMapPinIcon,
    createTargetIcon,
    createClusterTokenIcon,
    buildClusters,
    computeSpreadPositions,
    LabelMeasurer,
    type Cluster,
    type ClusterGuess,
    type SpreadPosition,
  } from '$lib/components/map';
  import ClusterPopover, {
    type PopoverMember,
  } from '$lib/components/map/ClusterPopover.svelte';
  import { formatDistance } from '$lib/utils';

  interface Guess {
    lat: number;
    lng: number;
    displayName: string;
    userId: string;
    distanceMeters: number;
    color?: string;
  }

  interface Props {
    correctLat: number;
    correctLng: number;
    guesses: Guess[];
    /** Current user's ID to highlight their guess specially. */
    currentUserId?: string;
    /** Enable staggered reveal animation (default: true). */
    animated?: boolean;
    /**
     * Externally-driven highlight (e.g., from the results table). When this
     * matches a singleton's userId, that pin is emphasized. When it matches a
     * cluster member's userId, the cluster token is emphasized.
     */
    highlightedUserId?: string | null;
    /**
     * Fires when the pointer hovers, focus enters, or the user selects a
     * marker (singleton or cluster member). Null when hover / focus leaves.
     */
    onMarkerHover?: (userId: string | null) => void;
  }

  let {
    correctLat,
    correctLng,
    guesses,
    currentUserId = '',
    animated = true,
    highlightedUserId = null,
    onMarkerHover,
  }: Props = $props();

  // ─── Component state ────────────────────────────────────────────────
  let container: HTMLDivElement;
  let wrapper: HTMLDivElement;
  let map: L.Map | null = null;
  let leaflet: typeof L | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let labelMeasurer: LabelMeasurer | null = null;

  // Layer groups — stable across recomputes.
  let targetLayer: L.LayerGroup | null = null;
  let entityLayer: L.LayerGroup | null = null; // singletons + cluster tokens
  let singletonLineLayer: L.LayerGroup | null = null;
  let spreadLayer: L.LayerGroup | null = null;

  // Entity bookkeeping — rebuilt on each recompute.
  type SingletonHandle = {
    kind: 'singleton';
    userId: string;
    guess: ClusterGuess;
    marker: L.Marker;
    line: L.Polyline | null;
    element: HTMLElement | null;
  };
  type ClusterHandle = {
    kind: 'cluster';
    cluster: Cluster;
    marker: L.Marker;
    element: HTMLElement | null;
  };
  type Entity = SingletonHandle | ClusterHandle;

  let entities: Entity[] = [];
  let revealTimers: number[] = [];
  /**
   * Whether the current `renderAll` call should play the staggered reveal
   * animation. True only on the very first render after mount; zoom-triggered
   * recomputes snap into place instantly to avoid replaying the intro.
   */
  let useRevealAnimation = true;

  // Open-cluster interaction state.
  type OpenState = {
    clusterId: string;
    mode: 'radial' | 'popover';
    kind: 'sticky' | 'hover-preview';
  };
  let openState = $state<OpenState | null>(null);
  let hoverCloseTimer: number | null = null;
  let hoverOpenTimer: number | null = null;

  // Popover placement / members (derived from openState when popover mode).
  let popoverInfo = $state<{
    x: number;
    y: number;
    members: PopoverMember[];
    hoverPreview: boolean;
    containerWidth: number;
    containerHeight: number;
  } | null>(null);

  // Detect prefers-reduced-motion once at mount.
  let reducedMotion = false;
  // Detect fine pointer (desktop) for hover preview.
  let finePointer = false;

  // ─── Helpers ───────────────────────────────────────────────────────
  function categorizeGuesses(all: Guess[]) {
    let currentUserGuess: Guess | null = null;
    const otherGuesses: Guess[] = [];
    for (const g of all) {
      if (g.userId === currentUserId || (currentUserId === '' && g.userId === '')) {
        currentUserGuess = g;
      } else {
        otherGuesses.push(g);
      }
    }
    return { currentUserGuess, otherGuesses };
  }

  function escapeHtml(value: string): string {
    return value
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#39;');
  }

  function getDisplayName(name: string, isYou: boolean): string {
    const trimmed = name.trim();
    return trimmed || (isYou ? 'You' : 'Player');
  }

  function formatLabel(name: string, distanceMeters: number, isYou: boolean): string {
    const dist = formatDistance(distanceMeters);
    const displayName = escapeHtml(getDisplayName(name, isYou));
    if (isYou && displayName !== 'You') {
      return `<strong>${displayName}</strong> <span class="results-tooltip-self-tag">(You)</span> &middot; ${dist}`;
    }
    return `<strong>${displayName}</strong> &middot; ${dist}`;
  }

  /** Plain text label used for bounding-box estimation (no HTML). */
  function labelPlainText(name: string, distanceMeters: number, isYou: boolean): string {
    const display = getDisplayName(name, isYou);
    const dist = formatDistance(distanceMeters);
    if (isYou && display !== 'You') return `${display} (You) · ${dist}`;
    return `${display} · ${dist}`;
  }

  function toClusterGuess(g: Guess): ClusterGuess {
    const isYou = g.userId === currentUserId || (currentUserId === '' && g.userId === '');
    return {
      userId: g.userId,
      lat: g.lat,
      lng: g.lng,
      displayName: g.displayName,
      distanceMeters: g.distanceMeters,
      color: g.color,
      isCurrentUser: isYou,
      labelText: labelPlainText(g.displayName, g.distanceMeters, isYou),
    };
  }

  function animationDelay(index: number, isFirst: boolean): number {
    if (!useRevealAnimation || !animated || reducedMotion) return 0;
    if (isFirst) return RESULTS_ANIMATION.currentUserDelay;
    return (
      RESULTS_ANIMATION.currentUserDelay +
      100 +
      RESULTS_ANIMATION.playerStagger * index
    );
  }

  function applyRevealAnimation(element: HTMLElement | null, delayMs: number) {
    if (!element) return;
    if (!useRevealAnimation || !animated || reducedMotion) {
      element.style.opacity = '1';
      return;
    }
    element.style.opacity = '0';
    const timer = window.setTimeout(() => {
      element.classList.add('marker-animated');
      element.style.animationDelay = '0ms';
      element.style.opacity = '';
    }, delayMs);
    revealTimers.push(timer);
  }

  // ─── Rendering ─────────────────────────────────────────────────────
  /** Clears all tracked entities and reveal timers; keeps layer groups. */
  function clearEntities() {
    for (const t of revealTimers) clearTimeout(t);
    revealTimers = [];
    entityLayer?.clearLayers();
    singletonLineLayer?.clearLayers();
    entities = [];
  }

  function closeOpenCluster(immediate = false) {
    if (!openState) return;
    clearHoverTimers();

    // Remove active class from the underlying token.
    const cluster = entities.find(
      (e): e is ClusterHandle => e.kind === 'cluster' && e.cluster.id === openState!.clusterId,
    );
    cluster?.element?.classList.remove('is-active');

    if (openState.mode === 'radial') {
      if (immediate || reducedMotion) {
        spreadLayer?.clearLayers();
      } else {
        // Trigger the collapse animation on each spread member, then clear.
        const spreadLayers = spreadLayer?.getLayers() ?? [];
        for (const layer of spreadLayers) {
          // Only markers have a pin-container to animate; polylines are cleared
          // alongside but remain static.
          if ('getElement' in layer && typeof layer.getElement === 'function') {
            const el = (layer as L.Marker)
              .getElement()
              ?.querySelector<HTMLElement>('.cluster-spread-member');
            el?.classList.add('cluster-spread-out');
          }
        }
        window.setTimeout(() => {
          spreadLayer?.clearLayers();
        }, CLUSTER_CONFIG.collapseDurationMs);
      }
    }
    // Popover closure is handled by Svelte unmount when popoverInfo = null.

    openState = null;
    popoverInfo = null;
    // Do not emit onMarkerHover(null) here — external highlight (e.g., table
    // row hover) is independent of the cluster open/close lifecycle, and
    // clearing it here would briefly drop a hover that was set from outside.
  }

  function clearHoverTimers() {
    if (hoverCloseTimer !== null) {
      clearTimeout(hoverCloseTimer);
      hoverCloseTimer = null;
    }
    if (hoverOpenTimer !== null) {
      clearTimeout(hoverOpenTimer);
      hoverOpenTimer = null;
    }
  }

  /** Place the correct-location target marker (rendered once, stable). */
  function placeTarget(L: typeof import('leaflet'), _mapInstance: L.Map) {
    const icon = createTargetIcon(L, {
      color: MARKER_CONFIG.colors.correct,
      size: MARKER_CONFIG.resultSizes.correct,
      pulse: true,
    });
    const marker = L.marker([correctLat, correctLng], { icon }).addTo(targetLayer!);
    marker.bindTooltip('<strong>Correct Location</strong>', {
      permanent: true,
      direction: 'right',
      offset: [12, 0],
      className: 'results-tooltip results-tooltip-correct',
    });
    const el = marker.getElement()?.querySelector<HTMLElement>('.map-pin-container');
    const delay =
      useRevealAnimation && animated && !reducedMotion ? RESULTS_ANIMATION.correctDelay : 0;
    applyRevealAnimation(el ?? null, delay);
  }

  function placeSingleton(
    L: typeof import('leaflet'),
    g: ClusterGuess,
    index: number,
    isFirst: boolean,
  ): SingletonHandle {
    const color = g.color ?? MARKER_CONFIG.colors.players[index % MARKER_CONFIG.colors.players.length];
    const size = g.isCurrentUser
      ? MARKER_CONFIG.resultSizes.currentUser
      : MARKER_CONFIG.resultSizes.otherPlayer;

    const icon = createMapPinIcon(L, {
      color,
      size,
      glow: g.isCurrentUser,
      className: g.isCurrentUser ? 'is-current-user' : '',
    });

    const marker = L.marker([g.lat, g.lng], { icon }).addTo(entityLayer!);
    marker.bindTooltip(formatLabel(g.displayName, g.distanceMeters, g.isCurrentUser === true), {
      permanent: true,
      direction: 'right',
      offset: [12, 0],
      className: `results-tooltip ${g.isCurrentUser ? 'results-tooltip-you' : ''}`.trim(),
    });

    const wantAnim = useRevealAnimation && animated && !reducedMotion;
    const line = L.polyline(
      [[g.lat, g.lng], [correctLat, correctLng]],
      {
        color,
        weight: 2.5,
        opacity: wantAnim ? 0 : 0.7,
        dashArray: '8, 10',
      },
    ).addTo(singletonLineLayer!);

    const delay = animationDelay(index, isFirst);
    if (wantAnim) {
      const timer = window.setTimeout(() => line.setStyle({ opacity: 0.7 }), delay);
      revealTimers.push(timer);
    }

    const element = marker.getElement()?.querySelector<HTMLElement>('.map-pin-container') ?? null;
    applyRevealAnimation(element, delay + 100);

    // Hover / click → emit user id for bi-directional highlight
    marker.on('mouseover', () => onMarkerHover?.(g.userId));
    marker.on('mouseout', () => onMarkerHover?.(null));
    marker.on('click', () => onMarkerHover?.(g.userId));

    return { kind: 'singleton', userId: g.userId, guess: g, marker, line, element };
  }

  function placeCluster(
    L: typeof import('leaflet'),
    cluster: Cluster,
    index: number,
    isFirst: boolean,
  ): ClusterHandle {
    const colors = cluster.members.map(
      (m, i) => m.color ?? MARKER_CONFIG.colors.players[i % MARKER_CONFIG.colors.players.length],
    );

    const names = cluster.members.map((m) => m.displayName).join(', ');
    const ariaLabel = cluster.containsCurrentUser
      ? `Reveal ${cluster.members.length} overlapping players (including you): ${names}`
      : `Reveal ${cluster.members.length} overlapping players: ${names}`;

    const icon = createClusterTokenIcon(L, {
      count: cluster.members.length,
      containsCurrentUser: cluster.containsCurrentUser,
      memberColors: colors,
      ariaLabel,
    });

    const marker = L.marker(cluster.centroidLatLng, { icon }).addTo(entityLayer!);
    const element = marker.getElement()?.querySelector<HTMLElement>('.cluster-token') ?? null;

    // Stash cluster id on the DOM for event delegation / tests.
    if (element) element.dataset.clusterId = cluster.id;

    const delay = animationDelay(index, isFirst);
    applyRevealAnimation(element, delay);

    // ─── Interaction wiring ─────────────────────────────────────────
    marker.on('click', (e) => {
      // Stop the click from reaching the map (which would close open popover).
      L.DomEvent.stopPropagation(e.originalEvent as MouseEvent);
      toggleClusterSticky(cluster);
    });

    marker.on('mouseover', () => {
      if (!finePointer) return;
      if (openState?.kind === 'sticky') return; // don't steal focus from sticky
      scheduleHoverPreview(cluster);
    });
    marker.on('mouseout', () => {
      if (!finePointer) return;
      if (hoverOpenTimer !== null) {
        clearTimeout(hoverOpenTimer);
        hoverOpenTimer = null;
      }
      if (openState?.kind === 'hover-preview' && openState.clusterId === cluster.id) {
        scheduleHoverClose();
      }
    });

    // Keyboard activation on the cluster token element itself.
    if (element) {
      element.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          toggleClusterSticky(cluster);
        } else if (event.key === 'Escape') {
          event.preventDefault();
          closeOpenCluster();
        }
      });
    }

    return { kind: 'cluster', cluster, marker, element };
  }

  function scheduleHoverPreview(cluster: Cluster) {
    if (hoverCloseTimer !== null) {
      clearTimeout(hoverCloseTimer);
      hoverCloseTimer = null;
    }
    if (hoverOpenTimer !== null) clearTimeout(hoverOpenTimer);
    if (openState?.kind === 'hover-preview' && openState.clusterId === cluster.id) {
      return; // already shown
    }
    hoverOpenTimer = window.setTimeout(() => {
      openClusterPopover(cluster, 'hover-preview');
    }, CLUSTER_CONFIG.hoverPreviewDelayMs);
  }

  function scheduleHoverClose() {
    if (hoverCloseTimer !== null) clearTimeout(hoverCloseTimer);
    hoverCloseTimer = window.setTimeout(() => {
      if (openState?.kind === 'hover-preview') closeOpenCluster();
    }, CLUSTER_CONFIG.hoverLeaveGraceMs);
  }

  function toggleClusterSticky(cluster: Cluster) {
    // Cancel any pending hover open/close transitions so they can't race with
    // the sticky state we're about to set.
    clearHoverTimers();

    if (openState?.kind === 'sticky' && openState.clusterId === cluster.id) {
      closeOpenCluster();
      return;
    }
    // Close any existing open cluster first.
    if (openState) closeOpenCluster(true);

    const useRadial =
      cluster.members.length < CLUSTER_CONFIG.popoverThreshold && !reducedMotion;
    if (useRadial) {
      openClusterRadial(cluster);
    } else {
      openClusterPopover(cluster, 'sticky');
    }
  }

  function openClusterRadial(cluster: Cluster) {
    if (!map || !leaflet) return;
    const L = leaflet;

    const mapSize = map.getSize();
    const centroidPx = map.latLngToContainerPoint(cluster.centroidLatLng);

    const positions = computeSpreadPositions(
      { x: centroidPx.x, y: centroidPx.y },
      cluster.members.length,
      {
        minRadiusPx: CLUSTER_CONFIG.radialMinRadiusPx,
        perMemberPx: CLUSTER_CONFIG.radialPerMemberPx,
        maxRadiusPx: CLUSTER_CONFIG.radialMaxRadiusPx,
        edgeMarginPx: CLUSTER_CONFIG.edgeBiasMarginPx,
        mapSize: { width: mapSize.x, height: mapSize.y },
      },
    );

    spreadLayer?.clearLayers();

    const duration = reducedMotion ? 0 : CLUSTER_CONFIG.spreadDurationMs;

    positions.forEach((pos, i) => {
      const member = cluster.members[i];
      placeSpreadMember(L, cluster, member, pos, i, duration);
    });

    // Mark cluster active.
    const handle = entities.find(
      (e): e is ClusterHandle => e.kind === 'cluster' && e.cluster.id === cluster.id,
    );
    handle?.element?.classList.add('is-active');

    openState = { clusterId: cluster.id, mode: 'radial', kind: 'sticky' };
  }

  function placeSpreadMember(
    L: typeof import('leaflet'),
    cluster: Cluster,
    member: ClusterGuess,
    pos: SpreadPosition,
    indexInCluster: number,
    durationMs: number,
  ) {
    if (!map) return;
    const latlng = map.containerPointToLatLng(
      [pos.x, pos.y] as unknown as L.PointExpression,
    );

    const color =
      member.color ??
      MARKER_CONFIG.colors.players[indexInCluster % MARKER_CONFIG.colors.players.length];
    const size = member.isCurrentUser
      ? MARKER_CONFIG.resultSizes.currentUser
      : MARKER_CONFIG.resultSizes.otherPlayer;

    const icon = createMapPinIcon(L, {
      color,
      size,
      glow: member.isCurrentUser,
      className: `cluster-spread-member ${member.isCurrentUser ? 'is-current-user' : ''}`.trim(),
    });

    const marker = L.marker(latlng, { icon }).addTo(spreadLayer!);
    marker.bindTooltip(
      formatLabel(member.displayName, member.distanceMeters, member.isCurrentUser === true),
      {
        permanent: true,
        direction: pos.tooltipDir,
        offset: tooltipOffsetForDir(pos.tooltipDir),
        className: `results-tooltip ${member.isCurrentUser ? 'results-tooltip-you' : ''} cluster-spread-tooltip`.trim(),
      },
    );

    // Animate from centroid outward: translate the inner wrapper from (-dx, -dy) to (0, 0).
    const el = marker.getElement()?.querySelector<HTMLElement>('.cluster-spread-member');
    if (el) {
      el.style.setProperty('--spread-from-x', `${-pos.dx}px`);
      el.style.setProperty('--spread-from-y', `${-pos.dy}px`);
      el.style.setProperty('--spread-duration', `${durationMs}ms`);
    }

    // Leader line from spread position back to the correct target location.
    const line = L.polyline(
      [[latlng.lat, latlng.lng], [correctLat, correctLng]],
      {
        color,
        weight: 2,
        opacity: 0.6,
        dashArray: '6, 8',
      },
    ).addTo(spreadLayer!);
    void line;

    marker.on('mouseover', () => onMarkerHover?.(member.userId));
    marker.on('mouseout', () => onMarkerHover?.(null));
  }

  function tooltipOffsetForDir(dir: SpreadPosition['tooltipDir']): [number, number] {
    switch (dir) {
      case 'left':
        return [-12, 0];
      case 'top':
        return [0, -10];
      case 'bottom':
        return [0, 10];
      default:
        return [12, 0];
    }
  }

  function openClusterPopover(cluster: Cluster, kind: 'sticky' | 'hover-preview') {
    if (!map) return;
    const centroidPx = map.latLngToContainerPoint(cluster.centroidLatLng);
    const size = map.getSize();

    const members: PopoverMember[] = cluster.members.map((m, i) => ({
      userId: m.userId,
      displayName: getDisplayName(m.displayName, m.isCurrentUser === true),
      distanceMeters: m.distanceMeters,
      color:
        m.color ?? MARKER_CONFIG.colors.players[i % MARKER_CONFIG.colors.players.length],
      isCurrentUser: m.isCurrentUser === true,
    }));

    popoverInfo = {
      x: centroidPx.x,
      y: centroidPx.y,
      members,
      hoverPreview: kind === 'hover-preview',
      containerWidth: size.x,
      containerHeight: size.y,
    };

    const handle = entities.find(
      (e): e is ClusterHandle => e.kind === 'cluster' && e.cluster.id === cluster.id,
    );
    handle?.element?.classList.add('is-active');

    openState = { clusterId: cluster.id, mode: 'popover', kind };
  }

  // ─── Full render pipeline ───────────────────────────────────────────
  function renderAll(L: typeof import('leaflet'), mapInstance: L.Map) {
    if (!labelMeasurer) {
      labelMeasurer = new LabelMeasurer(
        CLUSTER_CONFIG.labelFont,
        CLUSTER_CONFIG.labelLineHeightPx,
      );
    }

    clearEntities();

    // Build cluster input (excludes the correct-location target by design).
    const clusterGuesses: ClusterGuess[] = guesses.map(toClusterGuess);

    const { clusters, singletons } = buildClusters(clusterGuesses, mapInstance, L, {
      pinRadiusPx: CLUSTER_CONFIG.pinRadiusPx,
      labelPaddingPx: CLUSTER_CONFIG.labelPaddingPx,
      labelAnchorOffsetPx: CLUSTER_CONFIG.labelAnchorOffsetPx,
      labelBoxFn: (g) => labelMeasurer!.measure(g.labelText),
    });

    // Decide which entity is the "first" for reveal sequencing.
    const firstCluster = clusters.find((c) => c.containsCurrentUser);
    const firstSingleton = firstCluster
      ? null
      : singletons.find((s) => s.isCurrentUser) ?? null;

    // Stable ordering: the "first" entity uses currentUserDelay; everyone else
    // is staggered in roughly score order by using the input index.
    const ordered: Array<{ entity: { kind: 'cluster'; cluster: Cluster } | { kind: 'singleton'; guess: ClusterGuess }; orderKey: number }> = [];

    for (const c of clusters) {
      if (c === firstCluster) continue;
      const minIdx = Math.min(
        ...c.members.map((m) => clusterGuesses.indexOf(m)),
      );
      ordered.push({ entity: { kind: 'cluster', cluster: c }, orderKey: minIdx });
    }
    for (const s of singletons) {
      if (s === firstSingleton) continue;
      const idx = clusterGuesses.indexOf(s);
      ordered.push({ entity: { kind: 'singleton', guess: s }, orderKey: idx });
    }
    ordered.sort((a, b) => a.orderKey - b.orderKey);

    let index = 0;
    if (firstCluster) {
      entities.push(placeCluster(L, firstCluster, index, true));
    } else if (firstSingleton) {
      entities.push(placeSingleton(L, firstSingleton, index, true));
    }

    for (const { entity } of ordered) {
      index += 1;
      if (entity.kind === 'cluster') {
        entities.push(placeCluster(L, entity.cluster, index, false));
      } else {
        entities.push(placeSingleton(L, entity.guess, index, false));
      }
    }

    applyHighlight();
  }

  /**
   * Reactively apply the externally-driven highlight to the correct entity.
   */
  function applyHighlight() {
    for (const e of entities) {
      const el = e.element;
      if (!el) continue;
      el.classList.remove('is-highlighted');
    }
    if (!highlightedUserId) return;
    for (const e of entities) {
      if (e.kind === 'singleton' && e.userId === highlightedUserId) {
        e.element?.classList.add('is-highlighted');
      } else if (
        e.kind === 'cluster' &&
        e.cluster.members.some((m) => m.userId === highlightedUserId)
      ) {
        e.element?.classList.add('is-highlighted');
      }
    }
  }

  // Apply highlight whenever the prop changes.
  $effect(() => {
    // Touching the prop creates reactivity.
    highlightedUserId;
    applyHighlight();
  });

  let hasInitialized = false;

  // Re-render when guesses change after the initial mount.
  $effect(() => {
    // Reactive dependency on guesses (array identity change triggers re-run).
    guesses;
    if (!hasInitialized) return;
    if (map && leaflet) {
      closeOpenCluster(true);
      // Fresh data: replay the staggered reveal.
      useRevealAnimation = true;
      renderAll(leaflet, map);
    }
  });

  // ─── Map events ─────────────────────────────────────────────────────
  // Zoom changes can alter which markers pixel-overlap, so we recompute
  // clusters on zoomend. Pure pans preserve relative distances, so we keep
  // the current layout during panning and don't force-close open clusters.
  function handleZoomStart() {
    closeOpenCluster(true);
  }

  function handleZoomEnd() {
    if (!map || !leaflet) return;
    useRevealAnimation = false;
    renderAll(leaflet, map);
  }

  function handleMapClick() {
    if (openState?.kind === 'sticky') closeOpenCluster();
  }

  // ─── Keyboard: global Escape when any cluster is open ──────────────
  function handleDocumentKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && openState) {
      event.preventDefault();
      closeOpenCluster();
    }
  }

  // ─── Mount / destroy ───────────────────────────────────────────────
  onMount(async () => {
    if (!browser) return;

    reducedMotion =
      window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    finePointer =
      window.matchMedia &&
      window.matchMedia('(hover: hover) and (pointer: fine)').matches;

    leaflet = (await import('leaflet')).default;

    map = leaflet.map(container, {
      center: [correctLat, correctLng],
      zoom: MAP_DEFAULTS.resultZoom,
      zoomControl: false,
      attributionControl: false,
    });

    leaflet
      .tileLayer(MAP_TILES.url, { maxZoom: MAP_TILES.maxZoom })
      .addTo(map);

    leaflet.control.zoom({ position: 'bottomright' }).addTo(map);

    targetLayer = leaflet.layerGroup().addTo(map);
    singletonLineLayer = leaflet.layerGroup().addTo(map);
    entityLayer = leaflet.layerGroup().addTo(map);
    spreadLayer = leaflet.layerGroup().addTo(map);

    // Fit bounds to all guesses + correct location.
    const bounds = leaflet.latLngBounds([[correctLat, correctLng]]);
    for (const g of guesses) bounds.extend([g.lat, g.lng]);

    // Target first (stable).
    placeTarget(leaflet, map);

    // Initial render.
    renderAll(leaflet, map);

    if (guesses.length > 0) {
      map.fitBounds(bounds, { padding: MAP_DEFAULTS.padding });
    }

    map.on('zoomstart', handleZoomStart);
    map.on('zoomend', handleZoomEnd);
    map.on('click', handleMapClick);

    document.addEventListener('keydown', handleDocumentKeydown);

    // Watch for container resize (e.g., window resize, card layout shift).
    resizeObserver = new ResizeObserver(() => {
      if (!map) return;
      map.invalidateSize();
    });
    resizeObserver.observe(container);

    setTimeout(() => map?.invalidateSize(), 100);

    // After the initial render completes, subsequent renderAll calls from
    // zoomend or guess-data changes should behave as updates.
    useRevealAnimation = false;
    hasInitialized = true;
  });

  onDestroy(() => {
    for (const t of revealTimers) clearTimeout(t);
    revealTimers = [];
    clearHoverTimers();
    resizeObserver?.disconnect();
    resizeObserver = null;
    document.removeEventListener('keydown', handleDocumentKeydown);
    if (map) {
      map.off();
      map.remove();
      map = null;
    }
    labelMeasurer = null;
  });

  // Popover event handlers (declared at top level so the callbacks are stable).
  function handlePopoverMemberHover(userId: string | null) {
    onMarkerHover?.(userId);
    if (openState?.kind === 'hover-preview' && userId !== null) {
      // User is interacting with the preview — cancel auto-close.
      clearHoverTimers();
    }
  }

  function handlePopoverClose() {
    closeOpenCluster();
  }

  function handlePopoverPointerLeave() {
    // Only hover-preview popovers auto-close on pointer leave. Sticky
    // popovers require an explicit close action (click outside, Esc, re-click).
    if (openState?.kind === 'hover-preview') scheduleHoverClose();
  }
</script>

<div
  bind:this={wrapper}
  class="results-map-wrapper relative w-full h-full"
  onmouseenter={() => clearHoverTimers()}
  role="presentation"
>
  <div bind:this={container} class="w-full h-full rounded-lg"></div>

  {#if popoverInfo}
    <ClusterPopover
      x={popoverInfo.x}
      y={popoverInfo.y}
      containerWidth={popoverInfo.containerWidth}
      containerHeight={popoverInfo.containerHeight}
      members={popoverInfo.members}
      hoverPreview={popoverInfo.hoverPreview}
      onMemberHover={handlePopoverMemberHover}
      onClose={handlePopoverClose}
      onPointerLeave={handlePopoverPointerLeave}
    />
  {/if}
</div>

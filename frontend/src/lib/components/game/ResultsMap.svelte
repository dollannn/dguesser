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
    type Cluster,
    type ClusterGuess,
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
    /** External highlight from the results table. */
    highlightedUserId?: string | null;
    /** Hover / focus callback used to sync the results table. */
    onMarkerHover?: (userId: string | null) => void;
  }

  const MAX_ALWAYS_VISIBLE_OTHER_LABELS = 4;

  let {
    correctLat,
    correctLng,
    guesses,
    currentUserId = '',
    animated = true,
    highlightedUserId = null,
    onMarkerHover,
  }: Props = $props();

  let container: HTMLDivElement;
  let wrapper: HTMLDivElement;
  let map: L.Map | null = null;
  let leaflet: typeof L | null = null;
  let resizeObserver: ResizeObserver | null = null;

  let targetLayer: L.LayerGroup | null = null;
  let lineLayer: L.LayerGroup | null = null;
  let entityLayer: L.LayerGroup | null = null;

  type EntityHandle = {
    kind: 'singleton' | 'cluster';
    element: HTMLElement | null;
    userIds: string[];
    clusterId?: string;
  };

  type LineHandle = {
    userId: string;
    line: L.Polyline;
    defaultOpacity: number;
    defaultWeight: number;
  };

  type OpenState = {
    clusterId: string;
  };

  let entities: EntityHandle[] = [];
  let lineHandles: LineHandle[] = [];
  let revealTimers: number[] = [];
  let useRevealAnimation = true;
  let reducedMotion = false;
  let hasInitialized = false;
  let openState = $state<OpenState | null>(null);
  let popoverInfo = $state<{
    x: number;
    y: number;
    members: PopoverMember[];
    containerWidth: number;
    containerHeight: number;
  } | null>(null);

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

  function clearEntities() {
    for (const t of revealTimers) clearTimeout(t);
    revealTimers = [];
    lineLayer?.clearLayers();
    entityLayer?.clearLayers();
    entities = [];
    lineHandles = [];
  }

  function closeOpenCluster() {
    if (!openState) return;

    const handle = entities.find(
      (entity) => entity.kind === 'cluster' && entity.clusterId === openState?.clusterId,
    );
    handle?.element?.classList.remove('is-active');

    openState = null;
    popoverInfo = null;
  }

  function placeTarget(L: typeof import('leaflet')) {
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

  function placeGuessLine(
    L: typeof import('leaflet'),
    guess: ClusterGuess,
    index: number,
    isFirst: boolean,
  ) {
    const color =
      guess.color ??
      (guess.isCurrentUser
        ? MARKER_CONFIG.colors.currentUser
        : MARKER_CONFIG.colors.players[index % MARKER_CONFIG.colors.players.length]);

    const defaultOpacity = guess.isCurrentUser ? 0.85 : 0.55;
    const defaultWeight = guess.isCurrentUser ? 3 : 2.25;
    const wantAnim = useRevealAnimation && animated && !reducedMotion;

    const line = L.polyline(
      [
        [guess.lat, guess.lng],
        [correctLat, correctLng],
      ],
      {
        color,
        weight: defaultWeight,
        opacity: wantAnim ? 0 : defaultOpacity,
        dashArray: '8, 10',
      },
    ).addTo(lineLayer!);

    if (wantAnim) {
      const delay = animationDelay(index, isFirst);
      const timer = window.setTimeout(() => {
        line.setStyle({ opacity: defaultOpacity });
      }, delay);
      revealTimers.push(timer);
    }

    lineHandles.push({
      userId: guess.userId,
      line,
      defaultOpacity,
      defaultWeight,
    });
  }

  function placeSingleton(
    L: typeof import('leaflet'),
    guess: ClusterGuess,
    index: number,
    isFirst: boolean,
    showLabel: boolean,
  ): EntityHandle {
    const color =
      guess.color ??
      (guess.isCurrentUser
        ? MARKER_CONFIG.colors.currentUser
        : MARKER_CONFIG.colors.players[index % MARKER_CONFIG.colors.players.length]);
    const size = guess.isCurrentUser
      ? MARKER_CONFIG.resultSizes.currentUser
      : MARKER_CONFIG.resultSizes.otherPlayer;

    const icon = createMapPinIcon(L, {
      color,
      size,
      glow: guess.isCurrentUser,
      className: guess.isCurrentUser ? 'is-current-user' : '',
    });

    const marker = L.marker([guess.lat, guess.lng], { icon }).addTo(entityLayer!);
    marker.bindTooltip(formatLabel(guess.displayName, guess.distanceMeters, guess.isCurrentUser === true), {
      permanent: showLabel,
      direction: 'right',
      offset: [12, 0],
      className: `results-tooltip ${guess.isCurrentUser ? 'results-tooltip-you' : ''}`.trim(),
    });

    const delay = animationDelay(index, isFirst);
    const element = marker.getElement()?.querySelector<HTMLElement>('.map-pin-container') ?? null;
    applyRevealAnimation(element, delay + 100);

    marker.on('mouseover', () => onMarkerHover?.(guess.userId));
    marker.on('mouseout', () => onMarkerHover?.(null));
    marker.on('click', () => onMarkerHover?.(guess.userId));

    return {
      kind: 'singleton',
      element,
      userIds: [guess.userId],
    };
  }

  function placeCluster(
    L: typeof import('leaflet'),
    cluster: Cluster,
    index: number,
    isFirst: boolean,
  ): EntityHandle {
    const names = cluster.members.map((member) => getDisplayName(member.displayName, member.isCurrentUser === true));
    const ariaLabel = cluster.containsCurrentUser
      ? `Show ${cluster.members.length} overlapping guesses, including you: ${names.join(', ')}`
      : `Show ${cluster.members.length} overlapping guesses: ${names.join(', ')}`;

    const icon = createClusterTokenIcon(L, {
      count: cluster.members.length,
      containsCurrentUser: cluster.containsCurrentUser,
      ariaLabel,
    });

    const marker = L.marker(cluster.centroidLatLng, { icon }).addTo(entityLayer!);
    const element =
      marker.getElement()?.querySelector<HTMLElement>('.cluster-pin-container') ?? null;

    if (element) element.dataset.clusterId = cluster.id;

    const delay = animationDelay(index, isFirst);
    applyRevealAnimation(element, delay);

    marker.on('click', (event) => {
      L.DomEvent.stopPropagation(event.originalEvent as MouseEvent);
      toggleCluster(cluster);
    });

    if (element) {
      element.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          toggleCluster(cluster);
        } else if (event.key === 'Escape') {
          event.preventDefault();
          closeOpenCluster();
          onMarkerHover?.(null);
        }
      });
    }

    return {
      kind: 'cluster',
      element,
      userIds: cluster.members.map((member) => member.userId),
      clusterId: cluster.id,
    };
  }

  function toggleCluster(cluster: Cluster) {
    if (openState?.clusterId === cluster.id) {
      closeOpenCluster();
      onMarkerHover?.(null);
      return;
    }

    closeOpenCluster();
    openClusterPopover(cluster);
  }

  function openClusterPopover(cluster: Cluster) {
    if (!map) return;

    const centroidPx = map.latLngToContainerPoint(cluster.centroidLatLng);
    const size = map.getSize();

    popoverInfo = {
      x: centroidPx.x,
      y: centroidPx.y,
      members: cluster.members.map((member, index) => ({
        userId: member.userId,
        displayName: getDisplayName(member.displayName, member.isCurrentUser === true),
        distanceMeters: member.distanceMeters,
        color:
          member.color ??
          (member.isCurrentUser
            ? MARKER_CONFIG.colors.currentUser
            : MARKER_CONFIG.colors.players[index % MARKER_CONFIG.colors.players.length]),
        isCurrentUser: member.isCurrentUser === true,
      })),
      containerWidth: size.x,
      containerHeight: size.y,
    };

    const handle = entities.find(
      (entity) => entity.kind === 'cluster' && entity.clusterId === cluster.id,
    );
    handle?.element?.classList.add('is-active');

    openState = { clusterId: cluster.id };
  }

  function renderAll(L: typeof import('leaflet'), mapInstance: L.Map) {
    clearEntities();

    const clusterGuesses = guesses.map(toClusterGuess);
    const { currentUserGuess, otherGuesses } = categorizeGuesses(guesses);
    const currentLineGuess = currentUserGuess ? toClusterGuess(currentUserGuess) : null;
    const otherLineGuesses = otherGuesses.map(toClusterGuess);

    if (currentLineGuess) {
      placeGuessLine(L, currentLineGuess, 0, true);
    }

    otherLineGuesses.forEach((guess, index) => {
      placeGuessLine(L, guess, index + 1, false);
    });

    const { clusters, singletons } = buildClusters(clusterGuesses, mapInstance, {
      pinRadiusPx: CLUSTER_CONFIG.pinRadiusPx,
    });

    const guessOrderKey = (candidate: ClusterGuess) =>
      clusterGuesses.findIndex(
        (guess) =>
          guess.userId === candidate.userId &&
          guess.lat === candidate.lat &&
          guess.lng === candidate.lng,
      );

    const firstCluster = clusters.find((cluster) => cluster.containsCurrentUser);
    const firstSingleton = firstCluster
      ? null
      : singletons.find((guess) => guess.isCurrentUser) ?? null;

    const ordered: Array<
      | { entity: { kind: 'cluster'; cluster: Cluster }; orderKey: number }
      | { entity: { kind: 'singleton'; guess: ClusterGuess }; orderKey: number }
    > = [];

    for (const cluster of clusters) {
      if (cluster === firstCluster) continue;
      const minIdx = Math.min(...cluster.members.map((member) => guessOrderKey(member)));
      ordered.push({ entity: { kind: 'cluster', cluster }, orderKey: minIdx });
    }

    for (const guess of singletons) {
      if (guess === firstSingleton) continue;
      ordered.push({
        entity: { kind: 'singleton', guess },
        orderKey: guessOrderKey(guess),
      });
    }

    ordered.sort((a, b) => a.orderKey - b.orderKey);

    let index = 0;
    const showOtherLabels = otherGuesses.length <= MAX_ALWAYS_VISIBLE_OTHER_LABELS;

    if (firstCluster) {
      entities.push(placeCluster(L, firstCluster, index, true));
    } else if (firstSingleton) {
      entities.push(
        placeSingleton(L, firstSingleton, index, true, firstSingleton.isCurrentUser === true || showOtherLabels),
      );
    }

    for (const { entity } of ordered) {
      index += 1;

      if (entity.kind === 'cluster') {
        entities.push(placeCluster(L, entity.cluster, index, false));
      } else {
        entities.push(
          placeSingleton(
            L,
            entity.guess,
            index,
            false,
            entity.guess.isCurrentUser === true || showOtherLabels,
          ),
        );
      }
    }

    applyHighlight();
  }

  function applyHighlight() {
    for (const entity of entities) {
      entity.element?.classList.remove('is-highlighted');
    }

    for (const handle of lineHandles) {
      handle.line.setStyle({
        opacity: handle.defaultOpacity,
        weight: handle.defaultWeight,
      });
    }

    if (!highlightedUserId) return;

    for (const entity of entities) {
      if (entity.userIds.includes(highlightedUserId)) {
        entity.element?.classList.add('is-highlighted');
      }
    }

    for (const handle of lineHandles) {
      if (handle.userId !== highlightedUserId) continue;
      handle.line.setStyle({
        opacity: 0.95,
        weight: handle.defaultWeight + 0.9,
      });
      handle.line.bringToFront();
    }
  }

  $effect(() => {
    highlightedUserId;
    applyHighlight();
  });

  $effect(() => {
    guesses;
    if (!hasInitialized || !map || !leaflet) return;

    closeOpenCluster();
    useRevealAnimation = true;
    renderAll(leaflet, map);
  });

  function handleZoomStart() {
    closeOpenCluster();
    onMarkerHover?.(null);
  }

  function handleZoomEnd() {
    if (!map || !leaflet) return;
    useRevealAnimation = false;
    renderAll(leaflet, map);
  }

  function handleMapClick() {
    closeOpenCluster();
    onMarkerHover?.(null);
  }

  function handleDocumentKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !openState) return;

    event.preventDefault();
    closeOpenCluster();
    onMarkerHover?.(null);
  }

  onMount(async () => {
    if (!browser) return;

    reducedMotion =
      window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    leaflet = (await import('leaflet')).default;

    map = leaflet.map(container, {
      center: [correctLat, correctLng],
      zoom: MAP_DEFAULTS.resultZoom,
      zoomControl: false,
      attributionControl: false,
    });

    leaflet.tileLayer(MAP_TILES.url, { maxZoom: MAP_TILES.maxZoom }).addTo(map);
    leaflet.control.zoom({ position: 'bottomright' }).addTo(map);

    targetLayer = leaflet.layerGroup().addTo(map);
    lineLayer = leaflet.layerGroup().addTo(map);
    entityLayer = leaflet.layerGroup().addTo(map);

    const bounds = leaflet.latLngBounds([[correctLat, correctLng]]);
    for (const guess of guesses) bounds.extend([guess.lat, guess.lng]);

    if (guesses.length > 0) {
      map.fitBounds(bounds, { padding: MAP_DEFAULTS.padding });
    }

    placeTarget(leaflet);
    renderAll(leaflet, map);

    map.on('zoomstart', handleZoomStart);
    map.on('zoomend', handleZoomEnd);
    map.on('click', handleMapClick);

    document.addEventListener('keydown', handleDocumentKeydown);

    resizeObserver = new ResizeObserver(() => {
      if (!map || !leaflet) return;
      map.invalidateSize();
      closeOpenCluster();
      useRevealAnimation = false;
      renderAll(leaflet, map);
    });
    resizeObserver.observe(wrapper);

    setTimeout(() => {
      if (!map || !leaflet) return;
      map.invalidateSize();
      useRevealAnimation = false;
      renderAll(leaflet, map);
    }, 100);

    useRevealAnimation = false;
    hasInitialized = true;
  });

  onDestroy(() => {
    for (const t of revealTimers) clearTimeout(t);
    revealTimers = [];
    resizeObserver?.disconnect();
    resizeObserver = null;
    document.removeEventListener('keydown', handleDocumentKeydown);

    if (map) {
      map.off();
      map.remove();
      map = null;
    }
  });

  function handlePopoverMemberHover(userId: string | null) {
    onMarkerHover?.(userId);
  }

  function handlePopoverMemberSelect(userId: string) {
    onMarkerHover?.(userId);
  }

  function handlePopoverClose() {
    closeOpenCluster();
    onMarkerHover?.(null);
  }

  function handlePopoverPointerLeave() {
    onMarkerHover?.(null);
  }
</script>

<div bind:this={wrapper} class="results-map-wrapper relative w-full h-full" role="presentation">
  <div bind:this={container} class="w-full h-full rounded-lg"></div>

  {#if popoverInfo}
    <ClusterPopover
      x={popoverInfo.x}
      y={popoverInfo.y}
      containerWidth={popoverInfo.containerWidth}
      containerHeight={popoverInfo.containerHeight}
      members={popoverInfo.members}
      onMemberHover={handlePopoverMemberHover}
      onMemberSelect={handlePopoverMemberSelect}
      onClose={handlePopoverClose}
      onPointerLeave={handlePopoverPointerLeave}
    />
  {/if}
</div>

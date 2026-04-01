<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';
  import { MAP_TILES, MAP_DEFAULTS, MARKER_CONFIG, RESULTS_ANIMATION } from '$lib/config/map';
  import { createMapPinIcon, createTargetIcon } from '$lib/components/map';
  import { formatDistance } from '$lib/utils';

  interface Guess {
    lat: number;
    lng: number;
    displayName: string;
    userId: string;
    distanceMeters: number;
  }

  interface Props {
    correctLat: number;
    correctLng: number;
    guesses: Guess[];
    /** Current user's ID to highlight their guess specially */
    currentUserId?: string;
    /** Enable staggered reveal animation (default: true) */
    animated?: boolean;
  }

  let { correctLat, correctLng, guesses, currentUserId = '', animated = true }: Props = $props();

  let container: HTMLDivElement;
  let map: L.Map | null = null;
  let leaflet: typeof L | null = null;

  /** Separate current user's guess from other players */
  function categorizeGuesses(allGuesses: Guess[]) {
    let currentUserGuess: Guess | null = null;
    const otherGuesses: Guess[] = [];

    for (const g of allGuesses) {
      // In solo mode, userId is '' and currentUserId is the user's ID or ''
      if (g.userId === currentUserId || (currentUserId === '' && g.userId === '')) {
        currentUserGuess = g;
      } else {
        otherGuesses.push(g);
      }
    }

    return { currentUserGuess, otherGuesses };
  }

  /** Format a label for a guess marker tooltip */
  function formatLabel(name: string, distanceMeters: number, isYou: boolean): string {
    const dist = formatDistance(distanceMeters);
    if (isYou) {
      return `<strong>You</strong> &middot; ${dist}`;
    }
    return `${name} &middot; ${dist}`;
  }

  /** Add a marker to the map with optional delay for animation */
  function addMarkerWithDelay(
    L: typeof import('leaflet'),
    mapInstance: L.Map,
    latlng: [number, number],
    icon: L.DivIcon,
    tooltipContent: string,
    tooltipClass: string,
    delay: number,
    permanent: boolean = true,
  ): L.Marker {
    const marker = L.marker(latlng, {
      icon,
      opacity: animated ? 0 : 1,
    }).addTo(mapInstance);

    marker.bindTooltip(tooltipContent, {
      permanent,
      direction: 'right',
      offset: [12, 0],
      className: `results-tooltip ${tooltipClass}`,
    });

    if (animated) {
      setTimeout(() => {
        marker.setOpacity(1);
        // Add animation class to the icon element
        const el = marker.getElement();
        if (el) {
          el.classList.add('marker-animated');
          el.style.animationDelay = '0ms';
        }
      }, delay);
    }

    return marker;
  }

  /** Add a dashed line from guess to correct location with optional delay */
  function addLineWithDelay(
    L: typeof import('leaflet'),
    mapInstance: L.Map,
    from: [number, number],
    to: [number, number],
    color: string,
    delay: number,
  ): L.Polyline {
    const line = L.polyline([from, to], {
      color,
      weight: 2.5,
      opacity: animated ? 0 : 0.7,
      dashArray: '8, 10',
    }).addTo(mapInstance);

    if (animated) {
      setTimeout(() => {
        line.setStyle({ opacity: 0.7 });
      }, delay);
    }

    return line;
  }

  onMount(async () => {
    if (!browser) return;

    leaflet = (await import('leaflet')).default;

    map = leaflet.map(container, {
      center: [correctLat, correctLng],
      zoom: MAP_DEFAULTS.resultZoom,
      zoomControl: false,
      attributionControl: false,
    });

    leaflet.tileLayer(MAP_TILES.url, {
      maxZoom: MAP_TILES.maxZoom,
    }).addTo(map);

    leaflet.control.zoom({ position: 'bottomright' }).addTo(map);

    // Build bounds to fit all markers
    const bounds = leaflet.latLngBounds([[correctLat, correctLng]]);
    const correctLatLng: [number, number] = [correctLat, correctLng];

    // Categorize guesses
    const { currentUserGuess, otherGuesses } = categorizeGuesses(guesses);

    // ─── 1. Correct Location (target icon) ───
    const correctIcon = createTargetIcon(leaflet, {
      color: MARKER_CONFIG.colors.correct,
      size: MARKER_CONFIG.resultSizes.correct,
      pulse: true,
    });

    addMarkerWithDelay(
      leaflet, map, correctLatLng, correctIcon,
      '<strong>Correct Location</strong>',
      'results-tooltip-correct',
      animated ? RESULTS_ANIMATION.correctDelay : 0,
    );

    // ─── 2. Current User's Guess (large pin with glow) ───
    if (currentUserGuess) {
      const userLatLng: [number, number] = [currentUserGuess.lat, currentUserGuess.lng];
      bounds.extend(userLatLng);

      const userIcon = createMapPinIcon(leaflet, {
        color: MARKER_CONFIG.colors.currentUser,
        size: MARKER_CONFIG.resultSizes.currentUser,
        glow: true,
      });

      const userDelay = animated ? RESULTS_ANIMATION.currentUserDelay : 0;

      addLineWithDelay(
        leaflet, map, userLatLng, correctLatLng,
        MARKER_CONFIG.colors.currentUser,
        userDelay,
      );

      addMarkerWithDelay(
        leaflet, map, userLatLng, userIcon,
        formatLabel(
          currentUserGuess.displayName || 'You',
          currentUserGuess.distanceMeters,
          true,
        ),
        'results-tooltip-you',
        userDelay + 100, // Slight offset so line appears just before pin
      );
    }

    // ─── 3. Other Players' Guesses (smaller pins, staggered) ───
    const playerColors = MARKER_CONFIG.colors.players;
    otherGuesses.forEach((guess, i) => {
      const color = playerColors[i % playerColors.length];
      const guessLatLng: [number, number] = [guess.lat, guess.lng];
      bounds.extend(guessLatLng);

      const icon = createMapPinIcon(leaflet!, {
        color,
        size: MARKER_CONFIG.resultSizes.otherPlayer,
      });

      const baseDelay = animated
        ? RESULTS_ANIMATION.currentUserDelay + RESULTS_ANIMATION.playerStagger * (i + 1)
        : 0;

      addLineWithDelay(
        leaflet!, map!, guessLatLng, correctLatLng,
        color,
        baseDelay,
      );

      addMarkerWithDelay(
        leaflet!, map!, guessLatLng, icon,
        formatLabel(guess.displayName, guess.distanceMeters, false),
        'results-tooltip',
        baseDelay + 100,
        // Only show permanent labels for ≤4 other players to avoid clutter
        otherGuesses.length <= 4,
      );
    });

    // Fit map to show all markers with padding
    if (guesses.length > 0) {
      map.fitBounds(bounds, { padding: MAP_DEFAULTS.padding });
    }

    // Invalidate size after a brief delay to fix initial rendering
    setTimeout(() => {
      map?.invalidateSize();
    }, 100);
  });

  onDestroy(() => {
    if (map) {
      map.remove();
      map = null;
    }
  });
</script>

<div bind:this={container} class="w-full h-full rounded-lg"></div>

<!-- Leaflet map styles and tooltip styles are in app.css -->

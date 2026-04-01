<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';
  import { MAP_TILES, MAP_DEFAULTS, MARKER_CONFIG } from '$lib/config/map';
  import { createMapPinIcon, createNumberedCircleIcon } from '$lib/components/map';
  import { formatDistance } from '$lib/utils';
  import type { RoundLocation, RoundResult } from '$lib/socket/game';

  interface Props {
    /** Correct locations for each round (parallel with roundHistory) */
    roundLocations: RoundLocation[];
    /** Results for each round */
    roundHistory: RoundResult[][];
    /** Current user's ID to find their guess in each round */
    currentUserId: string;
  }

  let { roundLocations, roundHistory, currentUserId }: Props = $props();

  let container: HTMLDivElement;
  let map: L.Map | null = null;
  let leaflet: typeof L | null = null;

  /** Find the current user's result from a round's results array */
  function findMyResult(results: RoundResult[]): RoundResult | null {
    return results.find(
      (r) => r.user_id === currentUserId || (currentUserId === '' && r.user_id === '')
    ) ?? null;
  }

  /** Get a score-based opacity: higher score = more opaque line */
  function scoreOpacity(score: number): number {
    // Max score is typically 5000. Map 0-5000 to 0.25-0.8
    return Math.max(0.25, Math.min(0.8, 0.25 + (score / 5000) * 0.55));
  }

  onMount(async () => {
    if (!browser) return;

    leaflet = (await import('leaflet')).default;

    map = leaflet.map(container, {
      center: MAP_DEFAULTS.center,
      zoom: MAP_DEFAULTS.zoom,
      zoomControl: false,
      attributionControl: false,
    });

    leaflet.tileLayer(MAP_TILES.url, {
      maxZoom: MAP_TILES.maxZoom,
    }).addTo(map);

    leaflet.control.zoom({ position: 'bottomright' }).addTo(map);

    const bounds = leaflet.latLngBounds([]);
    const correctColor = MARKER_CONFIG.colors.correct;
    const userColor = MARKER_CONFIG.colors.currentUser;

    for (let i = 0; i < roundLocations.length; i++) {
      const loc = roundLocations[i];
      const results = roundHistory[i] ?? [];
      const myResult = findMyResult(results);
      const correctLatLng: [number, number] = [loc.lat, loc.lng];
      bounds.extend(correctLatLng);

      // Correct location: numbered circle
      const correctIcon = createNumberedCircleIcon(leaflet, {
        number: i + 1,
        color: correctColor,
        size: 28,
      });

      const correctMarker = leaflet.marker(correctLatLng, { icon: correctIcon }).addTo(map);
      correctMarker.bindTooltip(`<strong>Round ${i + 1}</strong> &middot; Correct`, {
        permanent: false,
        direction: 'right',
        offset: [10, 0],
        className: 'summary-tooltip',
      });

      // User's guess for this round (if they guessed)
      if (myResult && myResult.distance_meters >= 0) {
        const guessLatLng: [number, number] = [myResult.guess_lat, myResult.guess_lng];
        bounds.extend(guessLatLng);

        const opacity = scoreOpacity(myResult.score);

        // Dashed line from guess to correct location
        leaflet.polyline([guessLatLng, correctLatLng], {
          color: userColor,
          weight: 2,
          opacity,
          dashArray: '6, 8',
        }).addTo(map);

        // Small guess pin
        const guessIcon = createMapPinIcon(leaflet, {
          color: userColor,
          size: 20,
          opacity: Math.max(0.5, opacity),
        });

        const guessMarker = leaflet.marker(guessLatLng, { icon: guessIcon }).addTo(map);
        guessMarker.bindTooltip(
          `<strong>Round ${i + 1}</strong> &middot; ${formatDistance(myResult.distance_meters)} &middot; ${myResult.score} pts`,
          {
            permanent: false,
            direction: 'right',
            offset: [8, 0],
            className: 'summary-tooltip',
          }
        );
      }
    }

    // Fit map to show all markers
    if (bounds.isValid()) {
      map.fitBounds(bounds, { padding: [40, 40] });
    }

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

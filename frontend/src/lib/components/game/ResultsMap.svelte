<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';
  import { MAP_TILES, MAP_DEFAULTS, MARKER_CONFIG } from '$lib/config/map';
  import { createMapPinIcon } from '$lib/components/map';

  interface Guess {
    lat: number;
    lng: number;
    displayName: string;
  }

  interface Props {
    correctLat: number;
    correctLng: number;
    guesses: Guess[];
  }

  let { correctLat, correctLng, guesses }: Props = $props();

  let container: HTMLDivElement;
  let map: L.Map | null = null;
  let leaflet: typeof L | null = null;

  onMount(async () => {
    if (!browser) return;

    // Dynamically import Leaflet only on client side
    leaflet = (await import('leaflet')).default;

    // MAP-006: Initialize map using centralized config
    map = leaflet.map(container, {
      center: [correctLat, correctLng],
      zoom: MAP_DEFAULTS.resultZoom,
      zoomControl: false,
      attributionControl: false,
    });

    // Add map tiles using centralized config
    leaflet.tileLayer(MAP_TILES.url, {
      maxZoom: MAP_TILES.maxZoom,
    }).addTo(map);

    // Add zoom control to bottom right
    leaflet.control.zoom({ position: 'bottomright' }).addTo(map);

    // Create bounds to fit all markers
    const bounds = leaflet.latLngBounds([[correctLat, correctLng]]);

    // Add correct location marker (red pin with pulse animation)
    const correctIcon = createMapPinIcon(leaflet, {
      color: MARKER_CONFIG.colors.correct,
      size: MARKER_CONFIG.size,
      pulse: true,
    });

    leaflet.marker([correctLat, correctLng], { icon: correctIcon })
      .bindTooltip('Correct Location', { 
        permanent: false, 
        direction: 'top',
        className: 'results-tooltip'
      })
      .addTo(map);

    // Add guess markers and lines
    guesses.forEach((guess, i) => {
      const color = MARKER_CONFIG.colors.players[i % MARKER_CONFIG.colors.players.length];

      // Create standardized pin icon for guess
      const guessIcon = createMapPinIcon(leaflet!, {
        color,
        size: MARKER_CONFIG.size,
      });

      // Add guess marker
      leaflet!.marker([guess.lat, guess.lng], { icon: guessIcon })
        .bindTooltip(guess.displayName || 'Your guess', { 
          permanent: false, 
          direction: 'top',
          className: 'results-tooltip'
        })
        .addTo(map!);

      // Draw line from guess to correct location
      leaflet!.polyline(
        [[guess.lat, guess.lng], [correctLat, correctLng]],
        {
          color: color,
          weight: 2,
          opacity: 0.7,
          dashArray: '6, 8',
        }
      ).addTo(map!);

      // Extend bounds to include this guess
      bounds.extend([guess.lat, guess.lng]);
    });

    // Fit map to show all markers with padding from centralized config
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

<!-- Leaflet map styles are in app.css -->

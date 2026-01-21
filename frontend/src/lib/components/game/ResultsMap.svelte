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

<style>
  /* Map pin styles */
  :global(.map-pin-icon) {
    background: transparent !important;
    border: none !important;
  }

  @keyframes map-pin-pulse {
    0% {
      transform: translateX(-50%) scale(0.8);
      opacity: 0.6;
    }
    100% {
      transform: translateX(-50%) scale(2);
      opacity: 0;
    }
  }

  @keyframes map-pin-bounce {
    0% {
      transform: translateY(-8px);
      opacity: 0;
    }
    60% {
      transform: translateY(2px);
    }
    100% {
      transform: translateY(0);
      opacity: 1;
    }
  }

  /* Tooltip styles */
  :global(.results-tooltip) {
    background: rgba(255, 255, 255, 0.95);
    backdrop-filter: blur(8px);
    border: 1px solid hsl(var(--border));
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 13px;
    font-weight: 500;
    color: hsl(var(--foreground));
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }

  :global(.results-tooltip::before) {
    border-top-color: rgba(255, 255, 255, 0.95) !important;
  }

  /* Zoom control styles */
  :global(.leaflet-control-zoom) {
    border: none !important;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12) !important;
    border-radius: 8px !important;
    overflow: hidden;
  }

  :global(.leaflet-control-zoom a) {
    background: rgba(255, 255, 255, 0.95) !important;
    backdrop-filter: blur(8px);
    color: hsl(var(--foreground)) !important;
    border: none !important;
    width: 32px !important;
    height: 32px !important;
    line-height: 32px !important;
    font-size: 16px !important;
    transition: background 0.15s ease;
  }

  :global(.leaflet-control-zoom a:hover) {
    background: rgba(255, 255, 255, 1) !important;
    color: hsl(var(--foreground)) !important;
  }

  :global(.leaflet-control-zoom-in) {
    border-radius: 8px 8px 0 0 !important;
    border-bottom: 1px solid hsl(var(--border)) !important;
  }

  :global(.leaflet-control-zoom-out) {
    border-radius: 0 0 8px 8px !important;
  }
</style>

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';
  import { MAP_TILES, MAP_DEFAULTS } from '$lib/config/map';

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

  // Player colors for guesses
  const colors = ['#22c55e', '#3b82f6', '#a855f7', '#f59e0b', '#ec4899', '#06b6d4'];

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

    // Add correct location marker (red circle with pin)
    const correctIcon = leaflet.divIcon({
      className: 'correct-marker',
      html: `
        <div class="marker-container">
          <div class="marker-circle correct"></div>
          <div class="marker-pulse"></div>
        </div>
      `,
      iconSize: [32, 32],
      iconAnchor: [16, 16],
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
      const color = colors[i % colors.length];

      // Create custom icon for guess
      const guessIcon = leaflet!.divIcon({
        className: 'guess-marker',
        html: `
          <div class="marker-container">
            <div class="marker-circle" style="background-color: ${color};"></div>
          </div>
        `,
        iconSize: [24, 24],
        iconAnchor: [12, 12],
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
  :global(.correct-marker),
  :global(.guess-marker) {
    background: transparent;
    border: none;
  }

  :global(.marker-container) {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  :global(.marker-circle) {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 3px solid white;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  :global(.marker-circle.correct) {
    width: 24px;
    height: 24px;
    background-color: #ef4444;
    border: 3px solid white;
  }

  :global(.marker-pulse) {
    position: absolute;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background-color: rgba(239, 68, 68, 0.3);
    animation: pulse 2s ease-out infinite;
  }

  @keyframes pulse {
    0% {
      transform: scale(0.5);
      opacity: 1;
    }
    100% {
      transform: scale(1.5);
      opacity: 0;
    }
  }

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

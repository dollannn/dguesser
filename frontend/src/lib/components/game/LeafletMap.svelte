<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';
  import { MAP_TILES, MAP_DEFAULTS, MARKER_CONFIG } from '$lib/config/map';
  import { createMapPinIcon, MAP_PIN_STYLES } from '$lib/components/map';

  interface Props {
    guessLat?: number | null;
    guessLng?: number | null;
    disabled?: boolean;
    expanded?: boolean;
    onclick?: (coords: { lat: number; lng: number }) => void;
  }

  let {
    guessLat = null,
    guessLng = null,
    disabled = false,
    expanded = false,
    onclick,
  }: Props = $props();

  let container: HTMLDivElement;
  let map: L.Map | null = null;
  let marker: L.Marker | null = null;
  let hoverMarker: L.Marker | null = null;
  let leaflet: typeof L | null = null;
  let resizeObserver: ResizeObserver | null = null;

  onMount(async () => {
    if (!browser) return;

    // Dynamically import Leaflet only on client side
    leaflet = (await import('leaflet')).default;

    // Create marker icons using standardized map pin
    const guessIcon = createMapPinIcon(leaflet, {
      color: MARKER_CONFIG.colors.guess,
      size: MARKER_CONFIG.size,
      bounce: true,
    });
    const hoverIcon = createMapPinIcon(leaflet, {
      color: MARKER_CONFIG.colors.guess,
      size: MARKER_CONFIG.size,
      opacity: MARKER_CONFIG.hoverOpacity,
    });

    // MAP-006: Initialize map with centralized config
    map = leaflet.map(container, {
      center: MAP_DEFAULTS.center,
      zoom: MAP_DEFAULTS.zoom,
      zoomControl: false,
      attributionControl: false,
    });

    // Add map tiles using centralized config
    leaflet.tileLayer(MAP_TILES.url, {
      maxZoom: MAP_TILES.maxZoom,
    }).addTo(map);

    // Add zoom control to bottom left
    leaflet.control.zoom({ position: 'bottomleft' }).addTo(map);

    // Use ResizeObserver for robust tile loading - handles initial sizing,
    // expand/collapse transitions, and window resizes
    resizeObserver = new ResizeObserver(() => {
      map?.invalidateSize();
    });
    resizeObserver.observe(container);

    // Handle clicks
    map.on('click', (e: L.LeafletMouseEvent) => {
      if (disabled) return;

      const { lat, lng } = e.latlng;
      
      // Hide hover marker once a guess is placed
      if (hoverMarker) {
        hoverMarker.remove();
        hoverMarker = null;
      }

      // Create or update marker immediately for instant feedback
      if (marker) {
        marker.setLatLng([lat, lng]);
      } else {
        marker = leaflet!.marker([lat, lng], { icon: guessIcon }).addTo(map!);
      }

      // Notify parent
      onclick?.({ lat, lng });
    });

    // Handle mouse move for hover preview
    map.on('mousemove', (e: L.LeafletMouseEvent) => {
      // Don't show hover if disabled or already have a placed marker
      if (disabled || marker !== null) return;

      const { lat, lng } = e.latlng;
      if (hoverMarker) {
        hoverMarker.setLatLng([lat, lng]);
      } else {
        hoverMarker = leaflet!.marker([lat, lng], { icon: hoverIcon, interactive: false }).addTo(map!);
      }
    });

    // Remove hover marker when mouse leaves the map
    map.on('mouseout', () => {
      if (hoverMarker) {
        hoverMarker.remove();
        hoverMarker = null;
      }
    });

    // Set initial marker if coords exist
    if (guessLat !== null && guessLng !== null) {
      marker = leaflet.marker([guessLat, guessLng], { icon: guessIcon }).addTo(map);
    }
  });

  onDestroy(() => {
    if (resizeObserver) {
      resizeObserver.disconnect();
      resizeObserver = null;
    }
    if (hoverMarker) {
      hoverMarker.remove();
      hoverMarker = null;
    }
    if (map) {
      map.remove();
      map = null;
    }
  });

  // Update marker when guess changes
  $effect(() => {
    if (!map || !leaflet) return;

    // Use standardized map pin for guess marker
    const guessIcon = createMapPinIcon(leaflet, {
      color: MARKER_CONFIG.colors.guess,
      size: MARKER_CONFIG.size,
      bounce: true,
    });

    if (guessLat !== null && guessLng !== null) {
      // Remove hover marker when placing a real guess
      if (hoverMarker) {
        hoverMarker.remove();
        hoverMarker = null;
      }

      if (marker) {
        marker.setLatLng([guessLat, guessLng]);
      } else {
        marker = leaflet.marker([guessLat, guessLng], { icon: guessIcon }).addTo(map);
      }
    } else if (marker) {
      marker.remove();
      marker = null;
    }
  });

  // Note: ResizeObserver handles map size invalidation automatically
  // when expanded state changes and triggers container resize
</script>

<div
  bind:this={container}
  class="w-full h-full rounded-lg"
  class:cursor-crosshair={!disabled}
  class:cursor-not-allowed={disabled}
></div>

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

  /* Zoom control styles */
  :global(.leaflet-control-zoom) {
    border: none !important;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15) !important;
  }

  :global(.leaflet-control-zoom a) {
    background: rgba(255, 255, 255, 0.95) !important;
    backdrop-filter: blur(8px);
    color: #374151 !important;
    border: none !important;
    width: 32px !important;
    height: 32px !important;
    line-height: 32px !important;
    font-size: 16px !important;
  }

  :global(.leaflet-control-zoom a:hover) {
    background: rgba(255, 255, 255, 1) !important;
    color: #111827 !important;
  }

  :global(.leaflet-control-zoom-in) {
    border-radius: 8px 8px 0 0 !important;
  }

  :global(.leaflet-control-zoom-out) {
    border-radius: 0 0 8px 8px !important;
  }
</style>

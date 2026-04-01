<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';
  import { MAP_TILES, MAP_DEFAULTS, MARKER_CONFIG } from '$lib/config/map';
  import { createMapPinIcon } from '$lib/components/map';

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

<!-- Leaflet map styles are in app.css -->

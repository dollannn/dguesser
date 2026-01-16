<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';
  import { MAP_TILES, MAP_DEFAULTS } from '$lib/config/map';

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

  // MAP-005: Extracted helper function to create marker icons
  function createMarkerIcon(L: typeof import('leaflet'), opacity: number = 1): L.DivIcon {
    const opacityStyle = opacity < 1 ? `opacity: ${opacity};` : '';
    return L.divIcon({
      className: opacity < 1 ? 'hover-marker' : 'guess-marker',
      html: `
        <div style="position: relative; ${opacityStyle}">
          <div style="width: 24px; height: 24px; background: #22c55e; border-radius: 50%; border: 3px solid white; box-shadow: 0 4px 6px rgba(0,0,0,0.3); display: flex; align-items: center; justify-content: center;">
            <div style="width: 8px; height: 8px; background: white; border-radius: 50%;"></div>
          </div>
          <div style="position: absolute; left: 50%; transform: translateX(-50%); top: 100%; width: 0; height: 0; border-left: 6px solid transparent; border-right: 6px solid transparent; border-top: 8px solid #22c55e; margin-top: -3px;"></div>
        </div>
      `,
      iconSize: [24, 32],
      iconAnchor: [12, 32],
    });
  }

  onMount(async () => {
    if (!browser) return;

    // Dynamically import Leaflet only on client side
    leaflet = (await import('leaflet')).default;

    // Create marker icons using helper function
    const guessIcon = createMarkerIcon(leaflet);
    const hoverIcon = createMarkerIcon(leaflet, 0.5);

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

    // Invalidate size after a brief delay to fix initial centering
    setTimeout(() => {
      map?.invalidateSize();
    }, 100);
  });

  onDestroy(() => {
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

    // MAP-005: Use shared helper function for icon creation
    const guessIcon = createMarkerIcon(leaflet);

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

  // Invalidate map size when expanded state changes
  $effect(() => {
    if (map && expanded !== undefined) {
      // Small delay to allow CSS transition to complete
      setTimeout(() => {
        map?.invalidateSize();
      }, 350);
    }
  });
</script>

<div
  bind:this={container}
  class="w-full h-full rounded-lg"
  class:cursor-crosshair={!disabled}
  class:cursor-not-allowed={disabled}
></div>

<style>
  :global(.guess-marker),
  :global(.hover-marker) {
    background: transparent;
    border: none;
  }

  :global(.hover-marker) {
    pointer-events: none;
  }

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

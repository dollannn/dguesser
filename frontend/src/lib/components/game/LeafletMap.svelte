<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import type L from 'leaflet';

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
  let leaflet: typeof L | null = null;

  onMount(async () => {
    if (!browser) return;

    // Dynamically import Leaflet only on client side
    leaflet = (await import('leaflet')).default;

    // Custom marker icon
    const guessIcon = leaflet.divIcon({
      className: 'guess-marker',
      html: `
        <div class="relative">
          <div class="w-6 h-6 bg-green-500 rounded-full border-3 border-white shadow-lg flex items-center justify-center">
            <div class="w-2 h-2 bg-white rounded-full"></div>
          </div>
          <div class="absolute left-1/2 -translate-x-1/2 top-full w-0 h-0 border-l-[6px] border-r-[6px] border-t-[8px] border-l-transparent border-r-transparent border-t-green-500 -mt-1"></div>
        </div>
      `,
      iconSize: [24, 32],
      iconAnchor: [12, 32],
    });

    // Initialize map with world view
    map = leaflet.map(container, {
      center: [20, 0],
      zoom: 2,
      zoomControl: false,
      attributionControl: false,
    });

    // Add OpenStreetMap tiles with a clean style
    leaflet.tileLayer('https://{s}.basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png', {
      maxZoom: 19,
    }).addTo(map);

    // Add zoom control to bottom left
    leaflet.control.zoom({ position: 'bottomleft' }).addTo(map);

    // Handle clicks
    map.on('click', (e: L.LeafletMouseEvent) => {
      if (disabled) return;

      const { lat, lng } = e.latlng;
      onclick?.({ lat, lng });
    });

    // Set initial marker if coords exist
    if (guessLat !== null && guessLng !== null) {
      marker = leaflet.marker([guessLat, guessLng], { icon: guessIcon }).addTo(map);
    }
  });

  onDestroy(() => {
    if (map) {
      map.remove();
      map = null;
    }
  });

  // Update marker when guess changes
  $effect(() => {
    if (!map || !leaflet) return;

    const guessIcon = leaflet.divIcon({
      className: 'guess-marker',
      html: `
        <div class="relative">
          <div class="w-6 h-6 bg-green-500 rounded-full border-3 border-white shadow-lg flex items-center justify-center">
            <div class="w-2 h-2 bg-white rounded-full"></div>
          </div>
          <div class="absolute left-1/2 -translate-x-1/2 top-full w-0 h-0 border-l-[6px] border-r-[6px] border-t-[8px] border-l-transparent border-r-transparent border-t-green-500 -mt-1"></div>
        </div>
      `,
      iconSize: [24, 32],
      iconAnchor: [12, 32],
    });

    if (guessLat !== null && guessLng !== null) {
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
  :global(.guess-marker) {
    background: transparent;
    border: none;
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

<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';

  interface Props {
    guessLat?: number | null;
    guessLng?: number | null;
    disabled?: boolean;
    onclick?: (coords: { lat: number; lng: number }) => void;
  }

  let { guessLat = null, guessLng = null, disabled = false, onclick }: Props = $props();

  let container: HTMLDivElement;
  let map: google.maps.Map | null = null;
  let marker: google.maps.Marker | null = null;

  onMount(() => {
    if (!browser) return;

    // Initialize map
    map = new google.maps.Map(container, {
      center: { lat: 20, lng: 0 },
      zoom: 2,
      disableDefaultUI: true,
      zoomControl: true,
      gestureHandling: 'greedy',
      styles: [
        {
          featureType: 'poi',
          stylers: [{ visibility: 'off' }],
        },
        {
          featureType: 'transit',
          stylers: [{ visibility: 'off' }],
        },
      ],
    });

    // Handle clicks
    map.addListener('click', (e: google.maps.MapMouseEvent) => {
      if (disabled || !e.latLng) return;

      const lat = e.latLng.lat();
      const lng = e.latLng.lng();

      onclick?.({ lat, lng });
    });
  });

  // Update marker when guess changes
  $effect(() => {
    if (map && guessLat !== null && guessLng !== null) {
      if (marker) {
        marker.setPosition({ lat: guessLat, lng: guessLng });
      } else {
        marker = new google.maps.Marker({
          position: { lat: guessLat, lng: guessLng },
          map,
          icon: {
            path: google.maps.SymbolPath.CIRCLE,
            scale: 10,
            fillColor: '#22c55e',
            fillOpacity: 1,
            strokeColor: '#ffffff',
            strokeWeight: 2,
          },
        });
      }
    }
  });
</script>

<div
  bind:this={container}
  class="w-full h-full"
  class:cursor-crosshair={!disabled}
  class:cursor-not-allowed={disabled}
></div>

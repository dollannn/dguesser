<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';

  interface Props {
    lat: number;
    lng: number;
    panoramaId?: string | null;
    movementAllowed?: boolean;
    zoomAllowed?: boolean;
  }

  let { lat, lng, panoramaId = null, movementAllowed = true, zoomAllowed = true }: Props =
    $props();

  let container: HTMLDivElement;
  let panorama: google.maps.StreetViewPanorama | null = null;

  onMount(() => {
    if (!browser) return;

    // Initialize Street View
    const position = { lat, lng };

    panorama = new google.maps.StreetViewPanorama(container, {
      position,
      pov: { heading: 0, pitch: 0 },
      zoom: 1,
      disableDefaultUI: true,
      showRoadLabels: false,
      linksControl: movementAllowed,
      panControl: true,
      zoomControl: zoomAllowed,
      addressControl: false,
      fullscreenControl: false,
      motionTracking: false,
      motionTrackingControl: false,
      clickToGo: movementAllowed,
      scrollwheel: zoomAllowed,
    });

    // If panorama ID provided, use it
    if (panoramaId) {
      panorama.setPano(panoramaId);
    }

    // Disable keyboard movement if not allowed
    if (!movementAllowed) {
      panorama.setOptions({
        clickToGo: false,
      });
    }
  });

  onDestroy(() => {
    panorama = null;
  });

  // React to location changes
  $effect(() => {
    if (panorama && lat && lng) {
      if (panoramaId) {
        panorama.setPano(panoramaId);
      } else {
        panorama.setPosition({ lat, lng });
      }
    }
  });
</script>

<div bind:this={container} class="w-full h-full bg-gray-900 street-view-container"></div>

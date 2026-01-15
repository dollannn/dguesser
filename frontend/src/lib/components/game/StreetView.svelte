<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import { loadGoogleMaps } from '$lib/maps/loader';

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
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    if (!browser) return;

    try {
      // Load Google Maps API first
      await loadGoogleMaps();

      console.log('[StreetView] Initializing with:', { lat, lng, panoramaId });

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

      // Track if we've tried fallback
      let triedFallback = false;

      // Listen for status changes to detect issues
      panorama.addListener('status_changed', () => {
        const status = panorama?.getStatus();
        console.log('[StreetView] Status changed:', status);
        if (status === google.maps.StreetViewStatus.ZERO_RESULTS && !triedFallback) {
          triedFallback = true;
          console.warn('[StreetView] Panorama ID failed, falling back to coordinates');
          // Fallback: use StreetViewService to find nearest panorama
          const sv = new google.maps.StreetViewService();
          sv.getPanorama({ location: { lat, lng }, radius: 1000 }, (data, svStatus) => {
            if (svStatus === google.maps.StreetViewStatus.OK && data?.location?.pano) {
              console.log('[StreetView] Found nearby panorama:', data.location.pano);
              panorama?.setPano(data.location.pano);
            } else {
              console.error('[StreetView] No Street View coverage within 1km');
            }
          });
        }
      });

      // If panorama ID provided, use it
      if (panoramaId) {
        console.log('[StreetView] Setting panorama ID:', panoramaId);
        panorama.setPano(panoramaId);
      } else {
        // No panorama ID - use StreetViewService to find one near coordinates
        console.log('[StreetView] No panorama ID, searching near coordinates');
        const sv = new google.maps.StreetViewService();
        sv.getPanorama({ location: { lat, lng }, radius: 1000 }, (data, svStatus) => {
          if (svStatus === google.maps.StreetViewStatus.OK && data?.location?.pano) {
            console.log('[StreetView] Found panorama:', data.location.pano);
            panorama?.setPano(data.location.pano);
          } else {
            console.error('[StreetView] No Street View coverage at coordinates');
          }
        });
      }

      // Disable keyboard movement if not allowed
      if (!movementAllowed) {
        panorama.setOptions({
          clickToGo: false,
        });
      }

      loading = false;
    } catch (e) {
      console.error('Failed to load Google Maps:', e);
      error = e instanceof Error ? e.message : 'Failed to load Street View';
      loading = false;
    }
  });

  onDestroy(() => {
    panorama = null;
  });

  // React to location changes
  $effect(() => {
    if (panorama && Number.isFinite(lat) && Number.isFinite(lng)) {
      if (panoramaId) {
        console.log('[StreetView] Location changed, setting panorama:', panoramaId);
        panorama.setPano(panoramaId);
      } else {
        console.log('[StreetView] Location changed, searching near:', { lat, lng });
        const sv = new google.maps.StreetViewService();
        sv.getPanorama({ location: { lat, lng }, radius: 1000 }, (data, status) => {
          if (status === google.maps.StreetViewStatus.OK && data?.location?.pano) {
            panorama?.setPano(data.location.pano);
          }
        });
      }
    }
  });
</script>

{#if error}
  <div class="w-full h-full min-h-screen bg-gray-900 flex items-center justify-center">
    <div class="text-center text-white">
      <svg class="w-16 h-16 mx-auto mb-4 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
      </svg>
      <p class="text-lg font-medium mb-2">Street View Error</p>
      <p class="text-sm text-gray-400">{error}</p>
    </div>
  </div>
{:else}
  <div 
    bind:this={container} 
    class="w-full h-full min-h-screen bg-gray-900 street-view-container"
    class:opacity-0={loading}
    class:opacity-100={!loading}
    style="transition: opacity 0.3s ease-in-out;"
  ></div>
{/if}

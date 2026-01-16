<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import { loadGoogleMaps } from '$lib/maps/loader';
  import { api } from '$lib/api/client';

  interface Props {
    lat: number;
    lng: number;
    panoramaId?: string | null;
    locationId?: string | null;
    movementAllowed?: boolean;
    zoomAllowed?: boolean;
    rotationAllowed?: boolean;
    showReportButton?: boolean;
  }

  let {
    lat,
    lng,
    panoramaId = null,
    locationId = null,
    movementAllowed = true,
    zoomAllowed = true,
    rotationAllowed = true,
    showReportButton = true,
  }: Props = $props();

  let container: HTMLDivElement;
  let panorama: google.maps.StreetViewPanorama | null = null;
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showReportMenu = $state(false);
  let reportSubmitting = $state(false);
  let reportSuccess = $state(false);

  type ReportReason = 'corrupted' | 'low_quality' | 'indoor' | 'restricted' | 'other';

  const reportReasons: { value: ReportReason; label: string; description: string }[] = [
    { value: 'corrupted', label: 'Corrupted imagery', description: 'Pink/blue/purple screen' },
    { value: 'low_quality', label: 'Low quality', description: 'Old camera, blurry, bad quality' },
    { value: 'indoor', label: 'Indoor location', description: 'Inside a building' },
    { value: 'restricted', label: 'Restricted/blurred', description: 'Blurred or restricted area' },
    { value: 'other', label: 'Other issue', description: 'Other problem with this location' },
  ];

  async function reportLocation(reason: ReportReason) {
    if (!locationId || reportSubmitting) return;

    reportSubmitting = true;
    try {
      await api.post(`/locations/${locationId}/report`, { reason });
      reportSuccess = true;
      showReportMenu = false;
      // Reset success message after 3 seconds
      setTimeout(() => {
        reportSuccess = false;
      }, 3000);
    } catch (e) {
      console.error('Failed to report location:', e);
    } finally {
      reportSubmitting = false;
    }
  }

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
        panControl: rotationAllowed,
        zoomControl: zoomAllowed,
        addressControl: false,
        fullscreenControl: false,
        motionTracking: false,
        motionTrackingControl: false,
        clickToGo: movementAllowed,
        scrollwheel: zoomAllowed,
        // Note: rotationAllowed mainly controls the pan control UI
        // Mouse/touch panning is harder to fully disable via API options
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
  <div class="relative w-full h-full min-h-screen">
    <div 
      bind:this={container} 
      class="w-full h-full min-h-screen bg-gray-900 street-view-container"
      class:opacity-0={loading}
      class:opacity-100={!loading}
      style="transition: opacity 0.3s ease-in-out;"
    ></div>

    <!-- Report Location Button -->
    {#if showReportButton && locationId && !loading}
      <div class="absolute bottom-4 left-4 z-10">
        <!-- Success Message -->
        {#if reportSuccess}
          <div class="bg-green-600 text-white px-3 py-2 rounded-lg text-sm shadow-lg">
            Report submitted. Thank you!
          </div>
        {:else}
          <!-- Report Button -->
          <div class="relative">
            <button
              onclick={() => (showReportMenu = !showReportMenu)}
              class="bg-gray-800/80 hover:bg-gray-700/90 text-white p-2 rounded-lg shadow-lg backdrop-blur-sm transition-colors"
              title="Report location issue"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 21v-4m0 0V5a2 2 0 012-2h6.5l1 1H21l-3 6 3 6h-8.5l-1-1H5a2 2 0 00-2 2zm9-13.5V9" />
              </svg>
            </button>

            <!-- Report Menu -->
            {#if showReportMenu}
              <div class="absolute bottom-12 left-0 bg-gray-800/95 rounded-lg shadow-xl backdrop-blur-sm min-w-[220px] overflow-hidden">
                <div class="px-3 py-2 border-b border-gray-700">
                  <p class="text-white text-sm font-medium">Report this location</p>
                </div>
                <div class="py-1">
                  {#each reportReasons as reason}
                    <button
                      onclick={() => reportLocation(reason.value)}
                      disabled={reportSubmitting}
                      class="w-full px-3 py-2 text-left hover:bg-gray-700/50 transition-colors disabled:opacity-50"
                    >
                      <p class="text-white text-sm">{reason.label}</p>
                      <p class="text-gray-400 text-xs">{reason.description}</p>
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { user } from '$lib/stores/auth';
  import { mapsApi, type MapDetails, type MapLocationItem } from '$lib/api/maps';
  import { ApiClientError } from '$lib/api/client';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import LockIcon from '@lucide/svelte/icons/lock';
  import LinkIcon from '@lucide/svelte/icons/link';
  import MapIcon from '@lucide/svelte/icons/map';
  import EditIcon from '@lucide/svelte/icons/pencil';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import PlayIcon from '@lucide/svelte/icons/play';
  import * as AlertDialog from '$lib/components/ui/alert-dialog';
  import { Button } from '$lib/components/ui/button';

  let map = $state<MapDetails | null>(null);
  let locations = $state<MapLocationItem[]>([]);
  let loading = $state(true);
  let loadingLocations = $state(false);
  let error = $state('');
  let deleting = $state(false);
  let showDeleteDialog = $state(false);
  let currentPage = $state(1);
  let totalLocations = $state(0);
  const perPage = 50;

  const mapId = $derived($page.params.id ?? '');
  const totalPages = $derived(Math.ceil(totalLocations / perPage));

  async function loadMap() {
    if (!mapId) return;
    loading = true;
    error = '';

    try {
      map = await mapsApi.get(mapId);
      totalLocations = map.location_count;
      await loadLocations();
    } catch (e) {
      if (e instanceof ApiClientError && e.status === 404) {
        error = 'Map not found';
      } else {
        error = e instanceof Error ? e.message : 'Failed to load map';
      }
    } finally {
      loading = false;
    }
  }

  async function loadLocations() {
    loadingLocations = true;
    try {
      const response = await mapsApi.getLocations(mapId, currentPage, perPage);
      locations = response.locations;
      totalLocations = response.total;
    } catch (e) {
      console.error('Failed to load locations:', e);
    } finally {
      loadingLocations = false;
    }
  }

  async function deleteMap() {
    if (!map) return;
    deleting = true;
    try {
      await mapsApi.delete(map.id);
      goto('/maps');
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to delete map';
      deleting = false;
      showDeleteDialog = false;
    }
  }

  function formatDate(isoString: string): string {
    const date = new Date(isoString);
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  }

  function getVisibilityIcon(visibility: string) {
    switch (visibility) {
      case 'public':
        return GlobeIcon;
      case 'unlisted':
        return LinkIcon;
      default:
        return LockIcon;
    }
  }

  function getVisibilityLabel(visibility: string): string {
    switch (visibility) {
      case 'public':
        return 'Public';
      case 'unlisted':
        return 'Unlisted';
      default:
        return 'Private';
    }
  }

  function getVisibilityDescription(visibility: string): string {
    switch (visibility) {
      case 'public':
        return 'Anyone can see and play this map';
      case 'unlisted':
        return 'Anyone with the link can access this map';
      default:
        return 'Only you can see and play this map';
    }
  }

  // Load map when component mounts or mapId changes
  $effect(() => {
    if (mapId) {
      loadMap();
    }
  });

  // Reload locations when page changes
  $effect(() => {
    if (map && currentPage > 0) {
      loadLocations();
    }
  });
</script>

<svelte:head>
  <title>{map?.name || 'Map'} - DGuesser</title>
</svelte:head>

<div class="max-w-4xl mx-auto px-4 py-8">
  <!-- Back link -->
  <a href="/maps" class="inline-flex items-center gap-1 text-gray-500 hover:text-gray-700 mb-6">
    <ChevronLeftIcon class="w-4 h-4" />
    Back to Maps
  </a>

  <!-- Error -->
  {#if error}
    <div
      class="mb-6 p-4 bg-red-50 border border-red-200 rounded-xl text-red-700 flex items-center gap-3"
    >
      <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>
      {error}
    </div>
  {/if}

  <!-- Loading -->
  {#if loading}
    <div class="bg-white rounded-xl shadow p-8 animate-pulse">
      <div class="flex items-start justify-between mb-6">
        <div class="flex items-center gap-4">
          <div class="w-14 h-14 bg-gray-200 rounded-xl"></div>
          <div>
            <div class="w-48 h-7 bg-gray-200 rounded mb-2"></div>
            <div class="w-32 h-4 bg-gray-200 rounded"></div>
          </div>
        </div>
        <div class="w-20 h-8 bg-gray-200 rounded-full"></div>
      </div>
      <div class="w-full h-16 bg-gray-200 rounded mb-6"></div>
      <div class="flex gap-4">
        <div class="w-32 h-10 bg-gray-200 rounded-lg"></div>
        <div class="w-32 h-10 bg-gray-200 rounded-lg"></div>
      </div>
    </div>
  {:else if map}
    <!-- Map details card -->
    <div class="bg-white rounded-xl shadow overflow-hidden mb-8">
      <div class="p-6">
        <!-- Header -->
        <div class="flex items-start justify-between mb-6">
          <div class="flex items-center gap-4">
            <div
              class="w-14 h-14 rounded-xl flex items-center justify-center
                     {map.is_system_map ? 'bg-blue-100 text-blue-600' : 'bg-purple-100 text-purple-600'}"
            >
              {#if map.is_system_map}
                <GlobeIcon class="w-7 h-7" />
              {:else}
                <MapIcon class="w-7 h-7" />
              {/if}
            </div>
            <div>
              <h1 class="text-2xl font-bold text-gray-900">{map.name}</h1>
              <p class="text-gray-500 text-sm">
                {map.is_system_map ? 'System Map' : 'Custom Map'} &middot; Created {formatDate(
                  map.created_at
                )}
              </p>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <svelte:component this={getVisibilityIcon(map.visibility)} class="w-4 h-4 text-gray-400" />
            <span class="text-sm text-gray-500">{getVisibilityLabel(map.visibility)}</span>
          </div>
        </div>

        <!-- Description -->
        {#if map.description}
          <p class="text-gray-600 mb-6">{map.description}</p>
        {:else}
          <p class="text-gray-400 italic mb-6">No description</p>
        {/if}

        <!-- Stats -->
        <div class="flex items-center gap-6 mb-6 text-sm">
          <div>
            <span class="text-gray-500">Locations:</span>
            <span class="font-semibold text-gray-900 ml-1"
              >{map.location_count.toLocaleString()}</span
            >
          </div>
          <div>
            <span class="text-gray-500">Last updated:</span>
            <span class="font-semibold text-gray-900 ml-1">{formatDate(map.updated_at)}</span>
          </div>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-3">
          {#if map.location_count > 0}
            <a
              href="/play?map={map.id}"
              class="inline-flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-lg
                     font-medium hover:bg-green-700 transition-colors"
            >
              <PlayIcon class="w-5 h-5" />
              Play This Map
            </a>
          {/if}

          {#if map.is_owned}
            <a
              href="/maps/{map.id}/edit"
              class="inline-flex items-center gap-2 px-4 py-2 bg-gray-100 text-gray-700 rounded-lg
                     font-medium hover:bg-gray-200 transition-colors"
            >
              <EditIcon class="w-4 h-4" />
              Edit Map
            </a>

            <AlertDialog.Root bind:open={showDeleteDialog}>
              <AlertDialog.Trigger>
                <Button variant="outline" class="text-red-600 hover:text-red-700 hover:bg-red-50">
                  <TrashIcon class="w-4 h-4 mr-2" />
                  Delete
                </Button>
              </AlertDialog.Trigger>
              <AlertDialog.Content>
                <AlertDialog.Header>
                  <AlertDialog.Title>Delete Map</AlertDialog.Title>
                  <AlertDialog.Description>
                    Are you sure you want to delete "{map.name}"? This action cannot be undone.
                  </AlertDialog.Description>
                </AlertDialog.Header>
                <AlertDialog.Footer>
                  <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
                  <AlertDialog.Action
                    onclick={deleteMap}
                    class="bg-red-600 hover:bg-red-700 text-white"
                  >
                    {deleting ? 'Deleting...' : 'Delete Map'}
                  </AlertDialog.Action>
                </AlertDialog.Footer>
              </AlertDialog.Content>
            </AlertDialog.Root>
          {/if}
        </div>
      </div>

      <!-- Visibility info bar -->
      <div class="px-6 py-3 bg-gray-50 border-t border-gray-100">
        <p class="text-sm text-gray-500 flex items-center gap-2">
          <svelte:component this={getVisibilityIcon(map.visibility)} class="w-4 h-4" />
          {getVisibilityDescription(map.visibility)}
        </p>
      </div>
    </div>

    <!-- Locations section -->
    {#if map.location_count > 0}
      <div class="bg-white rounded-xl shadow overflow-hidden">
        <div class="p-4 border-b border-gray-100">
          <h2 class="font-semibold text-gray-900">Locations ({totalLocations.toLocaleString()})</h2>
        </div>

        {#if loadingLocations}
          <div class="p-8 text-center">
            <div class="animate-spin w-8 h-8 border-2 border-gray-300 border-t-gray-600 rounded-full mx-auto"></div>
          </div>
        {:else}
          <div class="divide-y divide-gray-100">
            {#each locations as location (location.id)}
              <div class="px-4 py-3 flex items-center justify-between text-sm">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 bg-gray-100 rounded-lg flex items-center justify-center text-gray-400">
                    <MapIcon class="w-4 h-4" />
                  </div>
                  <div>
                    <span class="text-gray-900">
                      {location.lat.toFixed(4)}, {location.lng.toFixed(4)}
                    </span>
                    {#if location.country_code}
                      <span class="text-gray-400 ml-2">({location.country_code})</span>
                    {/if}
                  </div>
                </div>
                <span class="text-gray-400 font-mono text-xs">{location.id}</span>
              </div>
            {/each}
          </div>

          <!-- Pagination -->
          {#if totalPages > 1}
            <div class="p-4 border-t border-gray-100 flex items-center justify-between">
              <button
                onclick={() => (currentPage = Math.max(1, currentPage - 1))}
                disabled={currentPage === 1}
                class="inline-flex items-center gap-1 px-3 py-1.5 text-sm text-gray-600
                       hover:text-gray-900 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <ChevronLeftIcon class="w-4 h-4" />
                Previous
              </button>
              <span class="text-sm text-gray-500">
                Page {currentPage} of {totalPages}
              </span>
              <button
                onclick={() => (currentPage = Math.min(totalPages, currentPage + 1))}
                disabled={currentPage === totalPages}
                class="inline-flex items-center gap-1 px-3 py-1.5 text-sm text-gray-600
                       hover:text-gray-900 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Next
                <ChevronRightIcon class="w-4 h-4" />
              </button>
            </div>
          {/if}
        {/if}
      </div>
    {:else}
      <div class="bg-white rounded-xl shadow p-8 text-center">
        <MapIcon class="w-12 h-12 mx-auto text-gray-300 mb-3" />
        <p class="text-gray-500">This map has no locations yet</p>
        {#if map.is_owned}
          <a
            href="/maps/{map.id}/edit"
            class="inline-flex items-center gap-2 mt-4 text-blue-600 hover:text-blue-700"
          >
            <EditIcon class="w-4 h-4" />
            Add locations
          </a>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<script lang="ts">
  import { goto } from '$app/navigation';
  import { user } from '$lib/stores/auth';
  import { mapsApi, type MapSummary } from '$lib/api/maps';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import MapIcon from '@lucide/svelte/icons/map';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import LockIcon from '@lucide/svelte/icons/lock';
  import LinkIcon from '@lucide/svelte/icons/link';
  import UsersIcon from '@lucide/svelte/icons/users';
  import EditIcon from '@lucide/svelte/icons/pencil';

  type FilterType = 'all' | 'mine' | 'public';

  let maps = $state<MapSummary[]>([]);
  let loading = $state(true);
  let error = $state('');
  let filter = $state<FilterType>('all');

  const filterOptions: { value: FilterType; label: string }[] = [
    { value: 'all', label: 'All Maps' },
    { value: 'mine', label: 'My Maps' },
    { value: 'public', label: 'Public' },
  ];

  // Filter maps based on selection
  let filteredMaps = $derived(() => {
    switch (filter) {
      case 'mine':
        return maps.filter((m) => m.is_owned);
      case 'public':
        return maps.filter((m) => m.visibility === 'public' && !m.is_owned);
      default:
        return maps;
    }
  });

  async function loadMaps() {
    loading = true;
    error = '';

    try {
      const response = await mapsApi.list();
      maps = response.maps;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load maps';
    } finally {
      loading = false;
    }
  }

  function formatDate(isoString: string): string {
    const date = new Date(isoString);
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
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

  function getVisibilityClass(visibility: string): string {
    switch (visibility) {
      case 'public':
        return 'bg-green-100 text-green-700';
      case 'unlisted':
        return 'bg-yellow-100 text-yellow-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  }

  // Load maps on mount
  $effect(() => {
    loadMaps();
  });
</script>

<svelte:head>
  <title>Maps - DGuesser</title>
</svelte:head>

<div class="max-w-6xl mx-auto px-4 py-8">
  <!-- Header -->
  <div class="flex items-center justify-between mb-8">
    <div>
      <h1 class="text-4xl font-bold text-gray-900 mb-2">Maps</h1>
      <p class="text-gray-600">Browse maps or create your own custom maps</p>
    </div>
    {#if $user}
      <a
        href="/maps/new"
        class="inline-flex items-center gap-2 px-4 py-2 bg-gray-900 text-white rounded-lg
               font-medium hover:bg-gray-800 transition-colors shadow-md"
      >
        <PlusIcon class="w-5 h-5" />
        Create Map
      </a>
    {/if}
  </div>

  <!-- Filter tabs -->
  <div class="flex flex-wrap gap-2 mb-6">
    {#each filterOptions as option}
      <button
        onclick={() => (filter = option.value)}
        class="px-4 py-2 rounded-lg text-sm font-medium transition-all {filter === option.value
          ? 'bg-gray-900 text-white shadow-md'
          : 'bg-gray-100 text-gray-700 hover:bg-gray-200'}"
      >
        {option.label}
      </button>
    {/each}
  </div>

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

  <!-- Loading skeleton -->
  {#if loading}
    <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {#each Array(6) as _}
        <div class="bg-white rounded-xl shadow p-6 animate-pulse">
          <div class="flex items-start justify-between mb-4">
            <div class="w-10 h-10 bg-gray-200 rounded-lg"></div>
            <div class="w-16 h-6 bg-gray-200 rounded-full"></div>
          </div>
          <div class="w-3/4 h-5 bg-gray-200 rounded mb-2"></div>
          <div class="w-1/2 h-4 bg-gray-200 rounded mb-4"></div>
          <div class="flex items-center justify-between">
            <div class="w-20 h-4 bg-gray-200 rounded"></div>
            <div class="w-24 h-4 bg-gray-200 rounded"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else if filteredMaps().length === 0}
    <!-- Empty state -->
    <div class="text-center py-16 bg-white rounded-xl shadow">
      <MapIcon class="w-16 h-16 mx-auto text-gray-300 mb-4" />
      {#if filter === 'mine'}
        <p class="text-lg text-gray-700 font-medium">You haven't created any maps yet</p>
        <p class="text-sm text-gray-500 mt-2">Create your first custom map to get started!</p>
        {#if $user}
          <a href="/maps/new" class="btn-primary mt-6 inline-flex items-center gap-2">
            <PlusIcon class="w-5 h-5" />
            Create Map
          </a>
        {/if}
      {:else}
        <p class="text-lg text-gray-700 font-medium">No maps found</p>
        <p class="text-sm text-gray-500 mt-2">Try a different filter</p>
        <button onclick={() => (filter = 'all')} class="btn-secondary mt-6"> Show All Maps </button>
      {/if}
    </div>
  {:else}
    <!-- Maps grid -->
    <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {#each filteredMaps() as map (map.id)}
        <a
          href="/maps/{map.id}"
          class="bg-white rounded-xl shadow p-6 hover:shadow-lg transition-shadow group"
        >
          <div class="flex items-start justify-between mb-4">
            <div
              class="w-10 h-10 rounded-lg flex items-center justify-center
                     {map.is_system_map ? 'bg-blue-100 text-blue-600' : 'bg-purple-100 text-purple-600'}"
            >
              {#if map.is_system_map}
                <GlobeIcon class="w-5 h-5" />
              {:else}
                <MapIcon class="w-5 h-5" />
              {/if}
            </div>
            <span
              class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-medium {getVisibilityClass(
                map.visibility
              )}"
            >
              <svelte:component this={getVisibilityIcon(map.visibility)} class="w-3 h-3" />
              {getVisibilityLabel(map.visibility)}
            </span>
          </div>

          <h3
            class="text-lg font-semibold text-gray-900 mb-1 group-hover:text-blue-600 transition-colors"
          >
            {map.name}
          </h3>

          {#if map.description}
            <p class="text-sm text-gray-500 mb-4 line-clamp-2">{map.description}</p>
          {:else}
            <p class="text-sm text-gray-400 mb-4 italic">No description</p>
          {/if}

          <div class="flex items-center justify-between text-sm">
            <span class="text-gray-500">
              {map.location_count.toLocaleString()} locations
            </span>
            <span class="text-gray-400">
              {formatDate(map.created_at)}
            </span>
          </div>

          {#if map.is_owned}
            <div class="mt-4 pt-4 border-t border-gray-100 flex items-center justify-between">
              <span class="text-xs text-purple-600 font-medium flex items-center gap-1">
                <UsersIcon class="w-3 h-3" />
                Your Map
              </span>
              <button
                onclick={(e) => {
                  e.preventDefault();
                  goto(`/maps/${map.id}/edit`);
                }}
                class="text-xs text-gray-500 hover:text-gray-700 flex items-center gap-1"
              >
                <EditIcon class="w-3 h-3" />
                Edit
              </button>
            </div>
          {/if}
        </a>
      {/each}
    </div>

    <!-- Stats footer -->
    <div class="mt-6 text-center">
      <p class="text-sm text-gray-500">
        Showing {filteredMaps().length}
        {filteredMaps().length === 1 ? 'map' : 'maps'}
        {#if filter !== 'all'}
          ({maps.length} total)
        {/if}
      </p>
    </div>
  {/if}
</div>

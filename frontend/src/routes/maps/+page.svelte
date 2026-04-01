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
  import SEO from '$lib/components/SEO.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  type FilterType = 'all' | 'mine' | 'public';

  // Initialize from server data
  let maps = $state<MapSummary[]>(data.maps ?? []);
  let loading = $state(false);
  let error = $state(data.error || '');
  let filter = $state<FilterType>('all');

  const filterOptions: { value: FilterType; label: string }[] = [
    { value: 'all', label: 'All Maps' },
    { value: 'mine', label: 'My Maps' },
    { value: 'public', label: 'Public' },
  ];

  // Filter maps based on selection
  let filteredMaps = $derived.by(() => {
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
        return 'bg-muted text-foreground';
    }
  }

</script>

<SEO
  title="Maps"
  description="Browse custom geography maps created by the community. Play maps focused on specific countries, cities, landmarks, or themes."
/>

<div class="max-w-6xl mx-auto px-4 py-8">
  <!-- Header -->
  <div class="flex items-center justify-between mb-8">
    <div>
      <h1 class="text-4xl font-bold text-foreground mb-2">Maps</h1>
      <p class="text-muted-foreground">Browse maps or create your own custom maps</p>
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
          : 'bg-muted text-foreground hover:bg-gray-200'}"
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
        <div class="bg-card rounded-xl shadow p-6 animate-pulse">
          <div class="flex items-start justify-between mb-4">
            <div class="w-10 h-10 bg-border rounded-lg"></div>
            <div class="w-16 h-6 bg-border rounded-full"></div>
          </div>
          <div class="w-3/4 h-5 bg-border rounded mb-2"></div>
          <div class="w-1/2 h-4 bg-border rounded mb-4"></div>
          <div class="flex items-center justify-between">
            <div class="w-20 h-4 bg-border rounded"></div>
            <div class="w-24 h-4 bg-border rounded"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else if filteredMaps.length === 0}
    <!-- Empty state -->
    <div class="text-center py-16 bg-card rounded-xl shadow">
      <MapIcon class="w-16 h-16 mx-auto text-muted-foreground/50 mb-4" />
      {#if filter === 'mine'}
        <p class="text-lg text-foreground font-medium">You haven't created any maps yet</p>
        <p class="text-sm text-muted-foreground mt-2">Create your first custom map to get started!</p>
        {#if $user}
          <a href="/maps/new" class="mt-6 inline-flex items-center gap-2 px-4 py-2 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 transition-colors">
            <PlusIcon class="w-5 h-5" />
            Create Map
          </a>
        {/if}
      {:else}
        <p class="text-lg text-foreground font-medium">No maps found</p>
        <p class="text-sm text-muted-foreground mt-2">Try a different filter</p>
        <button onclick={() => (filter = 'all')} class="mt-6 px-4 py-2 rounded-md border border-border bg-background text-sm font-medium hover:bg-muted transition-colors"> Show All Maps </button>
      {/if}
    </div>
  {:else}
    <!-- Maps grid -->
    <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {#each filteredMaps as map (map.id)}
        <a
          href="/maps/{map.id}"
          class="bg-card rounded-xl shadow p-6 hover:shadow-lg transition-shadow group"
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
              {#if map.visibility === 'public'}
                <GlobeIcon class="w-3 h-3" />
              {:else if map.visibility === 'unlisted'}
                <LinkIcon class="w-3 h-3" />
              {:else}
                <LockIcon class="w-3 h-3" />
              {/if}
              {getVisibilityLabel(map.visibility)}
            </span>
          </div>

          <h3
            class="text-lg font-semibold text-foreground mb-1 group-hover:text-primary transition-colors"
          >
            {map.name}
          </h3>

          {#if map.description}
            <p class="text-sm text-muted-foreground mb-4 line-clamp-2">{map.description}</p>
          {:else}
            <p class="text-sm text-muted-foreground mb-4 italic">No description</p>
          {/if}

          <div class="flex items-center justify-between text-sm">
            <span class="text-muted-foreground">
              {map.location_count.toLocaleString()} locations
            </span>
            <span class="text-muted-foreground">
              {formatDate(map.created_at)}
            </span>
          </div>

          {#if map.is_owned}
            <div class="mt-4 pt-4 border-t border-border flex items-center justify-between">
              <span class="text-xs text-purple-600 font-medium flex items-center gap-1">
                <UsersIcon class="w-3 h-3" />
                Your Map
              </span>
              <button
                onclick={(e) => {
                  e.preventDefault();
                  goto(`/maps/${map.id}/edit`);
                }}
                class="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"
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
      <p class="text-sm text-muted-foreground">
        Showing {filteredMaps.length}
        {filteredMaps.length === 1 ? 'map' : 'maps'}
        {#if filter !== 'all'}
          ({maps.length} total)
        {/if}
      </p>
    </div>
  {/if}
</div>

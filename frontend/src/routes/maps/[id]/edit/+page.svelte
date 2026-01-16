<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { user } from '$lib/stores/auth';
  import {
    mapsApi,
    locationsApi,
    type MapDetails,
    type MapVisibility,
    type LocationSearchItem,
    type CountryInfo,
    type MapLocationItem,
  } from '$lib/api/maps';
  import { ApiClientError } from '$lib/api/client';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as Tabs from '$lib/components/ui/tabs';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import SearchIcon from '@lucide/svelte/icons/search';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import LinkIcon from '@lucide/svelte/icons/link';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import LockIcon from '@lucide/svelte/icons/lock';
  import MapIcon from '@lucide/svelte/icons/map';
  import SaveIcon from '@lucide/svelte/icons/save';
  import CheckIcon from '@lucide/svelte/icons/check';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import SEO from '$lib/components/SEO.svelte';

  const mapId = $derived($page.params.id ?? '');

  // Map state
  let map = $state<MapDetails | null>(null);
  let loading = $state(true);
  let error = $state('');

  // Edit form state
  let name = $state('');
  let description = $state('');
  let visibility = $state<MapVisibility>('private');
  let saving = $state(false);
  let saved = $state(false);
  let nameError = $state('');

  // Search state
  let countries = $state<CountryInfo[]>([]);
  let selectedCountry = $state<string>('');
  let searchResults = $state<LocationSearchItem[]>([]);
  let searchTotal = $state(0);
  let searching = $state(false);
  let countriesLoading = $state(true);

  // Selected locations
  let selectedLocations = $state<MapLocationItem[]>([]);
  let locationsPage = $state(1);

  // URL import state
  let urlInput = $state('');
  let urlResults = $state<Array<{ url: string; success: boolean; error?: string }>>([]);
  let importingUrls = $state(false);

  const visibilityOptions = [
    { value: 'private', label: 'Private', icon: LockIcon, desc: 'Only you can see' },
    { value: 'unlisted', label: 'Unlisted', icon: LinkIcon, desc: 'Anyone with link' },
    { value: 'public', label: 'Public', icon: GlobeIcon, desc: 'Everyone can see' },
  ];

  // Validate name
  function validateName(value: string): string {
    if (!value.trim()) return 'Name is required';
    if (value.trim().length < 3) return 'Name must be at least 3 characters';
    if (value.length > 100) return 'Name must be at most 100 characters';
    return '';
  }

  // Load map data
  async function loadMap() {
    if (!mapId) return;
    loading = true;
    error = '';

    try {
      map = await mapsApi.get(mapId);

      // Check ownership
      if (!map.is_owned) {
        error = 'You can only edit your own maps';
        return;
      }

      // Set form values
      name = map.name;
      description = map.description || '';
      visibility = map.visibility;

      // Load locations
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

  // Load locations
  async function loadLocations() {
    if (!mapId) return;
    try {
      const response = await mapsApi.getLocations(mapId, locationsPage, 50);
      selectedLocations = response.locations;
    } catch (e) {
      console.error('Failed to load locations:', e);
    }
  }

  // Load countries
  async function loadCountries() {
    countriesLoading = true;
    try {
      const response = await locationsApi.getCountries();
      countries = response.countries;
    } catch (e) {
      console.error('Failed to load countries:', e);
    } finally {
      countriesLoading = false;
    }
  }

  // Search locations
  async function searchLocations() {
    if (!selectedCountry || !mapId) return;

    searching = true;
    try {
      const response = await locationsApi.search({
        country_code: selectedCountry || undefined,
        exclude_map_id: mapId,
        page: 1,
        per_page: 20,
      });
      searchResults = response.locations;
      searchTotal = response.total;
    } catch (e) {
      console.error('Failed to search locations:', e);
    } finally {
      searching = false;
    }
  }

  // Save map details
  async function saveMap() {
    nameError = validateName(name);
    if (nameError) return;

    saving = true;
    saved = false;

    try {
      await mapsApi.update(mapId, {
        name: name.trim(),
        description: description.trim() || undefined,
        visibility,
      });
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to save';
    } finally {
      saving = false;
    }
  }

  // Add location to map
  async function addLocation(location: LocationSearchItem) {
    if (!mapId) return;

    try {
      await mapsApi.addLocations(mapId, [location.id]);
      selectedLocations = [
        ...selectedLocations,
        {
          id: location.id,
          panorama_id: location.panorama_id,
          lat: location.lat,
          lng: location.lng,
          country_code: location.country_code,
          subdivision_code: location.subdivision_code,
        },
      ];
      searchResults = searchResults.filter((l) => l.id !== location.id);
      if (map) map.location_count++;
    } catch (e) {
      console.error('Failed to add location:', e);
    }
  }

  // Remove location from map
  async function removeLocation(locationId: string) {
    if (!mapId) return;

    try {
      await mapsApi.removeLocation(mapId, locationId);
      selectedLocations = selectedLocations.filter((l) => l.id !== locationId);
      if (map) map.location_count--;
    } catch (e) {
      console.error('Failed to remove location:', e);
    }
  }

  // Import URLs
  async function importUrls() {
    if (!mapId || !urlInput.trim()) return;

    const urls = urlInput
      .split('\n')
      .map((u) => u.trim())
      .filter((u) => u.length > 0);

    if (urls.length === 0) return;

    importingUrls = true;
    urlResults = [];

    try {
      const response = await mapsApi.addLocationsFromUrls(mapId, urls);
      urlResults = response.results.map((r) => ({
        url: r.url,
        success: r.success,
        error: r.error ?? undefined,
      }));

      // Reload locations
      await loadLocations();
      if (map) {
        const mapData = await mapsApi.get(mapId);
        map.location_count = mapData.location_count;
      }
    } catch (e) {
      console.error('Failed to import URLs:', e);
    } finally {
      importingUrls = false;
    }
  }

  // Load data
  $effect(() => {
    if ($user && mapId) {
      loadMap();
      loadCountries();
    }
  });

  // Search when country changes
  $effect(() => {
    if (selectedCountry && mapId) {
      searchLocations();
    }
  });
</script>

<SEO title="Edit {map?.name || 'Map'}" noindex />

<div class="max-w-5xl mx-auto px-4 py-8">
  <!-- Back link -->
  <a
    href="/maps/{mapId}"
    class="inline-flex items-center gap-1 text-gray-500 hover:text-gray-700 mb-6"
  >
    <ChevronLeftIcon class="w-4 h-4" />
    Back to Map
  </a>

  {#if loading}
    <div class="bg-white rounded-xl shadow p-8 animate-pulse">
      <div class="w-48 h-8 bg-gray-200 rounded mb-6"></div>
      <div class="space-y-4">
        <div class="w-full h-10 bg-gray-200 rounded"></div>
        <div class="w-full h-20 bg-gray-200 rounded"></div>
        <div class="w-full h-10 bg-gray-200 rounded"></div>
      </div>
    </div>
  {:else if error}
    <div class="bg-red-50 border border-red-200 rounded-xl p-6 text-red-700">
      {error}
      <a href="/maps" class="block mt-4 text-red-600 underline">Go back to maps</a>
    </div>
  {:else if map}
    <div class="space-y-6">
      <!-- Header -->
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-bold text-gray-900">Edit Map</h1>
        <Button onclick={() => goto(`/maps/${mapId}`)} variant="outline">
          View Map
        </Button>
      </div>

      <!-- Map details form -->
      <div class="bg-white rounded-xl shadow p-6">
        <h2 class="text-lg font-semibold text-gray-900 mb-4">Map Details</h2>

        <div class="space-y-4">
          <!-- Name -->
          <div>
            <Label for="name">Name</Label>
            <Input
              id="name"
              bind:value={name}
              class="mt-1.5"
              oninput={() => (nameError = '')}
            />
            {#if nameError}
              <p class="text-red-500 text-sm mt-1">{nameError}</p>
            {/if}
          </div>

          <!-- Description -->
          <div>
            <Label for="description">Description</Label>
            <textarea
              id="description"
              bind:value={description}
              rows="2"
              class="mt-1.5 w-full px-3 py-2 border border-gray-300 rounded-lg text-sm
                     focus:outline-none focus:ring-2 focus:ring-gray-900"
            ></textarea>
          </div>

          <!-- Visibility -->
          <div>
            <Label>Visibility</Label>
            <div class="mt-1.5 flex gap-2">
              {#each visibilityOptions as option}
                <button
                  type="button"
                  onclick={() => (visibility = option.value as MapVisibility)}
                  class="flex items-center gap-2 px-3 py-2 rounded-lg border text-sm transition-all
                         {visibility === option.value
                    ? 'border-gray-900 bg-gray-50'
                    : 'border-gray-200 hover:border-gray-300'}"
                >
                  <svelte:component
                    this={option.icon}
                    class="w-4 h-4 {visibility === option.value ? 'text-gray-900' : 'text-gray-400'}"
                  />
                  {option.label}
                </button>
              {/each}
            </div>
          </div>

          <!-- Save button -->
          <div class="flex items-center gap-3 pt-2">
            <Button onclick={saveMap} disabled={saving}>
              <SaveIcon class="w-4 h-4 mr-2" />
              {saving ? 'Saving...' : 'Save Changes'}
            </Button>
            {#if saved}
              <span class="text-green-600 text-sm flex items-center gap-1">
                <CheckIcon class="w-4 h-4" />
                Saved!
              </span>
            {/if}
          </div>
        </div>
      </div>

      <!-- Location builder -->
      <div class="grid gap-6 lg:grid-cols-2">
        <!-- Left: Search and add -->
        <div class="bg-white rounded-xl shadow overflow-hidden">
          <Tabs.Root value="search" class="w-full">
            <Tabs.List class="w-full border-b">
              <Tabs.Trigger value="search" class="flex-1">
                <SearchIcon class="w-4 h-4 mr-2" />
                Search
              </Tabs.Trigger>
              <Tabs.Trigger value="urls" class="flex-1">
                <LinkIcon class="w-4 h-4 mr-2" />
                Import URLs
              </Tabs.Trigger>
            </Tabs.List>

            <Tabs.Content value="search" class="p-4">
              <div class="mb-4">
                <Label for="country">Filter by Country</Label>
                <select
                  id="country"
                  bind:value={selectedCountry}
                  class="mt-1.5 w-full px-3 py-2 border border-gray-300 rounded-lg text-sm"
                >
                  <option value="">Select a country...</option>
                  {#each countries as country}
                    <option value={country.code}>
                      {country.code} ({country.count.toLocaleString()})
                    </option>
                  {/each}
                </select>
              </div>

              {#if searching}
                <div class="py-8 text-center">
                  <div class="animate-spin w-6 h-6 border-2 border-gray-300 border-t-gray-600 rounded-full mx-auto"></div>
                </div>
              {:else if searchResults.length > 0}
                <div class="space-y-2 max-h-64 overflow-y-auto">
                  {#each searchResults as location (location.id)}
                    <div class="flex items-center justify-between p-2 rounded-lg hover:bg-gray-50 text-sm">
                      <span class="text-gray-700">
                        {location.lat.toFixed(4)}, {location.lng.toFixed(4)}
                      </span>
                      <Button variant="ghost" size="icon-sm" onclick={() => addLocation(location)}>
                        <PlusIcon class="w-4 h-4" />
                      </Button>
                    </div>
                  {/each}
                </div>
              {:else if selectedCountry}
                <p class="text-gray-500 text-sm text-center py-8">No locations found</p>
              {:else}
                <p class="text-gray-500 text-sm text-center py-8">Select a country</p>
              {/if}
            </Tabs.Content>

            <Tabs.Content value="urls" class="p-4">
              <div class="space-y-4">
                <div>
                  <Label for="urls">Paste Street View URLs (one per line)</Label>
                  <textarea
                    id="urls"
                    bind:value={urlInput}
                    placeholder="https://www.google.com/maps/@48.8584,2.2945,3a..."
                    rows="5"
                    class="mt-1.5 w-full px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono"
                  ></textarea>
                </div>

                <Button onclick={importUrls} disabled={importingUrls || !urlInput.trim()} class="w-full">
                  {importingUrls ? 'Importing...' : 'Import URLs'}
                </Button>

                {#if urlResults.length > 0}
                  <div class="space-y-1 max-h-32 overflow-y-auto">
                    {#each urlResults as result}
                      <div
                        class="flex items-center gap-2 text-xs p-2 rounded
                               {result.success ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-700'}"
                      >
                        {#if result.success}
                          <CheckIcon class="w-3 h-3 shrink-0" />
                        {:else}
                          <AlertCircleIcon class="w-3 h-3 shrink-0" />
                        {/if}
                        <span class="truncate">{result.url}</span>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </Tabs.Content>
          </Tabs.Root>
        </div>

        <!-- Right: Selected locations -->
        <div class="bg-white rounded-xl shadow overflow-hidden">
          <div class="p-4 border-b border-gray-100">
            <h3 class="font-semibold text-gray-900">
              Locations ({map.location_count.toLocaleString()})
            </h3>
          </div>

          {#if selectedLocations.length === 0}
            <div class="p-8 text-center">
              <MapIcon class="w-10 h-10 mx-auto text-gray-300 mb-2" />
              <p class="text-gray-500 text-sm">No locations yet</p>
            </div>
          {:else}
            <div class="divide-y divide-gray-100 max-h-80 overflow-y-auto">
              {#each selectedLocations as location (location.id)}
                <div class="flex items-center justify-between px-4 py-2 text-sm">
                  <div>
                    <span class="font-mono text-xs text-gray-400">{location.country_code || '??'}</span>
                    <span class="ml-2 text-gray-700">
                      {location.lat.toFixed(4)}, {location.lng.toFixed(4)}
                    </span>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    class="text-red-500 hover:text-red-700 hover:bg-red-50"
                    onclick={() => removeLocation(location.id)}
                  >
                    <TrashIcon class="w-4 h-4" />
                  </Button>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

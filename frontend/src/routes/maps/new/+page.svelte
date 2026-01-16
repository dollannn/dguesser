<script lang="ts">
  import { goto } from '$app/navigation';
  import { user } from '$lib/stores/auth';
  import {
    mapsApi,
    locationsApi,
    type CreateMapRequest,
    type MapVisibility,
    type LocationSearchItem,
    type CountryInfo,
    type MapLocationItem,
  } from '$lib/api/maps';
  import { ApiClientError } from '$lib/api/client';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import * as Select from '$lib/components/ui/select';
  import * as Tabs from '$lib/components/ui/tabs';
  import * as Dialog from '$lib/components/ui/dialog';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import SearchIcon from '@lucide/svelte/icons/search';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import LinkIcon from '@lucide/svelte/icons/link';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import LockIcon from '@lucide/svelte/icons/lock';
  import MapIcon from '@lucide/svelte/icons/map';
  import XIcon from '@lucide/svelte/icons/x';
  import CheckIcon from '@lucide/svelte/icons/check';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import SEO from '$lib/components/SEO.svelte';

  // State
  let name = $state('');
  let description = $state('');
  let visibility = $state<MapVisibility>('private');
  let createdMapId = $state<string | null>(null);

  // Search state
  let countries = $state<CountryInfo[]>([]);
  let selectedCountry = $state<string>('');
  let searchResults = $state<LocationSearchItem[]>([]);
  let searchTotal = $state(0);
  let searchPage = $state(1);
  let searching = $state(false);
  let countriesLoading = $state(true);

  // Selected locations
  let selectedLocations = $state<MapLocationItem[]>([]);

  // URL import state
  let showUrlDialog = $state(false);
  let urlInput = $state('');
  let urlResults = $state<Array<{ url: string; success: boolean; error?: string }>>([]);
  let importingUrls = $state(false);

  // Form state
  let creating = $state(false);
  let error = $state('');
  let nameError = $state('');

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

  // Load countries on mount
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
    if (!selectedCountry) return;

    searching = true;
    try {
      const response = await locationsApi.search({
        country_code: selectedCountry || undefined,
        exclude_map_id: createdMapId || undefined,
        page: searchPage,
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

  // Create map (step 1)
  async function createMap() {
    nameError = validateName(name);
    if (nameError) return;

    creating = true;
    error = '';

    try {
      const response = await mapsApi.create({
        name: name.trim(),
        description: description.trim() || undefined,
        visibility,
      });
      createdMapId = response.id;
    } catch (e) {
      if (e instanceof ApiClientError) {
        error = e.message;
      } else {
        error = 'Failed to create map';
      }
    } finally {
      creating = false;
    }
  }

  // Add location to map
  async function addLocation(location: LocationSearchItem) {
    if (!createdMapId) return;

    try {
      await mapsApi.addLocations(createdMapId, [location.id]);
      // Add to selected list
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
      // Remove from search results
      searchResults = searchResults.filter((l) => l.id !== location.id);
    } catch (e) {
      console.error('Failed to add location:', e);
    }
  }

  // Remove location from map
  async function removeLocation(locationId: string) {
    if (!createdMapId) return;

    try {
      await mapsApi.removeLocation(createdMapId, locationId);
      selectedLocations = selectedLocations.filter((l) => l.id !== locationId);
    } catch (e) {
      console.error('Failed to remove location:', e);
    }
  }

  // Import URLs
  async function importUrls() {
    if (!createdMapId || !urlInput.trim()) return;

    const urls = urlInput
      .split('\n')
      .map((u) => u.trim())
      .filter((u) => u.length > 0);

    if (urls.length === 0) return;

    importingUrls = true;
    urlResults = [];

    try {
      const response = await mapsApi.addLocationsFromUrls(createdMapId, urls);
      urlResults = response.results.map((r) => ({
        url: r.url,
        success: r.success,
        error: r.error ?? undefined,
      }));

      // Reload selected locations count
      const mapData = await mapsApi.get(createdMapId);
      // Fetch the newly added locations
      const locsResponse = await mapsApi.getLocations(createdMapId, 1, 100);
      selectedLocations = locsResponse.locations;
    } catch (e) {
      console.error('Failed to import URLs:', e);
    } finally {
      importingUrls = false;
    }
  }

  // Finish and go to map
  function finishCreation() {
    if (createdMapId) {
      goto(`/maps/${createdMapId}`);
    }
  }

  // Load countries when authenticated
  $effect(() => {
    if ($user) {
      loadCountries();
    }
  });

  // Search when country changes
  $effect(() => {
    if (selectedCountry && createdMapId) {
      searchPage = 1;
      searchLocations();
    }
  });
</script>

<SEO title="Create Map" noindex />

<div class="max-w-4xl mx-auto px-4 py-8">
  <!-- Back link -->
  <a href="/maps" class="inline-flex items-center gap-1 text-gray-500 hover:text-gray-700 mb-6">
    <ChevronLeftIcon class="w-4 h-4" />
    Back to Maps
  </a>

  <h1 class="text-3xl font-bold text-gray-900 mb-8">Create New Map</h1>

  <!-- Not authenticated -->
  {#if !$user}
    <div class="text-center py-16 bg-white rounded-xl shadow">
      <LockIcon class="w-16 h-16 mx-auto text-gray-300 mb-4" />
      <p class="text-lg text-gray-700 font-medium">Sign in to create maps</p>
      <a href="/auth" class="btn-primary mt-6 inline-block">Sign In</a>
    </div>
  {:else if !createdMapId}
    <!-- Step 1: Map details -->
    <div class="bg-white rounded-xl shadow p-6">
      <h2 class="text-xl font-semibold text-gray-900 mb-6">Map Details</h2>

      {#if error}
        <div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
          {error}
        </div>
      {/if}

      <div class="space-y-6">
        <!-- Name -->
        <div>
          <Label for="name">Name *</Label>
          <Input
            id="name"
            bind:value={name}
            placeholder="My Custom Map"
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
            placeholder="A collection of interesting locations..."
            rows="3"
            class="mt-1.5 w-full px-3 py-2 border border-gray-300 rounded-lg text-sm
                   focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent"
          ></textarea>
        </div>

        <!-- Visibility -->
        <div>
          <Label>Visibility</Label>
          <div class="mt-1.5 grid grid-cols-3 gap-3">
            {#each visibilityOptions as option}
              <button
                type="button"
                onclick={() => (visibility = option.value as MapVisibility)}
                class="p-3 rounded-lg border text-left transition-all
                       {visibility === option.value
                  ? 'border-gray-900 bg-gray-50 ring-1 ring-gray-900'
                  : 'border-gray-200 hover:border-gray-300'}"
              >
                <svelte:component
                  this={option.icon}
                  class="w-5 h-5 mb-2 {visibility === option.value ? 'text-gray-900' : 'text-gray-400'}"
                />
                <div class="font-medium text-sm text-gray-900">{option.label}</div>
                <div class="text-xs text-gray-500">{option.desc}</div>
              </button>
            {/each}
          </div>
        </div>

        <!-- Submit -->
        <div class="pt-4">
          <Button onclick={createMap} disabled={creating} class="w-full">
            {creating ? 'Creating...' : 'Create Map & Add Locations'}
          </Button>
        </div>
      </div>
    </div>
  {:else}
    <!-- Step 2: Add locations -->
    <div class="space-y-6">
      <!-- Map created banner -->
      <div class="bg-green-50 border border-green-200 rounded-xl p-4 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <CheckIcon class="w-5 h-5 text-green-600" />
          <div>
            <p class="font-medium text-green-800">Map "{name}" created!</p>
            <p class="text-sm text-green-600">Now add locations to your map</p>
          </div>
        </div>
        <Button variant="outline" size="sm" onclick={finishCreation}>
          Finish & View Map
        </Button>
      </div>

      <!-- Location builder -->
      <div class="grid gap-6 lg:grid-cols-2">
        <!-- Left: Search and add -->
        <div class="bg-white rounded-xl shadow overflow-hidden">
          <Tabs.Root value="search" class="w-full">
            <Tabs.List class="w-full border-b">
              <Tabs.Trigger value="search" class="flex-1">
                <SearchIcon class="w-4 h-4 mr-2" />
                Search Locations
              </Tabs.Trigger>
              <Tabs.Trigger value="urls" class="flex-1">
                <LinkIcon class="w-4 h-4 mr-2" />
                Import URLs
              </Tabs.Trigger>
            </Tabs.List>

            <Tabs.Content value="search" class="p-4">
              <!-- Country filter -->
              <div class="mb-4">
                <Label for="country">Filter by Country</Label>
                <select
                  id="country"
                  bind:value={selectedCountry}
                  class="mt-1.5 w-full px-3 py-2 border border-gray-300 rounded-lg text-sm
                         focus:outline-none focus:ring-2 focus:ring-gray-900"
                >
                  <option value="">Select a country...</option>
                  {#each countries as country}
                    <option value={country.code}>
                      {country.code} ({country.count.toLocaleString()} locations)
                    </option>
                  {/each}
                </select>
              </div>

              <!-- Search results -->
              {#if searching}
                <div class="py-8 text-center">
                  <div class="animate-spin w-6 h-6 border-2 border-gray-300 border-t-gray-600 rounded-full mx-auto"></div>
                </div>
              {:else if searchResults.length > 0}
                <div class="space-y-2 max-h-64 overflow-y-auto">
                  {#each searchResults as location (location.id)}
                    <div
                      class="flex items-center justify-between p-2 rounded-lg hover:bg-gray-50 text-sm"
                    >
                      <div>
                        <span class="font-mono text-xs text-gray-400">{location.country_code}</span>
                        <span class="ml-2 text-gray-700">
                          {location.lat.toFixed(4)}, {location.lng.toFixed(4)}
                        </span>
                      </div>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        onclick={() => addLocation(location)}
                      >
                        <PlusIcon class="w-4 h-4" />
                      </Button>
                    </div>
                  {/each}
                </div>
                <p class="text-xs text-gray-500 mt-3 text-center">
                  Showing {searchResults.length} of {searchTotal.toLocaleString()} locations
                </p>
              {:else if selectedCountry}
                <p class="text-gray-500 text-sm text-center py-8">
                  No locations found for this country
                </p>
              {:else}
                <p class="text-gray-500 text-sm text-center py-8">
                  Select a country to search locations
                </p>
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
                    rows="6"
                    class="mt-1.5 w-full px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono
                           focus:outline-none focus:ring-2 focus:ring-gray-900"
                  ></textarea>
                </div>

                <Button onclick={importUrls} disabled={importingUrls || !urlInput.trim()} class="w-full">
                  {importingUrls ? 'Importing...' : 'Import URLs'}
                </Button>

                {#if urlResults.length > 0}
                  <div class="mt-4 space-y-1 max-h-40 overflow-y-auto">
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
                        {#if result.error}
                          <span class="ml-auto text-red-600">{result.error}</span>
                        {/if}
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
              Selected Locations ({selectedLocations.length})
            </h3>
          </div>

          {#if selectedLocations.length === 0}
            <div class="p-8 text-center">
              <MapIcon class="w-10 h-10 mx-auto text-gray-300 mb-2" />
              <p class="text-gray-500 text-sm">No locations added yet</p>
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

          <div class="p-4 border-t border-gray-100">
            <Button onclick={finishCreation} class="w-full">
              Finish & View Map
            </Button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

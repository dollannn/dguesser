<script lang="ts">
  import type { GameSettings } from '$lib/api/games';
  import { mapsApi, type MapSummary } from '$lib/api/maps';
  import { Slider } from '$lib/components/ui/slider';
  import { Switch } from '$lib/components/ui/switch';
  import { Label } from '$lib/components/ui/label';
  import * as ToggleGroup from '$lib/components/ui/toggle-group';
  import TargetIcon from '@lucide/svelte/icons/target';
  import ClockIcon from '@lucide/svelte/icons/clock';
  import FootprintsIcon from '@lucide/svelte/icons/footprints';
  import SearchIcon from '@lucide/svelte/icons/search';
  import CompassIcon from '@lucide/svelte/icons/compass';
  import ZapIcon from '@lucide/svelte/icons/zap';
  import HourglassIcon from '@lucide/svelte/icons/hourglass';
  import BanIcon from '@lucide/svelte/icons/ban';
  import SlidersHorizontalIcon from '@lucide/svelte/icons/sliders-horizontal';
  import MapIcon from '@lucide/svelte/icons/map';
  import LoaderIcon from '@lucide/svelte/icons/loader';

  interface Props {
    settings: GameSettings;
    readonly?: boolean;
    onchange?: (settings: Partial<GameSettings>) => void;
  }

  let { settings, readonly = false, onchange }: Props = $props();

  // Local state for form values - initialized in $effect below
  let rounds = $state(5);
  let timeLimitSeconds = $state(120);
  let unlimitedTime = $state(false);
  let movementAllowed = $state(true);
  let zoomAllowed = $state(true);
  let rotationAllowed = $state(true);
  let mapId = $state('');
  let initialized = $state(false);

  // Maps list state
  let maps = $state<MapSummary[]>([]);
  let mapsLoading = $state(true);
  let mapsError = $state<string | null>(null);

  // Fetch available maps on mount
  $effect(() => {
    loadMaps();
  });

  async function loadMaps() {
    mapsLoading = true;
    mapsError = null;
    try {
      const response = await mapsApi.list();
      maps = response.maps.filter(m => m.location_count > 0); // Only maps with locations
    } catch (e) {
      console.error('Failed to load maps:', e);
      mapsError = 'Failed to load maps';
    } finally {
      mapsLoading = false;
    }
  }

  // Find a map by ID or slug (backend accepts both)
  function findMapByIdOrSlug(idOrSlug: string): MapSummary | undefined {
    return maps.find(m => m.id === idOrSlug || m.slug === idOrSlug);
  }

  // Get the current map for display
  let currentMap = $derived(findMapByIdOrSlug(mapId));
  let currentMapName = $derived(currentMap?.name ?? 'World');

  // Sync from external settings changes (including initial load)
  $effect(() => {
    rounds = settings.rounds;
    timeLimitSeconds = settings.time_limit_seconds;
    unlimitedTime = settings.time_limit_seconds === 0;
    movementAllowed = settings.movement_allowed;
    zoomAllowed = settings.zoom_allowed;
    rotationAllowed = settings.rotation_allowed;
    initialized = true;
  });

  // Normalize map_id after maps are loaded (handles slug -> id conversion)
  // This runs when maps load or when settings.map_id changes
  $effect(() => {
    // Wait for maps to load before normalizing
    if (mapsLoading || maps.length === 0) {
      // Keep the raw map_id until maps are available
      mapId = settings.map_id;
      return;
    }
    // Normalize map_id to actual ID if we find a matching map
    const foundMap = findMapByIdOrSlug(settings.map_id);
    mapId = foundMap?.id ?? settings.map_id;
  });

  // Detect which preset matches current settings
  let currentPreset = $derived(detectPreset());

  // Presets configuration
  const presets: { id: string; name: string; icon: typeof ZapIcon; description: string }[] = [
    { id: 'classic', name: 'Classic', icon: TargetIcon, description: 'Standard game' },
    { id: 'nomove', name: 'No Move', icon: BanIcon, description: 'Fixed position only' },
    { id: 'speedround', name: 'Speed', icon: ZapIcon, description: '30 second rounds' },
    { id: 'explorer', name: 'Explorer', icon: HourglassIcon, description: 'No time limit' },
    { id: 'custom', name: 'Custom', icon: SlidersHorizontalIcon, description: 'Your settings' },
  ];

  function detectPreset(): string {
    // Check if settings match known presets
    if (rounds === 5 && timeLimitSeconds === 120 && movementAllowed && zoomAllowed && rotationAllowed) {
      return 'classic';
    }
    if (rounds === 5 && timeLimitSeconds === 120 && !movementAllowed && !zoomAllowed && rotationAllowed) {
      return 'nomove';
    }
    if (rounds === 5 && timeLimitSeconds === 30 && movementAllowed && zoomAllowed && rotationAllowed) {
      return 'speedround';
    }
    if (rounds === 10 && timeLimitSeconds === 0 && movementAllowed && zoomAllowed && rotationAllowed) {
      return 'explorer';
    }
    return 'custom';
  }

  function applyPreset(presetId: string) {
    if (readonly) return;
    
    switch (presetId) {
      case 'classic':
        rounds = 5;
        timeLimitSeconds = 120;
        unlimitedTime = false;
        movementAllowed = true;
        zoomAllowed = true;
        rotationAllowed = true;
        break;
      case 'nomove':
        rounds = 5;
        timeLimitSeconds = 120;
        unlimitedTime = false;
        movementAllowed = false;
        zoomAllowed = false;
        rotationAllowed = true;
        break;
      case 'speedround':
        rounds = 5;
        timeLimitSeconds = 30;
        unlimitedTime = false;
        movementAllowed = true;
        zoomAllowed = true;
        rotationAllowed = true;
        break;
      case 'explorer':
        rounds = 10;
        timeLimitSeconds = 0;
        unlimitedTime = true;
        movementAllowed = true;
        zoomAllowed = true;
        rotationAllowed = true;
        break;
      // custom - don't change anything
    }
    notifyChange();
  }

  function notifyChange() {
    if (readonly || !onchange) return;
    
    onchange({
      rounds,
      time_limit_seconds: unlimitedTime ? 0 : timeLimitSeconds,
      movement_allowed: movementAllowed,
      zoom_allowed: zoomAllowed,
      rotation_allowed: rotationAllowed,
      map_id: mapId,
    });
  }

  function handleMapChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    mapId = select.value;
    notifyChange();
  }

  // Debounce changes
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  function debouncedNotify() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(notifyChange, 300);
  }

  function formatTime(seconds: number): string {
    if (seconds === 0) return 'Unlimited';
    if (seconds < 60) return `${seconds}s`;
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return secs > 0 ? `${mins}m ${secs}s` : `${mins}m`;
  }
</script>

<div class="space-y-6">
  <!-- Map Selection -->
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <Label class="flex items-center gap-2 text-sm font-medium">
        <MapIcon class="size-4 text-primary" />
        Map
      </Label>
      {#if mapsLoading}
        <LoaderIcon class="size-4 animate-spin text-muted-foreground" />
      {/if}
    </div>
    {#if readonly}
      <div class="flex items-center gap-2 px-3 py-2 rounded-md border bg-muted/50">
        <span class="text-sm font-medium">{currentMapName}</span>
        {#if currentMap?.location_count}
          <span class="text-xs text-muted-foreground">
            ({currentMap.location_count} locations)
          </span>
        {/if}
      </div>
    {:else if mapsError}
      <div class="text-sm text-destructive">{mapsError}</div>
    {:else}
      <select
        value={mapId}
        onchange={handleMapChange}
        disabled={mapsLoading}
        class="w-full px-3 py-2 rounded-md border border-input bg-background text-sm
               focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2
               disabled:cursor-not-allowed disabled:opacity-50"
      >
        {#each maps as map}
          <option value={map.id}>
            {map.name} ({map.location_count} locations)
          </option>
        {/each}
      </select>
    {/if}
  </div>

  <!-- Presets -->
  {#if !readonly}
    <div>
      <Label class="text-sm font-medium text-muted-foreground mb-3 block">Quick Settings</Label>
      <ToggleGroup.Root 
        type="single" 
        value={currentPreset}
        onValueChange={(v) => v && applyPreset(v)}
        class="grid grid-cols-3 sm:grid-cols-5 gap-2"
      >
        {#each presets as preset}
          <ToggleGroup.Item 
            value={preset.id} 
            class="flex items-center justify-center gap-1.5 px-2 py-2 rounded-md border data-[state=on]:bg-primary data-[state=on]:text-primary-foreground"
            aria-label={preset.name}
          >
            <preset.icon class="size-4 shrink-0" />
            <span class="text-sm font-medium truncate">{preset.name}</span>
          </ToggleGroup.Item>
        {/each}
      </ToggleGroup.Root>
    </div>
  {/if}

  <!-- Rounds -->
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <Label class="flex items-center gap-2 text-sm font-medium">
        <TargetIcon class="size-4 text-primary" />
        Rounds
      </Label>
      <span class="text-sm font-semibold tabular-nums">{rounds}</span>
    </div>
    {#if readonly}
      <div class="h-1.5 bg-muted rounded-full overflow-hidden">
        <div class="h-full bg-primary" style="width: {(rounds / 20) * 100}%"></div>
      </div>
    {:else}
      <Slider 
        type="single"
        bind:value={rounds}
        min={1} 
        max={20} 
        step={1}
        onValueChange={() => debouncedNotify()}
      />
      <div class="flex justify-between text-xs text-muted-foreground">
        <span>1</span>
        <span>20</span>
      </div>
    {/if}
  </div>

  <!-- Time Limit -->
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <Label class="flex items-center gap-2 text-sm font-medium">
        <ClockIcon class="size-4 text-primary" />
        Time Limit
      </Label>
      <span class="text-sm font-semibold">{formatTime(unlimitedTime ? 0 : timeLimitSeconds)}</span>
    </div>
    {#if readonly}
      <div class="h-1.5 bg-muted rounded-full overflow-hidden">
        <div class="h-full bg-primary" style="width: {unlimitedTime ? 100 : (timeLimitSeconds / 600) * 100}%"></div>
      </div>
    {:else}
      <div class="space-y-2">
        <div class="flex items-center gap-2">
          <Switch 
            bind:checked={unlimitedTime} 
            onCheckedChange={() => debouncedNotify()}
          />
          <span class="text-sm text-muted-foreground">Unlimited time</span>
        </div>
        {#if !unlimitedTime}
          <Slider 
            type="single"
            bind:value={timeLimitSeconds}
            min={10} 
            max={600} 
            step={10}
            onValueChange={() => debouncedNotify()}
          />
          <div class="flex justify-between text-xs text-muted-foreground">
            <span>10s</span>
            <span>10m</span>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Movement Controls -->
  <div class="space-y-4">
    <Label class="text-sm font-medium text-muted-foreground block">Street View Controls</Label>
    
    <div class="space-y-3">
      <!-- Movement -->
      <div class="flex items-center justify-between">
        <Label class="flex items-center gap-2 text-sm">
          <FootprintsIcon class="size-4 text-muted-foreground" />
          Movement
          <span class="text-xs text-muted-foreground">(walk around)</span>
        </Label>
        {#if readonly}
          <span class="text-sm font-medium {movementAllowed ? 'text-green-600' : 'text-red-600'}">
            {movementAllowed ? 'Allowed' : 'Disabled'}
          </span>
        {:else}
          <Switch 
            bind:checked={movementAllowed} 
            onCheckedChange={() => debouncedNotify()}
          />
        {/if}
      </div>

      <!-- Zoom -->
      <div class="flex items-center justify-between">
        <Label class="flex items-center gap-2 text-sm">
          <SearchIcon class="size-4 text-muted-foreground" />
          Zoom
          <span class="text-xs text-muted-foreground">(zoom in/out)</span>
        </Label>
        {#if readonly}
          <span class="text-sm font-medium {zoomAllowed ? 'text-green-600' : 'text-red-600'}">
            {zoomAllowed ? 'Allowed' : 'Disabled'}
          </span>
        {:else}
          <Switch 
            bind:checked={zoomAllowed} 
            onCheckedChange={() => debouncedNotify()}
          />
        {/if}
      </div>

      <!-- Rotation/Compass -->
      <div class="flex items-center justify-between">
        <Label class="flex items-center gap-2 text-sm">
          <CompassIcon class="size-4 text-muted-foreground" />
          Rotation
          <span class="text-xs text-muted-foreground">(look around)</span>
        </Label>
        {#if readonly}
          <span class="text-sm font-medium {rotationAllowed ? 'text-green-600' : 'text-red-600'}">
            {rotationAllowed ? 'Allowed' : 'Disabled'}
          </span>
        {:else}
          <Switch 
            bind:checked={rotationAllowed} 
            onCheckedChange={() => debouncedNotify()}
          />
        {/if}
      </div>
    </div>
  </div>
</div>

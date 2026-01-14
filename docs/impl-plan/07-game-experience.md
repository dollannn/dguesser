# Phase 7: Game Experience

**Priority:** P1  
**Duration:** 5-7 days  
**Dependencies:** Phases 5 & 6 (Realtime + Frontend Foundation)

## Objectives

- Build the core gameplay UI
- Implement map/Street View integration
- Create guess submission flow
- Build real-time multiplayer experience
- Add round results and final standings views
- Implement game timer
- **Handle prefixed nanoid IDs in all components**

## Deliverables

### 7.1 Game Page Structure

**frontend/src/routes/game/[id]/+page.svelte:**
```svelte
<script lang="ts">
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { gamesApi, type GameDetails, type RoundInfo } from '$lib/api/games';
  import { gameStore, initGameSocketListeners } from '$lib/socket/game';
  import { socketClient } from '$lib/socket/client';
  
  import GameLoading from '$lib/components/game/GameLoading.svelte';
  import GameLobby from '$lib/components/game/GameLobby.svelte';
  import GamePlay from '$lib/components/game/GamePlay.svelte';
  import GameRoundEnd from '$lib/components/game/GameRoundEnd.svelte';
  import GameFinished from '$lib/components/game/GameFinished.svelte';

  let game: GameDetails | null = null;
  let loading = true;
  let error = '';

  $: gameId = $page.params.id;
  $: gameState = $gameStore;

  onMount(async () => {
    try {
      // Load game data
      game = await gamesApi.get(gameId);
      
      // Initialize socket listeners
      const cleanup = initGameSocketListeners();
      
      // Join game room
      if (game.mode === 'multiplayer') {
        gameStore.joinGame(gameId);
      }

      loading = false;

      return cleanup;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load game';
      loading = false;
    }
  });

  onDestroy(() => {
    gameStore.leaveGame();
  });

  async function startGame() {
    try {
      if (game?.mode === 'solo') {
        const round = await gamesApi.start(gameId);
        gameStore.handleRoundStart({
          round_number: round.round_number,
          total_rounds: game.total_rounds,
          location: round.location,
          time_limit_ms: round.time_limit_ms,
          started_at: Date.now(),
        });
      } else {
        // Multiplayer - emit via socket
        gameStore.startGame();
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to start game';
    }
  }
</script>

<svelte:head>
  <title>Game - dguesser</title>
</svelte:head>

{#if loading}
  <GameLoading />
{:else if error}
  <div class="max-w-lg mx-auto mt-12 p-6 bg-red-50 rounded-xl text-center">
    <h2 class="text-xl font-semibold text-red-700 mb-2">Error</h2>
    <p class="text-red-600 mb-4">{error}</p>
    <a href="/" class="btn-primary">Back to Home</a>
  </div>
{:else if game}
  {#if gameState.status === 'idle' || gameState.status === 'lobby'}
    <GameLobby {game} onStart={startGame} />
  {:else if gameState.status === 'playing'}
    <GamePlay {game} />
  {:else if gameState.status === 'round_end'}
    <GameRoundEnd {game} />
  {:else if gameState.status === 'finished'}
    <GameFinished {game} />
  {/if}
{/if}
```

### 7.2 Game Lobby Component

**frontend/src/lib/components/game/GameLobby.svelte:**
```svelte
<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { user } from '$lib/stores/auth';
  import { gameStore } from '$lib/socket/game';

  export let game: GameDetails;
  export let onStart: () => void;

  $: isHost = game.players.find(p => p.user_id === $user?.id)?.is_host ?? false;
  $: playerCount = game.players.length;
  $: canStart = isHost && (game.mode === 'solo' || playerCount >= 2);
</script>

<div class="max-w-2xl mx-auto px-4 py-12">
  <div class="card">
    <div class="text-center mb-8">
      <h1 class="text-3xl font-bold mb-2">
        {game.mode === 'solo' ? 'Solo Game' : 'Multiplayer Lobby'}
      </h1>
      
      {#if game.join_code}
        <div class="mt-4 p-4 bg-gray-100 rounded-lg">
          <p class="text-sm text-gray-600 mb-1">Share this code with friends:</p>
          <p class="text-3xl font-mono font-bold tracking-wider text-primary-600">
            {game.join_code}
          </p>
        </div>
      {/if}
    </div>

    <!-- Game Settings -->
    <div class="mb-8 p-4 bg-gray-50 rounded-lg">
      <h3 class="font-semibold mb-3">Game Settings</h3>
      <div class="grid grid-cols-2 gap-4 text-sm">
        <div>
          <span class="text-gray-600">Rounds:</span>
          <span class="font-medium">{game.settings.rounds}</span>
        </div>
        <div>
          <span class="text-gray-600">Time Limit:</span>
          <span class="font-medium">
            {game.settings.time_limit_seconds > 0 
              ? `${game.settings.time_limit_seconds}s` 
              : 'Unlimited'}
          </span>
        </div>
        <div>
          <span class="text-gray-600">Movement:</span>
          <span class="font-medium">
            {game.settings.movement_allowed ? 'Allowed' : 'Disabled'}
          </span>
        </div>
        <div>
          <span class="text-gray-600">Map:</span>
          <span class="font-medium">{game.settings.map_id}</span>
        </div>
      </div>
    </div>

    <!-- Players List (Multiplayer) -->
    {#if game.mode === 'multiplayer'}
      <div class="mb-8">
        <h3 class="font-semibold mb-3">Players ({playerCount})</h3>
        <ul class="space-y-2">
          {#each game.players as player}
            <li class="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
              <span class="font-medium">{player.display_name}</span>
              {#if player.is_host}
                <span class="text-xs bg-primary-100 text-primary-700 px-2 py-1 rounded">
                  Host
                </span>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    <!-- Start Button -->
    <div class="text-center">
      {#if canStart}
        <button onclick={onStart} class="btn-accent text-lg px-8 py-3">
          Start Game
        </button>
      {:else if !isHost}
        <p class="text-gray-600">Waiting for host to start...</p>
      {:else}
        <p class="text-gray-600">Need at least 2 players to start</p>
      {/if}
    </div>
  </div>
</div>
```

### 7.3 Main Gameplay Component

**frontend/src/lib/components/game/GamePlay.svelte:**
```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { gamesApi } from '$lib/api/games';
  
  import StreetView from './StreetView.svelte';
  import GuessMap from './GuessMap.svelte';
  import GameTimer from './GameTimer.svelte';
  import RoundInfo from './RoundInfo.svelte';

  export let game: GameDetails;

  let guessLat: number | null = null;
  let guessLng: number | null = null;
  let showMap = false;
  let submitting = false;
  let guessStartTime = Date.now();

  $: state = $gameStore;
  $: canSubmit = guessLat !== null && guessLng !== null && !state.hasGuessed;

  function handleMapClick(event: CustomEvent<{ lat: number; lng: number }>) {
    if (state.hasGuessed) return;
    guessLat = event.detail.lat;
    guessLng = event.detail.lng;
  }

  async function submitGuess() {
    if (!canSubmit || guessLat === null || guessLng === null) return;
    
    submitting = true;
    const timeTaken = Date.now() - guessStartTime;

    try {
      if (game.mode === 'solo') {
        // Solo mode - use REST API
        const result = await gamesApi.submitGuess(
          game.id,
          state.currentRound,
          guessLat,
          guessLng,
          timeTaken
        );
        
        // Handle result locally
        gameStore.handleRoundEnd({
          round_number: state.currentRound,
          correct_location: result.correct_location,
          results: [{
            user_id: '', // Will be filled
            display_name: '',
            guess_lat: guessLat,
            guess_lng: guessLng,
            distance_meters: result.distance_meters,
            score: result.score,
            total_score: 0,
          }],
        });
      } else {
        // Multiplayer - use socket
        gameStore.submitGuess(guessLat, guessLng, timeTaken);
      }
    } catch (e) {
      console.error('Failed to submit guess:', e);
    } finally {
      submitting = false;
    }
  }

  function handleTimeUp() {
    // Auto-submit if not guessed
    if (!state.hasGuessed && guessLat !== null && guessLng !== null) {
      submitGuess();
    }
  }

  onMount(() => {
    guessStartTime = Date.now();
  });
</script>

<div class="h-screen flex flex-col">
  <!-- Top bar with round info and timer -->
  <div class="bg-white shadow-sm z-10 p-4">
    <div class="max-w-7xl mx-auto flex justify-between items-center">
      <RoundInfo 
        currentRound={state.currentRound}
        totalRounds={state.totalRounds}
      />
      
      {#if state.timeLimit}
        <GameTimer 
          startedAt={state.roundStartedAt}
          durationMs={state.timeLimit}
          onTimeUp={handleTimeUp}
        />
      {/if}

      <!-- Players who have guessed (multiplayer) -->
      {#if game.mode === 'multiplayer'}
        <div class="flex gap-2">
          {#each [...state.players] as [id, player]}
            <div 
              class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-medium"
              class:bg-green-100={player.hasGuessed}
              class:text-green-700={player.hasGuessed}
              class:bg-gray-100={!player.hasGuessed}
              class:text-gray-500={!player.hasGuessed}
              title={player.displayName}
            >
              {player.displayName.charAt(0).toUpperCase()}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Main game area -->
  <div class="flex-1 relative">
    <!-- Street View -->
    {#if state.location}
      <StreetView 
        lat={state.location.lat}
        lng={state.location.lng}
        panoramaId={state.location.panorama_id}
        movementAllowed={game.settings.movement_allowed}
        zoomAllowed={game.settings.zoom_allowed}
      />
    {/if}

    <!-- Mini map / Guess map -->
    <div 
      class="absolute bottom-4 right-4 transition-all duration-300"
      class:w-64={!showMap}
      class:h-48={!showMap}
      class:w-[600px]={showMap}
      class:h-[400px]={showMap}
    >
      <div 
        class="relative w-full h-full bg-white rounded-lg shadow-lg overflow-hidden"
        role="button"
        tabindex="0"
        onmouseenter={() => showMap = true}
        onmouseleave={() => !guessLat && (showMap = false)}
      >
        <GuessMap 
          guessLat={guessLat}
          guessLng={guessLng}
          disabled={state.hasGuessed}
          on:click={handleMapClick}
        />
        
        <!-- Submit button overlay -->
        {#if showMap && canSubmit}
          <div class="absolute bottom-4 left-1/2 -translate-x-1/2">
            <button
              onclick={submitGuess}
              disabled={submitting}
              class="btn-accent px-8 py-3 text-lg shadow-lg"
            >
              {submitting ? 'Submitting...' : 'Submit Guess'}
            </button>
          </div>
        {/if}
      </div>
    </div>

    <!-- Already guessed indicator -->
    {#if state.hasGuessed}
      <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-black/80 text-white px-6 py-4 rounded-lg text-center">
        <p class="text-lg font-semibold">Guess submitted!</p>
        <p class="text-sm text-gray-300">Waiting for other players...</p>
      </div>
    {/if}
  </div>
</div>
```

### 7.4 Street View Component

**frontend/src/lib/components/game/StreetView.svelte:**
```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';

  export let lat: number;
  export let lng: number;
  export let panoramaId: string | null = null;
  export let movementAllowed = true;
  export let zoomAllowed = true;

  let container: HTMLDivElement;
  let panorama: google.maps.StreetViewPanorama | null = null;

  onMount(() => {
    if (!browser) return;

    // Initialize Street View
    // Note: Requires Google Maps API to be loaded
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
  $: if (panorama && lat && lng) {
    if (panoramaId) {
      panorama.setPano(panoramaId);
    } else {
      panorama.setPosition({ lat, lng });
    }
  }
</script>

<div 
  bind:this={container} 
  class="w-full h-full bg-gray-900"
/>

<style>
  /* Hide Google branding */
  div :global(.gm-style-cc),
  div :global(.gmnoprint) {
    display: none !important;
  }
</style>
```

### 7.5 Guess Map Component

**frontend/src/lib/components/game/GuessMap.svelte:**
```svelte
<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { browser } from '$app/environment';

  export let guessLat: number | null = null;
  export let guessLng: number | null = null;
  export let disabled = false;

  const dispatch = createEventDispatcher<{
    click: { lat: number; lng: number };
  }>();

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
      
      dispatch('click', { lat, lng });
    });
  });

  // Update marker when guess changes
  $: if (map && guessLat !== null && guessLng !== null) {
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
</script>

<div 
  bind:this={container} 
  class="w-full h-full"
  class:cursor-crosshair={!disabled}
  class:cursor-not-allowed={disabled}
/>
```

### 7.6 Timer Component

**frontend/src/lib/components/game/GameTimer.svelte:**
```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export let startedAt: number | null;
  export let durationMs: number;
  export let onTimeUp: () => void;

  let remaining = durationMs;
  let intervalId: number;

  $: if (startedAt) {
    remaining = Math.max(0, durationMs - (Date.now() - startedAt));
  }

  $: minutes = Math.floor(remaining / 60000);
  $: seconds = Math.floor((remaining % 60000) / 1000);
  $: isLow = remaining < 10000;
  $: isExpired = remaining <= 0;

  onMount(() => {
    intervalId = setInterval(() => {
      if (!startedAt) return;
      
      remaining = Math.max(0, durationMs - (Date.now() - startedAt));
      
      if (remaining <= 0) {
        clearInterval(intervalId);
        onTimeUp();
      }
    }, 100);
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });
</script>

<div 
  class="font-mono text-2xl font-bold px-4 py-2 rounded-lg"
  class:bg-red-100={isLow}
  class:text-red-600={isLow}
  class:bg-gray-100={!isLow}
  class:text-gray-700={!isLow}
  class:animate-pulse={isLow && !isExpired}
>
  {minutes}:{seconds.toString().padStart(2, '0')}
</div>
```

### 7.7 Round End Component

**frontend/src/lib/components/game/GameRoundEnd.svelte:**
```svelte
<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import ResultsMap from './ResultsMap.svelte';

  export let game: GameDetails;

  $: state = $gameStore;
  $: results = state.results;
  $: correctLocation = state.location;

  function formatDistance(meters: number): string {
    if (meters < 1000) {
      return `${Math.round(meters)} m`;
    }
    return `${(meters / 1000).toFixed(1)} km`;
  }

  function nextRound() {
    // For solo, trigger next round via API
    // For multiplayer, server will auto-advance
  }
</script>

<div class="min-h-screen bg-gray-50 p-4">
  <div class="max-w-6xl mx-auto">
    <div class="text-center mb-6">
      <h2 class="text-2xl font-bold">Round {state.currentRound} Results</h2>
    </div>

    <!-- Results map showing all guesses -->
    <div class="h-[400px] rounded-xl overflow-hidden shadow-lg mb-6">
      {#if correctLocation}
        <ResultsMap 
          correctLat={correctLocation.lat}
          correctLng={correctLocation.lng}
          guesses={results.map(r => ({
            lat: r.guess_lat,
            lng: r.guess_lng,
            displayName: r.display_name,
          }))}
        />
      {/if}
    </div>

    <!-- Results table -->
    <div class="card">
      <table class="w-full">
        <thead>
          <tr class="border-b">
            <th class="text-left py-3 px-4">Player</th>
            <th class="text-right py-3 px-4">Distance</th>
            <th class="text-right py-3 px-4">Score</th>
            <th class="text-right py-3 px-4">Total</th>
          </tr>
        </thead>
        <tbody>
          {#each results.sort((a, b) => b.score - a.score) as result, i}
            <tr class="border-b last:border-0">
              <td class="py-3 px-4">
                <span class="font-medium">{result.display_name}</span>
              </td>
              <td class="text-right py-3 px-4 text-gray-600">
                {formatDistance(result.distance_meters)}
              </td>
              <td class="text-right py-3 px-4 font-semibold text-accent-600">
                +{result.score.toLocaleString()}
              </td>
              <td class="text-right py-3 px-4 font-bold">
                {result.total_score.toLocaleString()}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Continue prompt -->
    {#if state.currentRound < state.totalRounds}
      <div class="text-center mt-6">
        <p class="text-gray-600 mb-4">Next round starting soon...</p>
        {#if game.mode === 'solo'}
          <button onclick={nextRound} class="btn-primary">
            Continue to Round {state.currentRound + 1}
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>
```

### 7.8 Game Finished Component

**frontend/src/lib/components/game/GameFinished.svelte:**
```svelte
<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { user } from '$lib/stores/auth';

  export let game: GameDetails;

  $: standings = $gameStore.finalStandings;
  // user_id is prefixed nanoid (usr_xxxxxxxxxxxx), safe string comparison
  $: myRank = standings.find(s => s.user_id === $user?.id)?.rank;
  $: winner = standings[0];

  function getRankEmoji(rank: number): string {
    switch (rank) {
      case 1: return '🥇';
      case 2: return '🥈';
      case 3: return '🥉';
      default: return '';
    }
  }
</script>

<div class="min-h-screen bg-gradient-to-b from-primary-50 to-white p-4">
  <div class="max-w-2xl mx-auto pt-12">
    <!-- Winner announcement -->
    {#if winner}
      <div class="text-center mb-12">
        <div class="text-6xl mb-4">🏆</div>
        <h1 class="text-3xl font-bold mb-2">
          {winner.display_name} Wins!
        </h1>
        <p class="text-xl text-gray-600">
          with {winner.total_score.toLocaleString()} points
        </p>
      </div>
    {/if}

    <!-- Full standings -->
    <div class="card">
      <h2 class="text-xl font-semibold mb-4">Final Standings</h2>
      
      <div class="space-y-3">
        {#each standings as standing}
          <div 
            class="flex items-center justify-between p-4 rounded-lg"
            class:bg-yellow-50={standing.rank === 1}
            class:bg-gray-50={standing.rank !== 1}
            class:ring-2={standing.user_id === $user?.id}
            class:ring-primary-500={standing.user_id === $user?.id}
          >
            <div class="flex items-center gap-4">
              <span class="text-2xl w-10 text-center">
                {getRankEmoji(standing.rank) || `#${standing.rank}`}
              </span>
              <span class="font-medium">{standing.display_name}</span>
            </div>
            <span class="text-xl font-bold">
              {standing.total_score.toLocaleString()}
            </span>
          </div>
        {/each}
      </div>
    </div>

    <!-- Your result -->
    {#if myRank}
      <div class="mt-6 p-4 bg-primary-50 rounded-xl text-center">
        <p class="text-lg">
          You finished in <span class="font-bold">#{myRank}</span> place!
        </p>
      </div>
    {/if}

    <!-- Actions -->
    <div class="mt-8 flex justify-center gap-4">
      <a href="/" class="btn-secondary">
        Back to Home
      </a>
      <a href={`/game/${game.id}/rematch`} class="btn-primary">
        Play Again
      </a>
    </div>
  </div>
</div>
```

### 7.9 Google Maps Loader

**frontend/src/lib/maps/loader.ts:**
```typescript
let loadPromise: Promise<void> | null = null;

export function loadGoogleMaps(): Promise<void> {
  if (loadPromise) return loadPromise;
  
  if (window.google?.maps) {
    return Promise.resolve();
  }

  loadPromise = new Promise((resolve, reject) => {
    const apiKey = import.meta.env.VITE_GOOGLE_MAPS_API_KEY;
    
    if (!apiKey) {
      reject(new Error('Google Maps API key not configured'));
      return;
    }

    const script = document.createElement('script');
    script.src = `https://maps.googleapis.com/maps/api/js?key=${apiKey}&libraries=places`;
    script.async = true;
    script.defer = true;
    
    script.onload = () => resolve();
    script.onerror = () => reject(new Error('Failed to load Google Maps'));
    
    document.head.appendChild(script);
  });

  return loadPromise;
}
```

**frontend/src/routes/game/[id]/+page.ts:**
```typescript
import { loadGoogleMaps } from '$lib/maps/loader';

export async function load() {
  // Pre-load Google Maps API
  await loadGoogleMaps();
  return {};
}
```

## Alternative: Leaflet + OpenStreetMap

If Google Maps is not preferred, here's a Leaflet-based alternative:

**frontend/src/lib/components/game/LeafletGuessMap.svelte:**
```svelte
<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import L from 'leaflet';
  import 'leaflet/dist/leaflet.css';

  export let guessLat: number | null = null;
  export let guessLng: number | null = null;
  export let disabled = false;

  const dispatch = createEventDispatcher();

  let container: HTMLDivElement;
  let map: L.Map;
  let marker: L.Marker | null = null;

  onMount(() => {
    map = L.map(container).setView([20, 0], 2);
    
    L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
      attribution: '&copy; OpenStreetMap contributors'
    }).addTo(map);

    map.on('click', (e: L.LeafletMouseEvent) => {
      if (disabled) return;
      dispatch('click', { lat: e.latlng.lat, lng: e.latlng.lng });
    });

    return () => {
      map.remove();
    };
  });

  $: if (map && guessLat !== null && guessLng !== null) {
    if (marker) {
      marker.setLatLng([guessLat, guessLng]);
    } else {
      marker = L.marker([guessLat, guessLng]).addTo(map);
    }
  }
</script>

<div bind:this={container} class="w-full h-full" />
```

## Acceptance Criteria

- [ ] Game lobby displays correctly
- [ ] Street View loads and works
- [ ] Can place guess on map
- [ ] Timer counts down correctly
- [ ] Guess submission works
- [ ] Round results display with map
- [ ] Final standings show correctly
- [ ] Multiplayer real-time updates work
- [ ] Disconnection handling works
- [ ] **User matching uses prefixed nanoid IDs correctly**

## Technical Notes

### ID Handling in Components

All IDs in the frontend are strings with prefixes:
- User IDs: `usr_xxxxxxxxxxxx`
- Game IDs: `gam_xxxxxxxxxxxx`

When comparing user IDs (e.g., to highlight current user in standings):
```typescript
// Safe string comparison - both are prefixed nanoids
standings.find(s => s.user_id === $user?.id)
```

Game IDs are used directly in routes (URL-safe):
```typescript
goto(`/game/${game.id}`)  // e.g., /game/gam_FybH2oF9Xaw8
```

## Next Phase

Once game experience is complete, proceed to [Phase 8: Polish & Production](./08-polish-production.md).

<script lang="ts">
  import { onMount } from 'svelte';
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { gamesApi } from '$lib/api/games';
  import { user } from '$lib/stores/auth';

  import StreetView from './StreetView.svelte';
  import GuessMap from './GuessMap.svelte';
  import GameTimer from './GameTimer.svelte';
  import RoundInfo from './RoundInfo.svelte';
  import GameScoreboard from './GameScoreboard.svelte';

  interface Props {
    game: GameDetails;
  }

  let { game }: Props = $props();

  let guessLat: number | null = $state(null);
  let guessLng: number | null = $state(null);
  let showMap = $state(false);
  let submitting = $state(false);
  let guessStartTime = $state(Date.now());

  let gameState = $derived($gameStore);
  let canSubmit = $derived(guessLat !== null && guessLng !== null && !gameState.hasGuessed);

  function handleMapClick(coords: { lat: number; lng: number }) {
    if (gameState.hasGuessed) return;
    guessLat = coords.lat;
    guessLng = coords.lng;
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
          gameState.currentRound,
          guessLat,
          guessLng,
          timeTaken
        );

        // Handle result locally
        gameStore.handleRoundEnd({
          round_number: gameState.currentRound,
          correct_location: result.correct_location,
          results: [
            {
              user_id: '',
              display_name: '',
              guess_lat: guessLat,
              guess_lng: guessLng,
              distance_meters: result.distance_meters,
              score: result.score,
              total_score: 0,
            },
          ],
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
    if (!gameState.hasGuessed && guessLat !== null && guessLng !== null) {
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
      <RoundInfo currentRound={gameState.currentRound} totalRounds={gameState.totalRounds} />

      {#if gameState.timeLimit}
        <GameTimer
          startedAt={gameState.roundStartedAt}
          durationMs={gameState.timeLimit}
          onTimeUp={handleTimeUp}
        />
      {/if}

      <!-- Players who have guessed (multiplayer) -->
      {#if game.mode === 'multiplayer'}
        <div class="flex gap-2">
          {#each [...gameState.players] as [id, player]}
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
    {#if gameState.location}
      <StreetView
        lat={gameState.location.lat}
        lng={gameState.location.lng}
        panoramaId={gameState.location.panorama_id}
        movementAllowed={game.settings.movement_allowed}
        zoomAllowed={game.settings.zoom_allowed}
      />
    {/if}

    <!-- Live scoreboard (multiplayer only) -->
    {#if game.mode === 'multiplayer' && gameState.liveScores.length > 0}
      <div class="absolute top-4 left-4 z-10">
        <GameScoreboard scores={gameState.liveScores} currentUserId={$user?.id ?? null} />
      </div>
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
        onmouseenter={() => (showMap = true)}
        onmouseleave={() => !guessLat && (showMap = false)}
        onkeydown={(e) => e.key === 'Enter' && (showMap = !showMap)}
      >
        <GuessMap
          {guessLat}
          {guessLng}
          disabled={gameState.hasGuessed}
          onclick={handleMapClick}
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
    {#if gameState.hasGuessed}
      <div
        class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-black/80 text-white px-6 py-4 rounded-lg text-center"
      >
        <p class="text-lg font-semibold">Guess submitted!</p>
        <p class="text-sm text-gray-300">Waiting for other players...</p>
      </div>
    {/if}
  </div>
</div>

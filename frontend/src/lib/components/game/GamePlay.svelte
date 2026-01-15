<script lang="ts">
  import { onMount } from 'svelte';
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { gamesApi } from '$lib/api/games';
  import { user } from '$lib/stores/auth';
  import { Send, CheckCircle, Loader2 } from '@lucide/svelte';

  import StreetView from './StreetView.svelte';
  import LeafletMap from './LeafletMap.svelte';
  import CircularTimer from './CircularTimer.svelte';
  import RoundBadge from './RoundBadge.svelte';
  import GameHUD from './GameHUD.svelte';
  import GameScoreboard from './GameScoreboard.svelte';
  import { Button } from '$lib/components/ui/button';

  interface Props {
    game: GameDetails;
  }

  let { game }: Props = $props();

  let guessLat: number | null = $state(null);
  let guessLng: number | null = $state(null);
  let mapExpanded = $state(false);
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
    if (gameState.hasGuessed) return;

    if (guessLat !== null && guessLng !== null) {
      // User placed a pin - auto-submit their guess
      submitGuess();
    } else if (game.mode === 'solo' && gameState.location) {
      // No pin placed in solo mode - score 0 and transition to results
      gameStore.handleRoundEnd({
        round_number: gameState.currentRound,
        correct_location: gameState.location,
        results: [
          {
            user_id: '',
            display_name: '',
            guess_lat: 0,
            guess_lng: 0,
            distance_meters: -1, // Sentinel value: "no guess"
            score: 0,
            total_score: 0,
          },
        ],
      });
    }
    // Multiplayer: server handles round end via socket event
  }

  function handleKeydown(e: KeyboardEvent) {
    // Space to submit guess (global shortcut)
    if (e.code === 'Space' && canSubmit && !submitting) {
      e.preventDefault();
      submitGuess();
    }
  }

  onMount(() => {
    guessStartTime = Date.now();
  });
</script>

<!-- Full screen game container (fixed to cover entire viewport) -->
<svelte:window onkeydown={handleKeydown} />

<div class="fixed inset-0 overflow-hidden bg-gray-900 z-40">
  <!-- Street View (full screen background) -->
  {#if gameState.location}
    <div class="absolute inset-0 w-full h-full">
      <StreetView
        lat={gameState.location.lat}
        lng={gameState.location.lng}
        panoramaId={gameState.location.panorama_id}
        movementAllowed={game.settings.movement_allowed}
        zoomAllowed={game.settings.zoom_allowed}
      />
    </div>
  {:else}
    <!-- Loading state when no location -->
    <div class="absolute inset-0 flex items-center justify-center">
      <div class="text-center text-white">
        <div class="w-12 h-12 border-4 border-white/30 border-t-white rounded-full animate-spin mx-auto mb-4"></div>
        <p class="text-lg">Loading Street View...</p>
      </div>
    </div>
  {/if}

  <!-- HUD Overlay -->
  <GameHUD>
    <!-- Top Left: Scoreboard (multiplayer only) -->
    {#snippet topLeft()}
      {#if game.mode === 'multiplayer' && gameState.liveScores.length > 0}
        <GameScoreboard scores={gameState.liveScores} currentUserId={$user?.id ?? null} />
      {/if}
    {/snippet}

    <!-- Top Right: Round badge + Timer -->
    {#snippet topRight()}
      <div class="flex items-center gap-3">
        <RoundBadge currentRound={gameState.currentRound} totalRounds={gameState.totalRounds} />

        {#if gameState.timeLimit}
          <div class="bg-background/85 backdrop-blur-md rounded-full p-1.5 border border-border/50 shadow-lg">
            <CircularTimer
              startedAt={gameState.roundStartedAt}
              durationMs={gameState.timeLimit}
              onTimeUp={handleTimeUp}
            />
          </div>
        {/if}

        <!-- Players who have guessed (multiplayer) -->
        {#if game.mode === 'multiplayer'}
          <div class="flex -space-x-2">
            {#each [...gameState.players].slice(0, 5) as [id, player]}
              <div
                class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-medium border-2 border-background shadow-sm transition-all duration-300"
                class:bg-green-500={player.hasGuessed}
                class:text-white={player.hasGuessed}
                class:bg-muted={!player.hasGuessed}
                class:text-muted-foreground={!player.hasGuessed}
                title="{player.displayName} {player.hasGuessed ? '(guessed)' : '(thinking...)'}"
              >
                {player.displayName.charAt(0).toUpperCase()}
              </div>
            {/each}
            {#if gameState.players.size > 5}
              <div
                class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-medium border-2 border-background shadow-sm bg-muted text-muted-foreground"
              >
                +{gameState.players.size - 5}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/snippet}

    <!-- Bottom Right: Interactive Map + Guess Button -->
    {#snippet bottomRight()}
      <div class="flex flex-col items-stretch gap-3">
        <!-- Map Container -->
        <div
          class="transition-all duration-300 ease-out"
          class:w-64={!mapExpanded}
          class:h-48={!mapExpanded}
          class:w-[500px]={mapExpanded}
          class:h-[350px]={mapExpanded}
          class:md:w-[600px]={mapExpanded}
          class:md:h-[400px]={mapExpanded}
        >
          <div
            class="relative w-full h-full bg-background/90 backdrop-blur-sm rounded-xl border border-border/50 shadow-2xl overflow-hidden"
            role="region"
            aria-label="Guess map"
            onmouseenter={() => (mapExpanded = true)}
            onmouseleave={() => !guessLat && (mapExpanded = false)}
          >
            <LeafletMap
              {guessLat}
              {guessLng}
              disabled={gameState.hasGuessed}
              expanded={mapExpanded}
              onclick={handleMapClick}
            />

            <!-- Expand hint (when collapsed and no guess) -->
            {#if !mapExpanded && !guessLat}
              <div class="absolute inset-0 flex items-center justify-center bg-black/20 pointer-events-none">
                <span class="text-white text-sm font-medium drop-shadow-md">
                  Hover to expand
                </span>
              </div>
            {/if}
          </div>
        </div>

        <!-- Guess Button (always visible, disabled until location selected) -->
        {#if !gameState.hasGuessed}
          <Button
            onclick={submitGuess}
            disabled={!canSubmit || submitting}
            size="lg"
            class="w-full shadow-lg gap-2"
          >
            {#if submitting}
              <Loader2 class="w-5 h-5 animate-spin" />
              Guessing...
            {:else}
              <Send class="w-5 h-5" />
              {guessLat !== null ? 'Guess' : 'Place a pin to guess'}
              {#if guessLat !== null}
                <kbd class="ml-2 px-1.5 py-0.5 text-xs bg-primary-foreground/20 rounded">Space</kbd>
              {/if}
            {/if}
          </Button>
        {/if}
      </div>
    {/snippet}

    <!-- Center: Status messages -->
    {#snippet center()}
      {#if gameState.hasGuessed}
        <div class="bg-background/90 backdrop-blur-md px-6 py-4 rounded-xl border border-border/50 shadow-xl text-center">
          <div class="flex items-center justify-center gap-2 mb-1">
            <CheckCircle class="w-5 h-5 text-green-500" />
            <p class="text-lg font-semibold text-foreground">Guess submitted!</p>
          </div>
          <p class="text-sm text-muted-foreground">
            {game.mode === 'multiplayer' ? 'Waiting for other players...' : 'Processing results...'}
          </p>
        </div>
      {/if}
    {/snippet}
  </GameHUD>
</div>

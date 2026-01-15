<script lang="ts">
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { gamesApi, type GameDetails } from '$lib/api/games';
  import { gameStore, initGameSocketListeners } from '$lib/socket/game';

  import GameLoading from '$lib/components/game/GameLoading.svelte';
  import GameLobby from '$lib/components/game/GameLobby.svelte';
  import GamePlay from '$lib/components/game/GamePlay.svelte';
  import GameRoundEnd from '$lib/components/game/GameRoundEnd.svelte';
  import GameFinished from '$lib/components/game/GameFinished.svelte';

  let game: GameDetails | null = $state(null);
  let loading = $state(true);
  let error = $state('');
  let cleanupListeners: (() => void) | null = null;

  // Game ID from route params - guaranteed to exist for [id] route
  let gameId = $derived($page.params.id as string);
  let gameState = $derived($gameStore);

  onMount(async () => {
    if (!gameId) {
      error = 'Invalid game ID';
      loading = false;
      return;
    }

    try {
      // Load game data
      game = await gamesApi.get(gameId);

      // Initialize socket listeners
      cleanupListeners = initGameSocketListeners();

      // Join game room for multiplayer
      if (game.mode === 'multiplayer') {
        gameStore.joinGame(gameId);
      }

      loading = false;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load game';
      loading = false;
    }
  });

  onDestroy(() => {
    cleanupListeners?.();
    gameStore.leaveGame();
  });

  async function startGame() {
    if (!game || !gameId) return;

    try {
      if (game.mode === 'solo') {
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

  async function handleNextRound() {
    if (!game || !gameId) return;

    try {
      const round = await gamesApi.nextRound(gameId);
      gameStore.handleRoundStart({
        round_number: round.round_number,
        total_rounds: game.total_rounds,
        location: round.location,
        time_limit_ms: round.time_limit_ms,
        started_at: Date.now(),
      });
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to start next round';
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
    <GameRoundEnd {game} onNextRound={handleNextRound} />
  {:else if gameState.status === 'finished'}
    <GameFinished {game} />
  {/if}
{/if}

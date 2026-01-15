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

      // Handle based on game mode and status
      if (game.mode === 'multiplayer') {
        // Multiplayer: join game room via socket
        gameStore.joinGame(gameId);
      } else if (game.mode === 'solo') {
        // Solo: restore state based on game status
        await restoreSoloGameState(game);
      }

      loading = false;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load game';
      loading = false;
    }
  });

  /** Restore game state for solo games based on backend status */
  async function restoreSoloGameState(gameDetails: GameDetails) {
    if (gameDetails.status === 'active') {
      // Game is in progress - fetch current round and resume
      try {
        const currentRound = await gamesApi.getCurrentRound(gameDetails.id);
        
        if (currentRound.has_guessed && currentRound.user_guess) {
          // User already guessed this round - show round end screen
          // Get user info from the game details
          const player = gameDetails.players[0]; // Solo game has only one player
          
          gameStore.handleRoundEnd({
            round_number: currentRound.round_number,
            correct_location: currentRound.location,
            results: [{
              user_id: player?.user_id ?? '',
              display_name: player?.display_name ?? 'You',
              guess_lat: currentRound.user_guess.guess_lat,
              guess_lng: currentRound.user_guess.guess_lng,
              distance_meters: currentRound.user_guess.distance_meters,
              score: currentRound.user_guess.score,
              total_score: player?.score ?? currentRound.user_guess.score,
            }],
          });
          
          // Also set round/total info
          gameStore.setRoundInfo(currentRound.round_number, currentRound.total_rounds);
        } else {
          // User hasn't guessed yet - resume playing
          // Calculate the original time limit from remaining time
          const timeLimit = currentRound.time_remaining_ms !== null 
            ? (currentRound.time_remaining_ms + (Date.now() - new Date(currentRound.started_at).getTime()))
            : null;
          
          gameStore.handleRoundStart({
            round_number: currentRound.round_number,
            total_rounds: currentRound.total_rounds,
            location: currentRound.location,
            time_limit_ms: timeLimit !== null ? Math.round(timeLimit) : null,
            started_at: new Date(currentRound.started_at).getTime(),
          });
        }
      } catch (e) {
        console.error('Failed to restore solo game state:', e);
        // If we fail to get current round, show an error or fallback to lobby
        error = 'Failed to resume game. The game may have ended.';
      }
    } else if (gameDetails.status === 'finished') {
      // Game is finished - show finished state
      gameStore.handleGameEnd({
        game_id: gameDetails.id,
        final_standings: gameDetails.players.map((p, i) => ({
          rank: i + 1,
          user_id: p.user_id,
          display_name: p.display_name,
          total_score: p.score,
        })),
      });
    }
    // If status is 'lobby', gameStore defaults to 'idle' which shows the lobby
  }

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

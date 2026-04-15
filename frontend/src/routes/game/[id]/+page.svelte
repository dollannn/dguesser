<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { gamesApi, type GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { socketClient } from '$lib/socket/client';
  import { partyStore } from '$lib/socket/party';
  import { authStore, user } from '$lib/stores/auth';
  import GameLoading from '$lib/components/game/GameLoading.svelte';
  import GameLobby from '$lib/components/game/GameLobby.svelte';
  import GamePlay from '$lib/components/game/GamePlay.svelte';
  import GameRoundEnd from '$lib/components/game/GameRoundEnd.svelte';
  import GameFinished from '$lib/components/game/GameFinished.svelte';
  import SEO from '$lib/components/SEO.svelte';
  import { Spinner } from '$lib/components/ui/spinner';
  import { toast } from 'svelte-sonner';

  let game: GameDetails | null = $state(null);
  let loading = $state(true);
  let error = $state('');
  let autoStarting = $state(false);
  let isStarting = $state(false);
  let startTimeout: ReturnType<typeof setTimeout> | null = null;

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
      // Ensure user has a session (create guest if needed)
      // This allows direct links to multiplayer games to work
      if (!$user) {
        await authStore.createGuest();
      }

      // Load game data
      game = await gamesApi.get(gameId);

      // Handle based on game mode and status
      if (game.mode === 'multiplayer') {
        // Check if user is already a player in the game (e.g., host or rejoining)
        const isExistingPlayer = game.players.some((p) => p.user_id === $user?.id);
        if (isExistingPlayer) {
          // Auto-join socket room to receive realtime updates
          try {
            await gameStore.joinGame(game.id);
          } catch (e) {
            console.error('Failed to join socket room:', e);
          }

          // Party game auto-start: if this game was created by a party,
          // the host auto-starts it so players skip the lobby entirely.
          const partyState = $partyStore;
          const isHost = game.players.find((p) => p.user_id === $user?.id)?.is_host;
          if (partyState.partyId && isHost && game.status === 'lobby') {
            autoStarting = true;
            // Small delay to let socket room join settle
            setTimeout(() => {
              gameStore.startGame();
            }, 500);
          }
        }
        // New players will click "Join Game" button
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

  // Clear starting state when the game transitions out of lobby
  $effect(() => {
    if (gameState.status === 'playing' || gameState.status === 'round_end') {
      isStarting = false;
      if (startTimeout) {
        clearTimeout(startTimeout);
        startTimeout = null;
      }
    }
  });

  // Clear starting state on socket errors
  $effect(() => {
    if (!$page.params.id) return;

    const unsub = socketClient.on<{ code: string; message: string }>('error', (data) => {
      if (
        isStarting &&
        (data.code === 'START_FAILED' ||
          data.code === 'NOT_HOST' ||
          data.code === 'GAME_NOT_FOUND')
      ) {
        isStarting = false;
        if (startTimeout) {
          clearTimeout(startTimeout);
          startTimeout = null;
        }
      }
    });

    return unsub;
  });

  onDestroy(() => {
    gameStore.leaveGame();
    if (startTimeout) clearTimeout(startTimeout);
  });

  async function startGame() {
    if (!game || !gameId || isStarting) return;
    isStarting = true;

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
        // Multiplayer - emit via socket, await round:start event
        const emitted = gameStore.startGame();
        if (!emitted) {
          toast.error('Not connected to game. Please refresh the page.');
          isStarting = false;
          return;
        }
        // Timeout if server never responds with round:start
        startTimeout = setTimeout(() => {
          if (isStarting) {
            isStarting = false;
            toast.error('Failed to start game. Please try again.');
          }
        }, 10000);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to start game';
      isStarting = false;
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

<SEO title="Game" noindex />

{#if loading}
  <GameLoading />
{:else if error}
  <div class="max-w-lg mx-auto mt-12 p-6 bg-red-50 rounded-xl text-center">
    <h2 class="text-xl font-semibold text-red-700 mb-2">Error</h2>
    <p class="text-red-600 mb-4">{error}</p>
    <a href="/" class="inline-block px-4 py-2 rounded-md bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 transition-colors">Back to Home</a>
  </div>
{:else if game}
  {#if autoStarting || (gameState.status === 'idle' || gameState.status === 'lobby') && $partyStore.partyId && $partyStore.status === 'in_game'}
    <!-- Party game: show loading screen while auto-starting -->
    <div class="flex flex-col items-center justify-center h-64 gap-4">
      <Spinner class="size-10 text-primary" />
      <p class="text-lg text-muted-foreground">Starting game...</p>
    </div>
  {:else if gameState.status === 'idle' || gameState.status === 'lobby'}
    <GameLobby {game} onStart={startGame} {isStarting} />
  {:else if gameState.status === 'playing'}
    <GamePlay {game} />
  {:else if gameState.status === 'round_end'}
    <GameRoundEnd {game} onNextRound={handleNextRound} />
  {:else if gameState.status === 'finished'}
    <GameFinished {game} />
  {/if}
{/if}

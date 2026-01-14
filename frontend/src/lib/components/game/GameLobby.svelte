<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { user } from '$lib/stores/auth';
  import { gameStore } from '$lib/socket/game';

  interface Props {
    game: GameDetails;
    onStart: () => void;
  }

  let { game, onStart }: Props = $props();

  let gameState = $derived($gameStore);
  let isHost = $derived(game.players.find((p) => p.user_id === $user?.id)?.is_host ?? false);
  let playerCount = $derived(game.players.length);
  let canStart = $derived(isHost && (game.mode === 'solo' || playerCount >= 2));
</script>

<div class="max-w-2xl mx-auto px-4 py-12">
  <div class="card">
    <div class="text-center mb-8">
      <h1 class="text-3xl font-bold mb-2">
        {game.mode === 'solo' ? 'Solo Game' : 'Multiplayer Lobby'}
      </h1>

      {#if game.mode === 'multiplayer'}
        <div class="mt-4 p-4 bg-gray-100 rounded-lg">
          <p class="text-sm text-gray-600 mb-1">Share this code with friends:</p>
          <p class="text-3xl font-mono font-bold tracking-wider text-primary-600">
            {game.status === 'waiting' ? 'Generating...' : game.id.slice(-6).toUpperCase()}
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
          <span class="font-medium">{game.settings.map_id || 'World'}</span>
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
        <button onclick={onStart} class="btn-accent text-lg px-8 py-3"> Start Game </button>
      {:else if !isHost}
        <p class="text-gray-600">Waiting for host to start...</p>
      {:else}
        <p class="text-gray-600">Need at least 2 players to start</p>
      {/if}
    </div>
  </div>
</div>

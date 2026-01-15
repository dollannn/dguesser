<script lang="ts">
  import type { PlayerScoreInfo } from '$lib/socket/game';
  import { getRankDisplay, getRankClass, formatScore } from '$lib/utils.js';

  interface Props {
    scores: PlayerScoreInfo[];
    currentUserId?: string | null;
    collapsed?: boolean;
  }

  let { scores, currentUserId = null, collapsed = false }: Props = $props();

  let isExpanded = $state(!collapsed);

  // Track previous scores for rank change animations
  let prevScores = $state<Map<string, { rank: number; score: number }>>(new Map());

  // Calculate rank changes when scores update
  let rankChanges = $derived(() => {
    const changes = new Map<string, number>();
    for (const player of scores) {
      const prev = prevScores.get(player.user_id);
      if (prev) {
        const change = prev.rank - player.rank; // Positive = moved up
        if (change !== 0) {
          changes.set(player.user_id, change);
        }
      }
    }
    return changes;
  });

  // Update previous scores after render
  $effect(() => {
    const newMap = new Map<string, { rank: number; score: number }>();
    for (const player of scores) {
      newMap.set(player.user_id, { rank: player.rank, score: player.total_score });
    }
    // Delay update so we can show the change animation
    setTimeout(() => {
      prevScores = newMap;
    }, 2000);
  });
</script>

<div
  class="bg-white/95 backdrop-blur-sm rounded-lg shadow-lg overflow-hidden transition-all duration-300"
  class:w-64={isExpanded}
  class:w-12={!isExpanded}
>
  <!-- Header / Toggle -->
  <button
    onclick={() => (isExpanded = !isExpanded)}
    class="w-full flex items-center gap-2 px-3 py-2.5 bg-gradient-to-r from-gray-50 to-gray-100 hover:from-gray-100 hover:to-gray-150 transition-colors border-b border-gray-200"
  >
    <svg
      class="w-5 h-5 text-gray-600 shrink-0"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="2"
        d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
      />
    </svg>
    {#if isExpanded}
      <span class="text-sm font-semibold text-gray-700">Scoreboard</span>
      <svg
        class="w-4 h-4 ml-auto text-gray-400"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
      </svg>
    {/if}
  </button>

  <!-- Scores list -->
  {#if isExpanded && scores.length > 0}
    <div class="max-h-72 overflow-y-auto">
      <ul class="divide-y divide-gray-100">
        {#each scores as player (player.user_id)}
          {@const isCurrentUser = currentUserId === player.user_id}
          {@const rankChange = rankChanges().get(player.user_id)}
          <li
            class="px-3 py-2.5 flex items-center gap-2 transition-colors {isCurrentUser
              ? 'bg-primary-50'
              : 'hover:bg-gray-50'}"
          >
            <!-- Rank -->
            <div class="w-10 flex items-center justify-center">
              <span class="text-sm font-semibold {getRankClass(player.rank)}">
                {getRankDisplay(player.rank)}
              </span>
            </div>

            <!-- Avatar / Status indicator -->
            <div class="relative shrink-0">
              {#if player.avatar_url}
                <img src={player.avatar_url} alt="" class="w-7 h-7 rounded-full ring-1 ring-gray-200" />
              {:else}
                <div
                  class="w-7 h-7 rounded-full bg-gradient-to-br from-gray-200 to-gray-300 flex items-center justify-center text-xs font-medium text-gray-600"
                >
                  {player.display_name.charAt(0).toUpperCase()}
                </div>
              {/if}

              <!-- Guessed indicator -->
              {#if player.has_guessed}
                <div
                  class="absolute -bottom-0.5 -right-0.5 w-3 h-3 bg-green-500 rounded-full border-2 border-white"
                  title="Has guessed"
                ></div>
              {/if}

              <!-- Disconnected indicator -->
              {#if !player.connected}
                <div class="absolute inset-0 bg-gray-900/40 rounded-full" title="Disconnected"></div>
              {/if}
            </div>

            <!-- Name -->
            <span
              class="flex-1 text-sm truncate {isCurrentUser
                ? 'font-semibold text-primary-700'
                : 'text-gray-700'} {!player.connected ? 'opacity-50' : ''}"
            >
              {player.display_name}
            </span>

            <!-- Score -->
            <div class="text-right min-w-[60px]">
              <div class="flex items-center justify-end gap-1">
                {#if rankChange && rankChange > 0}
                  <span class="text-green-500 text-xs animate-pulse">↑{rankChange}</span>
                {:else if rankChange && rankChange < 0}
                  <span class="text-red-400 text-xs animate-pulse">↓{Math.abs(rankChange)}</span>
                {/if}
                <span class="text-sm font-bold text-gray-900">
                  {formatScore(player.total_score)}
                </span>
              </div>
              {#if player.round_score > 0}
                <div class="text-xs text-green-600 font-medium">
                  +{formatScore(player.round_score)}
                </div>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    </div>
  {:else if isExpanded}
    <div class="px-3 py-6 text-center text-sm text-gray-500">
      <svg class="w-8 h-8 mx-auto mb-2 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197m9 5.197v1" />
      </svg>
      No players yet
    </div>
  {/if}
</div>

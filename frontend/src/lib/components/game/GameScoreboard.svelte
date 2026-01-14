<script lang="ts">
  import type { PlayerScoreInfo } from '$lib/socket/game';

  interface Props {
    scores: PlayerScoreInfo[];
    currentUserId?: string | null;
    collapsed?: boolean;
  }

  let { scores, currentUserId = null, collapsed = false }: Props = $props();

  let isExpanded = $state(!collapsed);

  function getRankIcon(rank: number): string {
    switch (rank) {
      case 1:
        return '1st';
      case 2:
        return '2nd';
      case 3:
        return '3rd';
      default:
        return `#${rank}`;
    }
  }

  function getRankClass(rank: number): string {
    switch (rank) {
      case 1:
        return 'text-yellow-500';
      case 2:
        return 'text-gray-400';
      case 3:
        return 'text-amber-600';
      default:
        return 'text-gray-500';
    }
  }
</script>

<div
  class="bg-white/95 backdrop-blur-sm rounded-lg shadow-lg overflow-hidden transition-all duration-300"
  class:w-56={isExpanded}
  class:w-12={!isExpanded}
>
  <!-- Header / Toggle -->
  <button
    onclick={() => (isExpanded = !isExpanded)}
    class="w-full flex items-center gap-2 px-3 py-2 bg-gray-100 hover:bg-gray-200 transition-colors"
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
    <div class="max-h-64 overflow-y-auto">
      <ul class="divide-y divide-gray-100">
        {#each scores as player}
          {@const isCurrentUser = currentUserId === player.user_id}
          <li
            class="px-3 py-2 flex items-center gap-2 {isCurrentUser
              ? 'bg-primary-50'
              : 'hover:bg-gray-50'}"
          >
            <!-- Rank -->
            <span class="w-8 text-xs font-bold {getRankClass(player.rank)}">
              {getRankIcon(player.rank)}
            </span>

            <!-- Avatar / Status indicator -->
            <div class="relative">
              {#if player.avatar_url}
                <img src={player.avatar_url} alt="" class="w-6 h-6 rounded-full" />
              {:else}
                <div
                  class="w-6 h-6 rounded-full bg-gray-200 flex items-center justify-center text-xs font-medium text-gray-500"
                >
                  {player.display_name.charAt(0).toUpperCase()}
                </div>
              {/if}

              <!-- Guessed indicator -->
              {#if player.has_guessed}
                <div
                  class="absolute -bottom-0.5 -right-0.5 w-3 h-3 bg-green-500 rounded-full border-2 border-white"
                ></div>
              {/if}

              <!-- Disconnected indicator -->
              {#if !player.connected}
                <div class="absolute inset-0 bg-gray-900/40 rounded-full"></div>
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
            <div class="text-right">
              <div class="text-sm font-semibold text-gray-900">
                {player.total_score.toLocaleString()}
              </div>
              {#if player.round_score > 0}
                <div class="text-xs text-green-600">+{player.round_score.toLocaleString()}</div>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    </div>
  {:else if isExpanded}
    <div class="px-3 py-4 text-center text-sm text-gray-500">No players yet</div>
  {/if}
</div>

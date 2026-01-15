<script lang="ts">
  import type { PlayerScoreInfo } from '$lib/socket/game';
  import { getRankDisplay, getRankClass, formatScore } from '$lib/utils.js';
  import { ChevronLeft, Trophy, Users, Wifi, WifiOff } from '@lucide/svelte';
  import * as Avatar from '$lib/components/ui/avatar';

  interface Props {
    scores: PlayerScoreInfo[];
    currentUserId?: string | null;
    collapsed?: boolean;
  }

  let { scores, currentUserId = null, collapsed = false }: Props = $props();

  let isExpanded = $state(true);

  // Sync with collapsed prop
  $effect(() => {
    isExpanded = !collapsed;
  });

  // Track previous scores for rank change animations
  let prevScores = $state<Map<string, { rank: number; score: number }>>(new Map());

  // Calculate rank changes when scores update
  let rankChanges = $derived(() => {
    const changes = new Map<string, number>();
    for (const player of scores) {
      const prev = prevScores.get(player.user_id);
      if (prev) {
        const change = prev.rank - player.rank;
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
    setTimeout(() => {
      prevScores = newMap;
    }, 2000);
  });
</script>

<div
  class="bg-background/85 backdrop-blur-md rounded-xl border border-border/50 shadow-lg overflow-hidden transition-all duration-300"
  class:w-72={isExpanded}
  class:w-12={!isExpanded}
>
  <!-- Header / Toggle -->
  <button
    onclick={() => (isExpanded = !isExpanded)}
    class="w-full flex items-center gap-2.5 px-3 py-2.5 hover:bg-accent/50 transition-colors"
  >
    <Trophy class="w-5 h-5 text-primary shrink-0" />
    {#if isExpanded}
      <span class="text-sm font-semibold text-foreground">Scoreboard</span>
      <span class="ml-auto text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded-full">
        {scores.length}
      </span>
      <ChevronLeft class="w-4 h-4 text-muted-foreground" />
    {/if}
  </button>

  <!-- Scores list -->
  {#if isExpanded && scores.length > 0}
    <div class="max-h-80 overflow-y-auto border-t border-border/50">
      <ul class="divide-y divide-border/30">
        {#each scores as player (player.user_id)}
          {@const isCurrentUser = currentUserId === player.user_id}
          {@const rankChange = rankChanges().get(player.user_id)}
          <li
            class="px-3 py-2.5 flex items-center gap-2.5 transition-colors {isCurrentUser
              ? 'bg-primary/10'
              : 'hover:bg-accent/30'}"
          >
            <!-- Rank Badge -->
            <div class="w-8 flex items-center justify-center shrink-0">
              <span
                class="text-sm font-bold {getRankClass(player.rank)} {player.rank <= 3
                  ? 'text-base'
                  : ''}"
              >
                {getRankDisplay(player.rank)}
              </span>
            </div>

            <!-- Avatar with status indicator -->
            <div class="relative shrink-0">
              <Avatar.Root class="w-8 h-8 ring-2 ring-background">
                {#if player.avatar_url}
                  <Avatar.Image src={player.avatar_url} alt={player.display_name} />
                {/if}
                <Avatar.Fallback
                  class="bg-gradient-to-br from-primary/20 to-primary/40 text-xs font-semibold"
                >
                  {player.display_name.charAt(0).toUpperCase()}
                </Avatar.Fallback>
              </Avatar.Root>

              <!-- Guessed indicator -->
              {#if player.has_guessed}
                <div
                  class="absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 bg-green-500 rounded-full border-2 border-background flex items-center justify-center"
                  title="Has guessed"
                >
                  <svg class="w-2 h-2 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                    <polyline points="20 6 9 17 4 12"></polyline>
                  </svg>
                </div>
              {/if}

              <!-- Disconnected overlay -->
              {#if !player.connected}
                <div
                  class="absolute inset-0 bg-background/60 rounded-full flex items-center justify-center"
                  title="Disconnected"
                >
                  <WifiOff class="w-3 h-3 text-muted-foreground" />
                </div>
              {/if}
            </div>

            <!-- Name -->
            <span
              class="flex-1 text-sm truncate {isCurrentUser
                ? 'font-semibold text-primary'
                : 'text-foreground'} {!player.connected ? 'opacity-60' : ''}"
            >
              {player.display_name}
              {#if isCurrentUser}
                <span class="text-xs text-muted-foreground ml-1">(you)</span>
              {/if}
            </span>

            <!-- Score -->
            <div class="text-right min-w-[60px] shrink-0">
              <div class="flex items-center justify-end gap-1">
                {#if rankChange && rankChange > 0}
                  <span class="text-green-500 text-xs font-medium animate-pulse">
                    +{rankChange}
                  </span>
                {:else if rankChange && rankChange < 0}
                  <span class="text-red-400 text-xs font-medium animate-pulse">
                    {rankChange}
                  </span>
                {/if}
                <span class="text-sm font-bold text-foreground">
                  {formatScore(player.total_score)}
                </span>
              </div>
              {#if player.round_score > 0}
                <div class="text-xs text-green-600 dark:text-green-400 font-medium">
                  +{formatScore(player.round_score)}
                </div>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    </div>
  {:else if isExpanded}
    <div class="px-4 py-8 text-center border-t border-border/50">
      <Users class="w-10 h-10 mx-auto mb-2 text-muted-foreground/50" />
      <p class="text-sm text-muted-foreground">No players yet</p>
    </div>
  {/if}
</div>

<script lang="ts">
  import { user } from '$lib/stores/auth';
  import {
    leaderboardApi,
    type LeaderboardType,
    type TimePeriod,
    type LeaderboardEntry,
  } from '$lib/api';
  import {
    getRankDisplay,
    getRankClass,
    getRankRowClass,
    formatScore,
  } from '$lib/utils.js';
  import SEO from '$lib/components/SEO.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // Initialize from server data
  let entries = $state<LeaderboardEntry[]>(data.initialEntries as LeaderboardEntry[]);
  let loading = $state(false);
  let loadingMore = $state(false);
  let error = $state('');
  let totalPlayers = $state(data.totalPlayers);
  let currentUserRank = $state<number | null>(data.currentUserRank);
  let currentUserScore = $state<number | null>(data.currentUserScore);
  let offset = $state(0);
  let hasMore = $state((data.initialEntries?.length ?? 0) < data.totalPlayers);

  let selectedType = $state<LeaderboardType>('total_score');
  let selectedPeriod = $state<TimePeriod>('all_time');

  const LIMIT = 50;

  const typeOptions: { value: LeaderboardType; label: string }[] = [
    { value: 'total_score', label: 'Total Score' },
    { value: 'best_game', label: 'Best Game' },
    { value: 'games_played', label: 'Games Played' },
    { value: 'average_score', label: 'Average' },
  ];

  const periodOptions: { value: TimePeriod; label: string }[] = [
    { value: 'all_time', label: 'All Time' },
    { value: 'daily', label: 'Today' },
    { value: 'weekly', label: 'This Week' },
    { value: 'monthly', label: 'This Month' },
  ];

  async function loadLeaderboard(append = false) {
    if (append) {
      loadingMore = true;
    } else {
      loading = true;
      offset = 0;
      // Don't clear entries here - keep showing old data until new data arrives
    }
    error = '';

    try {
      const response = await leaderboardApi.getLeaderboard({
        type: selectedType,
        period: selectedPeriod,
        limit: LIMIT,
        offset: append ? offset : 0,
      });

      if (append) {
        entries = [...entries, ...response.entries];
      } else {
        entries = response.entries;
      }
      totalPlayers = response.total_players;
      currentUserRank = response.current_user_rank;
      currentUserScore = response.current_user_score;
      hasMore = entries.length < totalPlayers;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load leaderboard';
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  function loadMore() {
    offset += LIMIT;
    loadLeaderboard(true);
  }

  function getScoreLabel(type: LeaderboardType): string {
    switch (type) {
      case 'total_score':
        return 'Total Score';
      case 'best_game':
        return 'Best Game';
      case 'games_played':
        return 'Games';
      case 'average_score':
        return 'Average';
    }
  }

  function formatDisplayScore(score: number, type: LeaderboardType): string {
    if (type === 'games_played') {
      return score.toString();
    }
    return formatScore(score);
  }

  // Show skeleton only on initial load, not on filter changes
  let hasLoadedOnce = $state(true); // Already loaded via SSR
  let showSkeleton = $derived(loading && !hasLoadedOnce);

  // Track previous filter values to detect changes
  let prevType = selectedType;
  let prevPeriod = selectedPeriod;

  // Load on filter change (skip initial since we have SSR data)
  $effect(() => {
    // Track dependencies
    const currentType = selectedType;
    const currentPeriod = selectedPeriod;

    // Only reload if filters changed
    if (currentType !== prevType || currentPeriod !== prevPeriod) {
      prevType = currentType;
      prevPeriod = currentPeriod;
      loadLeaderboard();
    }
  });
</script>

<SEO
  title="Leaderboard"
  description="See how you rank against other DGuesser players. View top scores by total points, best game, games played, and average score."
/>

<div class="max-w-4xl mx-auto px-4 py-8">
  <!-- Header -->
  <div class="mb-8">
    <h1 class="text-4xl font-bold text-gray-900 mb-2">Leaderboard</h1>
    <p class="text-gray-600">See how you stack up against other players</p>
  </div>

  <!-- Filters -->
  <div class="flex flex-col sm:flex-row gap-4 mb-6">
    <!-- Type selector -->
    <div class="flex flex-wrap gap-2">
      {#each typeOptions as option}
        <button
          onclick={() => (selectedType = option.value)}
          class="px-3 py-2 rounded-lg text-sm font-medium transition-all {selectedType === option.value
            ? 'bg-gray-900 text-white shadow-md'
            : 'bg-gray-100 text-gray-700 hover:bg-gray-200'}"
        >
          {option.label}
        </button>
      {/each}
    </div>

    <!-- Period selector -->
    <div class="flex gap-2 sm:ml-auto">
      {#each periodOptions as option}
        <button
          onclick={() => (selectedPeriod = option.value)}
          class="px-3 py-2 rounded-lg text-sm font-medium transition-all {selectedPeriod === option.value
            ? 'bg-gray-900 text-white shadow-md'
            : 'bg-gray-100 text-gray-700 hover:bg-gray-200'}"
        >
          {option.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Current user rank (when not in top entries) -->
  {#if currentUserRank && !entries.some((e) => e.is_current_user)}
    <div class="mb-6 p-4 bg-primary-50 rounded-xl border border-primary-200">
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm text-primary-600 font-medium">Your Rank</span>
          <div class="text-2xl font-bold text-primary-800">
            {getRankDisplay(currentUserRank)}
          </div>
        </div>
        {#if currentUserScore !== null}
          <div class="text-right">
            <span class="text-sm text-primary-600 font-medium">{getScoreLabel(selectedType)}</span>
            <div class="text-2xl font-bold text-primary-800">
              {formatDisplayScore(currentUserScore, selectedType)}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Error -->
  {#if error}
    <div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-xl text-red-700 flex items-center gap-3">
      <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      {error}
    </div>
  {/if}

  <!-- Loading skeleton (initial load or loading with no data) -->
  {#if showSkeleton || (loading && entries.length === 0)}
    <div class="bg-white rounded-xl shadow overflow-hidden">
      <div class="animate-pulse">
        <div class="h-12 bg-gray-100 border-b"></div>
        {#each Array(10) as _}
          <div class="flex items-center gap-4 px-6 py-4 border-b border-gray-100">
            <div class="w-12 h-6 bg-gray-200 rounded"></div>
            <div class="w-10 h-10 bg-gray-200 rounded-full"></div>
            <div class="flex-1 h-4 bg-gray-200 rounded w-32"></div>
            <div class="w-20 h-4 bg-gray-200 rounded"></div>
            <div class="w-12 h-4 bg-gray-200 rounded"></div>
          </div>
        {/each}
      </div>
    </div>
  {:else if entries.length === 0}
    <div class="text-center py-16 bg-white rounded-xl shadow">
      <svg class="w-16 h-16 mx-auto text-gray-300 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z" />
      </svg>
      <p class="text-lg text-gray-700 font-medium">No players on the leaderboard yet</p>
      <p class="text-sm text-gray-500 mt-2">Be the first to play and claim your spot!</p>
      <a href="/play" class="btn-primary mt-6 inline-block">Start Playing</a>
    </div>
  {:else}
    <!-- Leaderboard table -->
    <div class="bg-white rounded-xl shadow overflow-hidden {loading ? 'opacity-60' : ''}">
      <table class="w-full">
        <thead>
          <tr class="bg-gray-50 border-b border-gray-200">
            <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-600 w-20">
              Rank
            </th>
            <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-600">
              Player
            </th>
            <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-600">
              {getScoreLabel(selectedType)}
            </th>
            <th class="hidden sm:table-cell px-6 py-4 text-right text-sm font-semibold text-gray-600">
              Games
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each entries as entry (entry.user_id + entry.rank)}
            <tr
              class="transition-colors hover:bg-gray-50 {getRankRowClass(entry.rank, entry.is_current_user)}"
            >
              <td class="px-4 sm:px-6 py-4">
                <span class="text-lg font-semibold {getRankClass(entry.rank)}">
                  {getRankDisplay(entry.rank)}
                </span>
              </td>
              <td class="px-4 sm:px-6 py-4">
                <div class="flex items-center gap-3">
                  {#if entry.avatar_url}
                    <img
                      src={entry.avatar_url}
                      alt=""
                      class="w-10 h-10 rounded-full ring-2 ring-white shadow"
                    />
                  {:else}
                    <div
                      class="w-10 h-10 rounded-full bg-gradient-to-br from-gray-200 to-gray-300 flex items-center justify-center ring-2 ring-white shadow"
                    >
                      <span class="text-gray-600 text-sm font-bold">
                        {entry.display_name.charAt(0).toUpperCase()}
                      </span>
                    </div>
                  {/if}
                  <div>
                    <span
                      class="font-medium {entry.is_current_user
                        ? 'text-primary-700'
                        : 'text-gray-900'}"
                    >
                      {entry.display_name}
                    </span>
                    {#if entry.is_current_user}
                      <span class="ml-1.5 text-xs bg-primary-100 text-primary-700 px-2 py-0.5 rounded-full font-medium">
                        You
                      </span>
                    {/if}
                    <!-- Show games on mobile under the name -->
                    <div class="sm:hidden text-xs text-gray-500 mt-0.5">
                      {entry.games_played} games
                    </div>
                  </div>
                </div>
              </td>
              <td class="px-4 sm:px-6 py-4 text-right">
                <span class="font-bold text-gray-900 text-lg">
                  {formatDisplayScore(entry.score, selectedType)}
                </span>
              </td>
              <td class="hidden sm:table-cell px-6 py-4 text-right text-gray-600">
                {entry.games_played}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Load more / Stats -->
    <div class="mt-6 flex flex-col items-center gap-4">
      {#if hasMore}
        <button
          onclick={loadMore}
          disabled={loadingMore}
          class="btn-secondary px-6 py-2.5 {loadingMore ? 'opacity-50 cursor-not-allowed' : ''}"
        >
          {#if loadingMore}
            <svg class="inline-block w-4 h-4 mr-2 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Loading...
          {:else}
            Load More
          {/if}
        </button>
      {/if}
      
      <p class="text-sm text-gray-500">
        Showing {entries.length} of {totalPlayers.toLocaleString()} players
      </p>
    </div>
  {/if}
</div>

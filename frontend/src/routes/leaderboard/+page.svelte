<script lang="ts">
  import { onMount } from 'svelte';
  import { user } from '$lib/stores/auth';
  import {
    leaderboardApi,
    type LeaderboardType,
    type TimePeriod,
    type LeaderboardEntry,
  } from '$lib/api';

  let entries = $state<LeaderboardEntry[]>([]);
  let loading = $state(true);
  let error = $state('');
  let totalPlayers = $state(0);
  let currentUserRank = $state<number | null>(null);
  let currentUserScore = $state<number | null>(null);

  let selectedType = $state<LeaderboardType>('total_score');
  let selectedPeriod = $state<TimePeriod>('all_time');

  const typeLabels: Record<LeaderboardType, string> = {
    total_score: 'Total Score',
    best_game: 'Best Game',
    games_played: 'Games Played',
    average_score: 'Average Score',
  };

  const periodLabels: Record<TimePeriod, string> = {
    all_time: 'All Time',
    daily: 'Today',
    weekly: 'This Week',
    monthly: 'This Month',
  };

  async function loadLeaderboard() {
    loading = true;
    error = '';

    try {
      const response = await leaderboardApi.getLeaderboard({
        type: selectedType,
        period: selectedPeriod,
        limit: 100,
      });

      entries = response.entries;
      totalPlayers = response.total_players;
      currentUserRank = response.current_user_rank;
      currentUserScore = response.current_user_score;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load leaderboard';
    } finally {
      loading = false;
    }
  }

  function formatScore(score: number, type: LeaderboardType): string {
    if (type === 'games_played') {
      return score.toString();
    }
    return score.toLocaleString();
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
        return 'text-yellow-500 font-bold';
      case 2:
        return 'text-gray-400 font-bold';
      case 3:
        return 'text-amber-600 font-bold';
      default:
        return 'text-gray-500';
    }
  }

  onMount(() => {
    loadLeaderboard();
  });

  // Reload when filters change
  $effect(() => {
    // Track dependencies
    const _ = [selectedType, selectedPeriod];
    loadLeaderboard();
  });
</script>

<div class="max-w-4xl mx-auto px-4 py-8">
  <div class="mb-8">
    <h1 class="text-4xl font-bold text-gray-900 mb-2">Leaderboard</h1>
    <p class="text-gray-600">See how you stack up against other players</p>
  </div>

  <!-- Filters -->
  <div class="flex flex-wrap gap-4 mb-6">
    <!-- Type selector -->
    <div class="flex gap-2">
      {#each Object.entries(typeLabels) as [value, label]}
        <button
          onclick={() => (selectedType = value as LeaderboardType)}
          class="px-4 py-2 rounded-lg text-sm font-medium transition-colors {selectedType ===
          value
            ? 'bg-primary-600 text-white'
            : 'bg-gray-100 text-gray-700 hover:bg-gray-200'}"
        >
          {label}
        </button>
      {/each}
    </div>

    <!-- Period selector -->
    <div class="flex gap-2 ml-auto">
      {#each Object.entries(periodLabels) as [value, label]}
        <button
          onclick={() => (selectedPeriod = value as TimePeriod)}
          class="px-3 py-2 rounded-lg text-sm font-medium transition-colors {selectedPeriod ===
          value
            ? 'bg-accent-600 text-white'
            : 'bg-gray-100 text-gray-700 hover:bg-gray-200'}"
        >
          {label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Current user rank -->
  {#if currentUserRank && !entries.some((e) => e.is_current_user)}
    <div class="mb-6 p-4 bg-primary-50 rounded-lg border border-primary-200">
      <div class="flex items-center justify-between">
        <div>
          <span class="text-sm text-primary-600 font-medium">Your Rank</span>
          <div class="text-2xl font-bold text-primary-800">
            {getRankIcon(currentUserRank)}
          </div>
        </div>
        {#if currentUserScore !== null}
          <div class="text-right">
            <span class="text-sm text-primary-600 font-medium"
              >{getScoreLabel(selectedType)}</span
            >
            <div class="text-2xl font-bold text-primary-800">
              {formatScore(currentUserScore, selectedType)}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Error -->
  {#if error}
    <div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700">
      {error}
    </div>
  {/if}

  <!-- Loading -->
  {#if loading}
    <div class="flex justify-center items-center py-12">
      <div
        class="animate-spin rounded-full h-8 w-8 border-2 border-primary-500 border-t-transparent"
      ></div>
    </div>
  {:else if entries.length === 0}
    <div class="text-center py-12 text-gray-500">
      <p class="text-lg">No players on the leaderboard yet.</p>
      <p class="text-sm mt-2">Be the first to play!</p>
    </div>
  {:else}
    <!-- Leaderboard table -->
    <div class="bg-white rounded-xl shadow overflow-hidden">
      <table class="w-full">
        <thead>
          <tr class="bg-gray-50 border-b border-gray-200">
            <th class="px-6 py-4 text-left text-sm font-semibold text-gray-600"
              >Rank</th
            >
            <th class="px-6 py-4 text-left text-sm font-semibold text-gray-600"
              >Player</th
            >
            <th class="px-6 py-4 text-right text-sm font-semibold text-gray-600"
              >{getScoreLabel(selectedType)}</th
            >
            <th class="px-6 py-4 text-right text-sm font-semibold text-gray-600"
              >Games</th
            >
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each entries as entry}
            <tr
              class="hover:bg-gray-50 transition-colors {entry.is_current_user
                ? 'bg-primary-50 ring-2 ring-primary-200 ring-inset'
                : ''}"
            >
              <td class="px-6 py-4">
                <span class={getRankClass(entry.rank)}>
                  {getRankIcon(entry.rank)}
                </span>
              </td>
              <td class="px-6 py-4">
                <div class="flex items-center gap-3">
                  {#if entry.avatar_url}
                    <img
                      src={entry.avatar_url}
                      alt=""
                      class="w-8 h-8 rounded-full"
                    />
                  {:else}
                    <div
                      class="w-8 h-8 rounded-full bg-gray-200 flex items-center justify-center"
                    >
                      <span class="text-gray-500 text-sm font-medium">
                        {entry.display_name.charAt(0).toUpperCase()}
                      </span>
                    </div>
                  {/if}
                  <span
                    class="font-medium {entry.is_current_user
                      ? 'text-primary-700'
                      : 'text-gray-900'}"
                  >
                    {entry.display_name}
                    {#if entry.is_current_user}
                      <span class="text-xs text-primary-600 ml-1">(You)</span>
                    {/if}
                  </span>
                </div>
              </td>
              <td class="px-6 py-4 text-right font-semibold text-gray-900">
                {formatScore(entry.score, selectedType)}
              </td>
              <td class="px-6 py-4 text-right text-gray-600">
                {entry.games_played}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Total players -->
    <div class="mt-4 text-center text-sm text-gray-500">
      Showing top {entries.length} of {totalPlayers.toLocaleString()} players
    </div>
  {/if}
</div>

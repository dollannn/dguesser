<script lang="ts">
  import { goto } from '$app/navigation';
  import { user } from '$lib/stores/auth';
  import { gamesApi, type GameSummary } from '$lib/api/games';
  import { formatScore } from '$lib/utils.js';
  import SEO from '$lib/components/SEO.svelte';

  type ModeFilter = 'all' | 'solo' | 'multiplayer';

  let games = $state<GameSummary[]>([]);
  let loading = $state(true);
  let error = $state('');
  let selectedMode = $state<ModeFilter>('all');

  const modeOptions: { value: ModeFilter; label: string }[] = [
    { value: 'all', label: 'All Games' },
    { value: 'solo', label: 'Solo' },
    { value: 'multiplayer', label: 'Multiplayer' },
  ];

  // Filter games by selected mode (client-side)
  let filteredGames = $derived(
    selectedMode === 'all' ? games : games.filter((g) => g.mode === selectedMode)
  );

  async function loadHistory() {
    loading = true;
    error = '';

    try {
      games = await gamesApi.getHistory();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load game history';
    } finally {
      loading = false;
    }
  }

  function formatDate(isoString: string): string {
    const date = new Date(isoString);
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }

  function formatTime(isoString: string): string {
    const date = new Date(isoString);
    return date.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function getModeLabel(mode: string): string {
    switch (mode) {
      case 'solo':
        return 'Solo';
      case 'multiplayer':
        return 'Multiplayer';
      case 'challenge':
        return 'Challenge';
      default:
        return mode;
    }
  }

  function getModeClass(mode: string): string {
    switch (mode) {
      case 'solo':
        return 'bg-blue-100 text-blue-700';
      case 'multiplayer':
        return 'bg-purple-100 text-purple-700';
      case 'challenge':
        return 'bg-amber-100 text-amber-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  }

  function getStatusLabel(status: string): string {
    switch (status) {
      case 'finished':
        return 'Finished';
      case 'abandoned':
        return 'Abandoned';
      case 'active':
        return 'In Progress';
      default:
        return status;
    }
  }

  function getStatusClass(status: string): string {
    switch (status) {
      case 'finished':
        return 'bg-green-100 text-green-700';
      case 'abandoned':
        return 'bg-red-100 text-red-700';
      case 'active':
        return 'bg-yellow-100 text-yellow-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  }

  function viewGame(gameId: string) {
    goto(`/game/${gameId}`);
  }

  // Load history when user is authenticated
  $effect(() => {
    if ($user) {
      loadHistory();
    } else {
      loading = false;
    }
  });
</script>

<SEO title="Game History" noindex />

<div class="max-w-4xl mx-auto px-4 py-8">
  <!-- Header -->
  <div class="mb-8">
    <h1 class="text-4xl font-bold text-gray-900 mb-2">Game History</h1>
    <p class="text-gray-600">Review your past games and scores</p>
  </div>

  <!-- Not authenticated -->
  {#if !$user}
    <div class="text-center py-16 bg-white rounded-xl shadow">
      <svg
        class="w-16 h-16 mx-auto text-gray-300 mb-4"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.5"
          d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
        />
      </svg>
      <p class="text-lg text-gray-700 font-medium">Sign in to view your game history</p>
      <p class="text-sm text-gray-500 mt-2">
        Your game history is saved when you're logged in
      </p>
      <a href="/auth" class="btn-primary mt-6 inline-block">Sign In</a>
    </div>
  {:else}
    <!-- Mode filter -->
    <div class="flex flex-wrap gap-2 mb-6">
      {#each modeOptions as option}
        <button
          onclick={() => (selectedMode = option.value)}
          class="px-4 py-2 rounded-lg text-sm font-medium transition-all {selectedMode ===
          option.value
            ? 'bg-gray-900 text-white shadow-md'
            : 'bg-gray-100 text-gray-700 hover:bg-gray-200'}"
        >
          {option.label}
        </button>
      {/each}
    </div>

    <!-- Error -->
    {#if error}
      <div
        class="mb-6 p-4 bg-red-50 border border-red-200 rounded-xl text-red-700 flex items-center gap-3"
      >
        <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        {error}
      </div>
    {/if}

    <!-- Loading skeleton -->
    {#if loading}
      <div class="bg-white rounded-xl shadow overflow-hidden">
        <div class="animate-pulse">
          <div class="h-12 bg-gray-100 border-b"></div>
          {#each Array(5) as _}
            <div class="flex items-center gap-4 px-6 py-4 border-b border-gray-100">
              <div class="w-24 h-4 bg-gray-200 rounded"></div>
              <div class="w-20 h-6 bg-gray-200 rounded-full"></div>
              <div class="flex-1"></div>
              <div class="w-16 h-4 bg-gray-200 rounded"></div>
              <div class="w-20 h-6 bg-gray-200 rounded-full"></div>
              <div class="w-6 h-6 bg-gray-200 rounded"></div>
            </div>
          {/each}
        </div>
      </div>
    {:else if filteredGames.length === 0}
      <!-- Empty state -->
      <div class="text-center py-16 bg-white rounded-xl shadow">
        <svg
          class="w-16 h-16 mx-auto text-gray-300 mb-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
          />
        </svg>
        {#if selectedMode === 'all'}
          <p class="text-lg text-gray-700 font-medium">No games played yet</p>
          <p class="text-sm text-gray-500 mt-2">Start playing to build your history!</p>
          <a href="/play" class="btn-primary mt-6 inline-block">Play Now</a>
        {:else}
          <p class="text-lg text-gray-700 font-medium">
            No {getModeLabel(selectedMode).toLowerCase()} games found
          </p>
          <p class="text-sm text-gray-500 mt-2">Try a different filter or play more games</p>
          <button onclick={() => (selectedMode = 'all')} class="btn-secondary mt-6">
            Show All Games
          </button>
        {/if}
      </div>
    {:else}
      <!-- Games table -->
      <div class="bg-white rounded-xl shadow overflow-hidden">
        <table class="w-full">
          <thead>
            <tr class="bg-gray-50 border-b border-gray-200">
              <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-600">
                Date
              </th>
              <th class="px-4 sm:px-6 py-4 text-left text-sm font-semibold text-gray-600">
                Mode
              </th>
              <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-600">
                Score
              </th>
              <th
                class="hidden sm:table-cell px-6 py-4 text-center text-sm font-semibold text-gray-600"
              >
                Status
              </th>
              <th class="px-4 sm:px-6 py-4 text-right text-sm font-semibold text-gray-600">
                <span class="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-100">
            {#each filteredGames as game (game.id)}
              <tr
                class="transition-colors hover:bg-gray-50 cursor-pointer"
                onclick={() => viewGame(game.id)}
                onkeydown={(e) => e.key === 'Enter' && viewGame(game.id)}
                tabindex="0"
                role="button"
              >
                <td class="px-4 sm:px-6 py-4">
                  <div class="text-gray-900 font-medium">
                    {formatDate(game.played_at)}
                  </div>
                  <div class="text-gray-500 text-sm">
                    {formatTime(game.played_at)}
                  </div>
                </td>
                <td class="px-4 sm:px-6 py-4">
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium {getModeClass(
                      game.mode
                    )}"
                  >
                    {getModeLabel(game.mode)}
                  </span>
                </td>
                <td class="px-4 sm:px-6 py-4 text-right">
                  <span class="font-bold text-gray-900 text-lg">
                    {formatScore(game.score)}
                  </span>
                </td>
                <td class="hidden sm:table-cell px-6 py-4 text-center">
                  <span
                    class="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium {getStatusClass(
                      game.status
                    )}"
                  >
                    {getStatusLabel(game.status)}
                  </span>
                </td>
                <td class="px-4 sm:px-6 py-4 text-right">
                  <svg
                    class="w-5 h-5 text-gray-400 inline-block"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M9 5l7 7-7 7"
                    />
                  </svg>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Stats footer -->
      <div class="mt-6 text-center">
        <p class="text-sm text-gray-500">
          Showing {filteredGames.length}
          {filteredGames.length === 1 ? 'game' : 'games'}
          {#if selectedMode !== 'all'}
            ({games.length} total)
          {/if}
        </p>
      </div>
    {/if}
  {/if}
</div>

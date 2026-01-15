<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { user } from '$lib/stores/auth';
  import { getRankDisplay, getRankClass, formatScore, formatDistance } from '$lib/utils.js';
  import ResultsMap from './ResultsMap.svelte';

  interface Props {
    game: GameDetails;
    onNextRound?: () => void;
  }

  let { game, onNextRound }: Props = $props();

  let state = $derived($gameStore);
  let results = $derived(state.results);
  let correctLocation = $derived(state.location);
  
  // Sort results by score (highest first) and assign ranks
  let rankedResults = $derived(() => {
    return results
      .toSorted((a, b) => b.score - a.score)
      .map((result, index) => ({
        ...result,
        rank: index + 1,
        isCurrentUser: result.user_id === $user?.id,
      }));
  });
</script>

<div class="min-h-screen bg-gray-50 p-4">
  <div class="max-w-6xl mx-auto">
    <!-- Header -->
    <div class="text-center mb-6">
      <h2 class="text-3xl font-bold text-gray-900">
        Round {state.currentRound} Results
      </h2>
      <p class="text-gray-600 mt-1">
        {state.currentRound} of {state.totalRounds} rounds complete
      </p>
    </div>

    <!-- Results map showing all guesses -->
    <div class="h-[400px] rounded-xl overflow-hidden shadow-lg mb-6 ring-1 ring-gray-200">
      {#if correctLocation}
        <ResultsMap
          correctLat={correctLocation.lat}
          correctLng={correctLocation.lng}
          guesses={results.map((r) => ({
            lat: r.guess_lat,
            lng: r.guess_lng,
            displayName: r.display_name,
          }))}
        />
      {/if}
    </div>

    <!-- Results table -->
    <div class="bg-white rounded-xl shadow-lg overflow-hidden">
      <table class="w-full">
        <thead class="bg-gradient-to-r from-gray-50 to-gray-100 border-b">
          <tr>
            <th class="text-left py-4 px-4 sm:px-6 font-semibold text-gray-700 w-16">Rank</th>
            <th class="text-left py-4 px-4 sm:px-6 font-semibold text-gray-700">Player</th>
            <th class="text-right py-4 px-4 sm:px-6 font-semibold text-gray-700">Distance</th>
            <th class="text-right py-4 px-4 sm:px-6 font-semibold text-gray-700">Score</th>
            <th class="hidden sm:table-cell text-right py-4 px-6 font-semibold text-gray-700">Total</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          {#each rankedResults() as result (result.user_id)}
            {@const bgClass = result.rank === 1 ? 'bg-yellow-50' : result.rank === 2 ? 'bg-gray-50' : result.rank === 3 ? 'bg-amber-50' : ''}
            <tr class="transition-colors {bgClass} {result.isCurrentUser ? 'ring-2 ring-primary-500 ring-inset' : ''}">
              <td class="py-4 px-4 sm:px-6">
                <span class="text-lg font-semibold {getRankClass(result.rank)}">
                  {getRankDisplay(result.rank)}
                </span>
              </td>
              <td class="py-4 px-4 sm:px-6">
                <div class="flex items-center gap-2">
                  <span class="font-medium {result.isCurrentUser ? 'text-primary-700' : 'text-gray-900'}">
                    {result.display_name || 'You'}
                  </span>
                  {#if result.isCurrentUser}
                    <span class="text-xs bg-primary-100 text-primary-700 px-2 py-0.5 rounded-full font-medium">
                      You
                    </span>
                  {/if}
                </div>
                <!-- Show total on mobile under the name -->
                <div class="sm:hidden text-xs text-gray-500 mt-1">
                  Total: {formatScore(result.total_score)}
                </div>
              </td>
              <td class="text-right py-4 px-4 sm:px-6 text-gray-600">
                {formatDistance(result.distance_meters)}
              </td>
              <td class="text-right py-4 px-4 sm:px-6">
                <span class="font-bold text-green-600">
                  +{formatScore(result.score)}
                </span>
              </td>
              <td class="hidden sm:table-cell text-right py-4 px-6 font-bold text-gray-900">
                {formatScore(result.total_score)}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Continue prompt -->
    {#if state.currentRound < state.totalRounds}
      <div class="text-center mt-8">
        {#if game.mode === 'solo' && onNextRound}
          <button onclick={onNextRound} class="btn-primary px-8 py-3 text-lg">
            Continue to Round {state.currentRound + 1}
            <svg class="inline-block w-5 h-5 ml-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
            </svg>
          </button>
        {:else}
          <div class="inline-flex items-center gap-3 px-6 py-4 bg-white rounded-xl shadow">
            <div class="animate-spin w-5 h-5 border-2 border-primary-500 border-t-transparent rounded-full"></div>
            <span class="text-gray-700 font-medium">Next round starting soon...</span>
          </div>
        {/if}
      </div>
    {:else}
      <div class="text-center mt-8">
        <div class="inline-flex items-center gap-3 px-6 py-4 bg-primary-50 rounded-xl border border-primary-200">
          <svg class="w-6 h-6 text-primary-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 21v-4m0 0V5a2 2 0 012-2h6.5l1 1H21l-3 6 3 6h-8.5l-1-1H5a2 2 0 00-2 2zm9-13.5V9" />
          </svg>
          <span class="text-primary-700 font-medium">Final results coming up...</span>
        </div>
      </div>
    {/if}
  </div>
</div>

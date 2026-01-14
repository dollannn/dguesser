<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { gamesApi } from '$lib/api/games';
  import ResultsMap from './ResultsMap.svelte';

  interface Props {
    game: GameDetails;
    onNextRound?: () => void;
  }

  let { game, onNextRound }: Props = $props();

  let state = $derived($gameStore);
  let results = $derived(state.results);
  let correctLocation = $derived(state.location);

  function formatDistance(meters: number): string {
    if (meters < 1000) {
      return `${Math.round(meters)} m`;
    }
    return `${(meters / 1000).toFixed(1)} km`;
  }
</script>

<div class="min-h-screen bg-gray-50 p-4">
  <div class="max-w-6xl mx-auto">
    <div class="text-center mb-6">
      <h2 class="text-2xl font-bold">Round {state.currentRound} Results</h2>
    </div>

    <!-- Results map showing all guesses -->
    <div class="h-[400px] rounded-xl overflow-hidden shadow-lg mb-6">
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
    <div class="card">
      <table class="w-full">
        <thead>
          <tr class="border-b">
            <th class="text-left py-3 px-4">Player</th>
            <th class="text-right py-3 px-4">Distance</th>
            <th class="text-right py-3 px-4">Score</th>
            <th class="text-right py-3 px-4">Total</th>
          </tr>
        </thead>
        <tbody>
          {#each results.toSorted((a, b) => b.score - a.score) as result}
            <tr class="border-b last:border-0">
              <td class="py-3 px-4">
                <span class="font-medium">{result.display_name || 'You'}</span>
              </td>
              <td class="text-right py-3 px-4 text-gray-600">
                {formatDistance(result.distance_meters)}
              </td>
              <td class="text-right py-3 px-4 font-semibold text-accent-600">
                +{result.score.toLocaleString()}
              </td>
              <td class="text-right py-3 px-4 font-bold">
                {result.total_score.toLocaleString()}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Continue prompt -->
    {#if state.currentRound < state.totalRounds}
      <div class="text-center mt-6">
        {#if game.mode === 'solo' && onNextRound}
          <button onclick={onNextRound} class="btn-primary">
            Continue to Round {state.currentRound + 1}
          </button>
        {:else}
          <p class="text-gray-600">Next round starting soon...</p>
        {/if}
      </div>
    {/if}
  </div>
</div>

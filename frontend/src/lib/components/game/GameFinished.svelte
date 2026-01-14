<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { user } from '$lib/stores/auth';

  interface Props {
    game: GameDetails;
  }

  let { game }: Props = $props();

  let standings = $derived($gameStore.finalStandings);
  // user_id is prefixed nanoid (usr_xxxxxxxxxxxx), safe string comparison
  let myRank = $derived(standings.find((s) => s.user_id === $user?.id)?.rank);
  let winner = $derived(standings[0]);

  function getRankDisplay(rank: number): string {
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
</script>

<div class="min-h-screen bg-gradient-to-b from-primary-50 to-white p-4">
  <div class="max-w-2xl mx-auto pt-12">
    <!-- Winner announcement -->
    {#if winner}
      <div class="text-center mb-12">
        <div class="text-6xl mb-4">*</div>
        <h1 class="text-3xl font-bold mb-2">
          {winner.display_name} Wins!
        </h1>
        <p class="text-xl text-gray-600">with {winner.total_score.toLocaleString()} points</p>
      </div>
    {/if}

    <!-- Full standings -->
    <div class="card">
      <h2 class="text-xl font-semibold mb-4">Final Standings</h2>

      <div class="space-y-3">
        {#each standings as standing}
          <div
            class="flex items-center justify-between p-4 rounded-lg"
            class:bg-yellow-50={standing.rank === 1}
            class:bg-gray-50={standing.rank !== 1}
            class:ring-2={standing.user_id === $user?.id}
            class:ring-primary-500={standing.user_id === $user?.id}
          >
            <div class="flex items-center gap-4">
              <span class="text-2xl w-10 text-center font-bold">
                {getRankDisplay(standing.rank)}
              </span>
              <span class="font-medium">{standing.display_name}</span>
            </div>
            <span class="text-xl font-bold">
              {standing.total_score.toLocaleString()}
            </span>
          </div>
        {/each}
      </div>
    </div>

    <!-- Your result -->
    {#if myRank}
      <div class="mt-6 p-4 bg-primary-50 rounded-xl text-center">
        <p class="text-lg">
          You finished in <span class="font-bold">{getRankDisplay(myRank)}</span> place!
        </p>
      </div>
    {/if}

    <!-- Actions -->
    <div class="mt-8 flex justify-center gap-4">
      <a href="/" class="btn-secondary"> Back to Home </a>
      <a href={`/game/${game.id}/rematch`} class="btn-primary"> Play Again </a>
    </div>
  </div>
</div>

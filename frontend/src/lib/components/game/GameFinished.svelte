<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { user } from '$lib/stores/auth';
  import { getRankDisplay, getRankClass, formatScore } from '$lib/utils.js';

  interface Props {
    game: GameDetails;
  }

  let { game }: Props = $props();

  let standings = $derived($gameStore.finalStandings);
  let myStanding = $derived(standings.find((s) => s.user_id === $user?.id));
  let myRank = $derived(myStanding?.rank);
  let winner = $derived(standings[0]);

  // Confetti effect for winner
  let showConfetti = $state(true);
  $effect(() => {
    const timer = setTimeout(() => {
      showConfetti = false;
    }, 3000);
    return () => clearTimeout(timer);
  });
</script>

<div class="min-h-screen bg-gradient-to-b from-primary-50 to-white p-4 relative overflow-hidden">
  <!-- Confetti animation (simple CSS version) -->
  {#if showConfetti}
    <div class="absolute inset-0 pointer-events-none overflow-hidden">
      {#each Array(30) as _, i}
        <div
          class="absolute w-3 h-3 opacity-80"
          style="
            left: {Math.random() * 100}%;
            animation: confetti-fall {2 + Math.random() * 2}s linear forwards;
            animation-delay: {Math.random() * 0.5}s;
            background: {['#FFD700', '#FF6B6B', '#4ECDC4', '#45B7D1', '#96E6A1', '#DDA0DD'][i % 6]};
            transform: rotate({Math.random() * 360}deg);
          "
        ></div>
      {/each}
    </div>
  {/if}

  <div class="max-w-2xl mx-auto pt-8 relative z-10">
    <!-- Winner announcement -->
    {#if winner}
      <div class="text-center mb-10">
        <!-- Trophy icon -->
        <svg class="w-20 h-20 mx-auto text-yellow-500 mb-4 animate-bounce" fill="currentColor" viewBox="0 0 24 24">
          <path d="M5 3h14a1 1 0 011 1v2a5 5 0 01-5 5h-.17A5.002 5.002 0 0112 15a5.002 5.002 0 01-2.83-4H9a5 5 0 01-5-5V4a1 1 0 011-1zm0 2v1a3 3 0 003 3h.17A5.002 5.002 0 0112 5V5H5zm14 0h-5v.17c1.17.41 2.15 1.26 2.75 2.33H17a3 3 0 003-3V5z"/>
          <path d="M12 15v2h3a1 1 0 011 1v2H8v-2a1 1 0 011-1h3v-2"/>
        </svg>
        <h1 class="text-4xl font-bold mb-3 text-gray-900">
          {winner.display_name} Wins!
        </h1>
        <p class="text-xl text-gray-600">
          with <span class="font-bold text-primary-600">{formatScore(winner.total_score)}</span> points
        </p>
      </div>
    {/if}

    <!-- Full standings -->
    <div class="bg-white rounded-2xl shadow-lg overflow-hidden">
      <div class="px-6 py-4 bg-gradient-to-r from-gray-50 to-gray-100 border-b">
        <h2 class="text-xl font-semibold text-gray-800">Final Standings</h2>
      </div>

      <div class="divide-y divide-gray-100">
        {#each standings as standing (standing.user_id)}
          {@const isCurrentUser = standing.user_id === $user?.id}
          {@const bgClass = standing.rank === 1 ? 'bg-yellow-50' : standing.rank === 2 ? 'bg-gray-100' : standing.rank === 3 ? 'bg-amber-50' : ''}
          <div
            class="flex items-center justify-between px-6 py-4 transition-colors {bgClass} {isCurrentUser ? 'ring-2 ring-primary-500 ring-inset bg-primary-50' : ''}"
          >
            <div class="flex items-center gap-4">
              <!-- Rank -->
              <span class="text-2xl font-bold w-14 text-center {getRankClass(standing.rank)}">
                {getRankDisplay(standing.rank)}
              </span>
              
              <!-- Player name -->
              <div>
                <span class="font-medium text-lg {isCurrentUser ? 'text-primary-700' : 'text-gray-900'}">
                  {standing.display_name}
                </span>
                {#if isCurrentUser}
                  <span class="ml-2 text-xs bg-primary-100 text-primary-700 px-2 py-0.5 rounded-full font-medium">
                    You
                  </span>
                {/if}
              </div>
            </div>
            
            <!-- Score -->
            <span class="text-2xl font-bold text-gray-900">
              {formatScore(standing.total_score)}
            </span>
          </div>
        {/each}
      </div>
    </div>

    <!-- Your result highlight -->
    {#if myRank && myRank > 1}
      <div class="mt-6 p-5 bg-gradient-to-r from-primary-50 to-primary-100 rounded-xl text-center border border-primary-200">
        <p class="text-lg text-gray-700">
          You finished in 
          <span class="font-bold text-primary-700 text-xl">{getRankDisplay(myRank)}</span> 
          place!
        </p>
        {#if myStanding}
          <p class="text-sm text-gray-600 mt-1">
            with {formatScore(myStanding.total_score)} total points
          </p>
        {/if}
      </div>
    {:else if myRank === 1}
      <div class="mt-6 p-5 bg-gradient-to-r from-yellow-50 to-yellow-100 rounded-xl text-center border border-yellow-300">
        <p class="text-lg font-bold text-yellow-800">
          Congratulations! You're the champion!
        </p>
      </div>
    {/if}

    <!-- Actions -->
    <div class="mt-8 flex justify-center gap-4">
      <a href="/" class="btn-secondary px-6 py-3">
        Back to Home
      </a>
      <a href="/play" class="btn-primary px-6 py-3">
        Play Again
      </a>
    </div>
  </div>
</div>

<style>
  @keyframes confetti-fall {
    0% {
      transform: translateY(-10vh) rotate(0deg);
      opacity: 1;
    }
    100% {
      transform: translateY(110vh) rotate(720deg);
      opacity: 0;
    }
  }
</style>

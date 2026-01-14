<script lang="ts">
  import { goto } from '$app/navigation';
  import { user, authStore } from '$lib/stores/auth';
  import { gamesApi } from '$lib/api/games';
  import { authApi } from '$lib/api/auth';

  let joinCode = $state('');
  let loading = $state(false);
  let error = $state('');

  async function startSoloGame() {
    loading = true;
    error = '';

    try {
      // Create guest if not logged in
      if (!$user) {
        await authStore.createGuest();
      }

      const game = await gamesApi.create({ mode: 'solo' });
      goto(`/game/${game.id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to start game';
    } finally {
      loading = false;
    }
  }

  async function createMultiplayerGame() {
    loading = true;
    error = '';

    try {
      if (!$user) {
        await authStore.createGuest();
      }

      const game = await gamesApi.create({ mode: 'multiplayer' });
      goto(`/game/${game.id}/lobby`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create game';
    } finally {
      loading = false;
    }
  }

  async function joinGame() {
    if (!joinCode.trim()) return;

    loading = true;
    error = '';

    try {
      if (!$user) {
        await authStore.createGuest();
      }

      const game = await gamesApi.joinByCode(joinCode.trim().toUpperCase());
      goto(`/game/${game.id}/lobby`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to join game';
    } finally {
      loading = false;
    }
  }
</script>

<div class="max-w-4xl mx-auto px-4 py-12">
  <div class="text-center mb-12">
    <h1 class="text-5xl font-bold text-gray-900 mb-4">
      Welcome to dguesser
    </h1>
    <p class="text-xl text-gray-600">
      Test your geography knowledge by guessing locations around the world
    </p>
  </div>

  {#if error}
    <div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700">
      {error}
    </div>
  {/if}

  <div class="grid md:grid-cols-2 gap-6">
    <!-- Solo Play -->
    <div class="card">
      <h2 class="text-2xl font-semibold mb-4">Solo Play</h2>
      <p class="text-gray-600 mb-6">
        Play on your own and try to beat your high score. No account required.
      </p>
      <button
        onclick={startSoloGame}
        disabled={loading}
        class="btn-primary w-full"
      >
        {loading ? 'Starting...' : 'Start Solo Game'}
      </button>
    </div>

    <!-- Multiplayer -->
    <div class="card">
      <h2 class="text-2xl font-semibold mb-4">Multiplayer</h2>
      <p class="text-gray-600 mb-4">
        Create a room or join friends with a code.
      </p>
      
      <button
        onclick={createMultiplayerGame}
        disabled={loading}
        class="btn-accent w-full mb-4"
      >
        Create Room
      </button>

      <div class="flex gap-2">
        <input
          type="text"
          bind:value={joinCode}
          placeholder="Enter join code"
          maxlength="6"
          class="input uppercase"
        />
        <button
          onclick={joinGame}
          disabled={loading || !joinCode.trim()}
          class="btn-secondary"
        >
          Join
        </button>
      </div>
    </div>
  </div>

  <!-- Leaderboard link -->
  <div class="mt-8 text-center">
    <a
      href="/leaderboard"
      class="inline-flex items-center gap-2 text-primary-600 hover:text-primary-700 font-medium"
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
        />
      </svg>
      View Global Leaderboard
    </a>
  </div>

  <!-- Sign in prompt for guests -->
  {#if $user?.is_guest}
    <div class="mt-8 p-6 bg-primary-50 rounded-xl text-center">
      <h3 class="text-lg font-semibold text-primary-900 mb-2">
        Want to save your progress?
      </h3>
      <p class="text-primary-700 mb-4">
        Sign in to track your scores and compete on leaderboards.
      </p>
      <div class="flex justify-center gap-4">
        <a href={authApi.getGoogleAuthUrl()} class="btn-primary">
          Sign in with Google
        </a>
        <a href={authApi.getMicrosoftAuthUrl()} class="btn-secondary">
          Sign in with Microsoft
        </a>
      </div>
    </div>
  {/if}

  <!-- Stats for logged in users -->
  {#if $user && !$user.is_guest}
    <div class="mt-8 grid grid-cols-3 gap-4">
      <div class="card text-center">
        <div class="text-3xl font-bold text-primary-600">
          {$user.games_played}
        </div>
        <div class="text-gray-600">Games Played</div>
      </div>
      <div class="card text-center">
        <div class="text-3xl font-bold text-accent-600">
          {$user.total_score.toLocaleString()}
        </div>
        <div class="text-gray-600">Total Score</div>
      </div>
      <div class="card text-center">
        <div class="text-3xl font-bold text-yellow-600">
          {$user.best_score.toLocaleString()}
        </div>
        <div class="text-gray-600">Best Score</div>
      </div>
    </div>
  {/if}
</div>

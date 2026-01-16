<script lang="ts">
  import { goto } from '$app/navigation';
  import { user, authStore } from '$lib/stores/auth';
  import { authModalOpen } from '$lib/stores/authModal';
  import { gamesApi } from '$lib/api/games';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as Card from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import * as Alert from '$lib/components/ui/alert';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import UsersIcon from '@lucide/svelte/icons/users';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import PlayIcon from '@lucide/svelte/icons/play';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import TargetIcon from '@lucide/svelte/icons/target';
  import MapPinIcon from '@lucide/svelte/icons/map-pin';
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';

  let joinCode = $state('');
  let loading = $state(false);
  let error = $state('');

  async function startSoloGame() {
    loading = true;
    error = '';

    try {
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
      goto(`/game/${game.id}`);
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
      goto(`/game/${game.id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to join game';
    } finally {
      loading = false;
    }
  }
</script>

<!-- Hero Section -->
<section class="relative overflow-hidden bg-gradient-to-b from-background to-muted/30 py-20 md:py-32">
  <div class="absolute inset-0 -z-10">
    <div class="absolute inset-0 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-primary/10 via-transparent to-transparent"></div>
    <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 size-[600px] rounded-full bg-primary/5 blur-3xl"></div>
  </div>
  
  <div class="container mx-auto px-4 text-center">
    <Badge variant="secondary" class="mb-4">
      <SparklesIcon class="size-3 mr-1" />
      Free to play
    </Badge>
    
    <h1 class="text-4xl md:text-6xl font-bold tracking-tight mb-6 bg-gradient-to-r from-foreground to-foreground/70 bg-clip-text">
      Explore the World,
      <br />
      <span class="text-primary">One Guess at a Time</span>
    </h1>
    
    <p class="text-lg md:text-xl text-muted-foreground max-w-2xl mx-auto mb-8">
      Test your geography knowledge by guessing locations from around the world. 
      Challenge yourself solo or compete with friends in real-time multiplayer.
    </p>

    <div class="flex flex-col sm:flex-row gap-4 justify-center items-center">
      <Button size="lg" href="/play" class="w-full sm:w-auto">
        <PlayIcon class="size-5" />
        Play Now
      </Button>
      <Button variant="outline" size="lg" href="/leaderboard" class="w-full sm:w-auto">
        <TrophyIcon class="size-5" />
        View Leaderboard
      </Button>
    </div>
  </div>
</section>

<!-- Error Alert -->
{#if error}
  <div class="container mx-auto px-4 -mt-8 mb-8">
    <Alert.Root variant="destructive">
      <AlertCircleIcon class="size-4" />
      <Alert.Title>Error</Alert.Title>
      <Alert.Description>{error}</Alert.Description>
    </Alert.Root>
  </div>
{/if}

<!-- Game Mode Cards -->
<section class="container mx-auto px-4 py-16">
  <div class="text-center mb-12">
    <h2 class="text-3xl font-bold mb-4">Choose Your Mode</h2>
    <p class="text-muted-foreground max-w-xl mx-auto">
      Play solo to practice or create a room to challenge your friends
    </p>
  </div>

  <div class="grid md:grid-cols-2 gap-6 max-w-4xl mx-auto">
    <!-- Solo Play Card -->
    <Card.Root class="relative overflow-hidden group hover:shadow-lg transition-all duration-300 border-2 hover:border-primary/50">
      <div class="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
      <Card.Header>
        <div class="flex items-center gap-3 mb-2">
          <div class="p-2 rounded-lg bg-primary/10">
            <TargetIcon class="size-6 text-primary" />
          </div>
          <Card.Title class="text-2xl">Solo Play</Card.Title>
        </div>
        <Card.Description class="text-base">
          Play on your own and try to beat your personal best. Perfect for practice!
        </Card.Description>
      </Card.Header>
      <Card.Content>
        <ul class="space-y-2 text-sm text-muted-foreground mb-6">
          <li class="flex items-center gap-2">
            <MapPinIcon class="size-4 text-primary" />
            5 rounds per game
          </li>
          <li class="flex items-center gap-2">
            <TrophyIcon class="size-4 text-primary" />
            Track your progress
          </li>
          <li class="flex items-center gap-2">
            <SparklesIcon class="size-4 text-primary" />
            No account required
          </li>
        </ul>
      </Card.Content>
      <Card.Footer>
        <Button onclick={startSoloGame} disabled={loading} class="w-full">
          <PlayIcon class="size-4" />
          {loading ? 'Starting...' : 'Start Solo Game'}
        </Button>
      </Card.Footer>
    </Card.Root>

    <!-- Multiplayer Card -->
    <Card.Root class="relative overflow-hidden group hover:shadow-lg transition-all duration-300 border-2 hover:border-primary/50">
      <div class="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
      <Card.Header>
        <div class="flex items-center gap-3 mb-2">
          <div class="p-2 rounded-lg bg-primary/10">
            <UsersIcon class="size-6 text-primary" />
          </div>
          <Card.Title class="text-2xl">Multiplayer</Card.Title>
        </div>
        <Card.Description class="text-base">
          Create a room or join friends with a code. Compete in real-time!
        </Card.Description>
      </Card.Header>
      <Card.Content>
        <div class="space-y-4">
          <Button variant="outline" onclick={createMultiplayerGame} disabled={loading} class="w-full">
            <UsersIcon class="size-4" />
            Create Room
          </Button>
          
          <div class="relative">
            <div class="absolute inset-0 flex items-center">
              <span class="w-full border-t"></span>
            </div>
            <div class="relative flex justify-center text-xs uppercase">
              <span class="bg-card px-2 text-muted-foreground">or join with code</span>
            </div>
          </div>

          <div class="flex gap-2">
            <Input
              type="text"
              bind:value={joinCode}
              placeholder="Enter code"
              maxlength={6}
              class="uppercase font-mono text-center tracking-widest"
            />
            <Button
              variant="secondary"
              onclick={joinGame}
              disabled={loading || !joinCode.trim()}
            >
              Join
              <ArrowRightIcon class="size-4" />
            </Button>
          </div>
        </div>
      </Card.Content>
    </Card.Root>
  </div>
</section>

<!-- User Stats Section (for logged in users) -->
{#if $user && !$user.is_guest}
  <section class="container mx-auto px-4 pb-16">
    <div class="text-center mb-8">
      <h2 class="text-2xl font-bold mb-2">Your Stats</h2>
      <p class="text-muted-foreground">Keep up the great work, {$user.display_name}!</p>
    </div>
    
    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 max-w-3xl mx-auto">
      <Card.Root class="text-center">
        <Card.Content class="pt-6">
          <div class="text-4xl font-bold text-primary mb-1">
            {$user.games_played}
          </div>
          <p class="text-sm text-muted-foreground">Games Played</p>
        </Card.Content>
      </Card.Root>
      
      <Card.Root class="text-center">
        <Card.Content class="pt-6">
          <div class="text-4xl font-bold text-primary mb-1">
            {$user.total_score.toLocaleString()}
          </div>
          <p class="text-sm text-muted-foreground">Total Score</p>
        </Card.Content>
      </Card.Root>
      
      <Card.Root class="text-center">
        <Card.Content class="pt-6">
          <div class="text-4xl font-bold text-amber-500 mb-1">
            {$user.best_score.toLocaleString()}
          </div>
          <p class="text-sm text-muted-foreground">Best Score</p>
        </Card.Content>
      </Card.Root>
    </div>
  </section>
{/if}

<!-- Sign in CTA for guests -->
{#if $user?.is_guest}
  <section class="container mx-auto px-4 pb-16">
    <Card.Root class="max-w-2xl mx-auto bg-gradient-to-r from-primary/10 via-primary/5 to-transparent border-primary/20">
      <Card.Content class="flex flex-col sm:flex-row items-center justify-between gap-6 py-8">
        <div class="text-center sm:text-left">
          <h3 class="text-xl font-semibold mb-2">Want to save your progress?</h3>
          <p class="text-muted-foreground">
            Sign in to track your scores and compete on leaderboards.
          </p>
        </div>
        <Button onclick={() => authModalOpen.open()}>
          Sign In
        </Button>
      </Card.Content>
    </Card.Root>
  </section>
{/if}

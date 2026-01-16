<script lang="ts">
  import { user } from '$lib/stores/auth';
  import { authModalOpen } from '$lib/stores/authModal';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import UsersIcon from '@lucide/svelte/icons/users';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import PlayIcon from '@lucide/svelte/icons/play';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import TargetIcon from '@lucide/svelte/icons/target';
  import MapPinIcon from '@lucide/svelte/icons/map-pin';
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import ZapIcon from '@lucide/svelte/icons/zap';
  import Share2Icon from '@lucide/svelte/icons/share-2';
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
    <a href="/play" class="block group">
      <Card.Root class="relative overflow-hidden h-full hover:shadow-lg transition-all duration-300 border-2 hover:border-primary/50">
        <div class="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
        <Card.Header>
          <div class="flex items-center gap-3 mb-2">
            <div class="p-2 rounded-lg bg-primary/10 transition-colors group-hover:bg-primary/20">
              <TargetIcon class="size-6 text-primary" />
            </div>
            <Card.Title class="text-2xl">Solo Play</Card.Title>
          </div>
          <Card.Description class="text-base">
            Play on your own and try to beat your personal best. Perfect for practice!
          </Card.Description>
        </Card.Header>
        <Card.Content>
          <ul class="space-y-2 text-sm text-muted-foreground">
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
        <!-- Hover Overlay -->
        <div class="absolute inset-0 flex items-center justify-center bg-background/80 backdrop-blur-sm opacity-0 group-hover:opacity-100 transition-all duration-300">
          <div class="flex items-center gap-2 text-lg font-semibold text-primary">
            <PlayIcon class="size-5" />
            Start Playing
            <ArrowRightIcon class="size-5 transition-transform group-hover:translate-x-1" />
          </div>
        </div>
      </Card.Root>
    </a>

    <!-- Multiplayer Card -->
    <a href="/play" class="block group">
      <Card.Root class="relative overflow-hidden h-full hover:shadow-lg transition-all duration-300 border-2 hover:border-primary/50">
        <div class="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity"></div>
        <Card.Header>
          <div class="flex items-center gap-3 mb-2">
            <div class="p-2 rounded-lg bg-primary/10 transition-colors group-hover:bg-primary/20">
              <UsersIcon class="size-6 text-primary" />
            </div>
            <Card.Title class="text-2xl">Multiplayer</Card.Title>
          </div>
          <Card.Description class="text-base">
            Create a room or join friends with a code. Compete in real-time!
          </Card.Description>
        </Card.Header>
        <Card.Content>
          <ul class="space-y-2 text-sm text-muted-foreground">
            <li class="flex items-center gap-2">
              <ZapIcon class="size-4 text-primary" />
              Real-time competition
            </li>
            <li class="flex items-center gap-2">
              <Share2Icon class="size-4 text-primary" />
              Share room codes
            </li>
            <li class="flex items-center gap-2">
              <UsersIcon class="size-4 text-primary" />
              Play with friends
            </li>
          </ul>
        </Card.Content>
        <!-- Hover Overlay -->
        <div class="absolute inset-0 flex items-center justify-center bg-background/80 backdrop-blur-sm opacity-0 group-hover:opacity-100 transition-all duration-300">
          <div class="flex items-center gap-2 text-lg font-semibold text-primary">
            <PlayIcon class="size-5" />
            Start Playing
            <ArrowRightIcon class="size-5 transition-transform group-hover:translate-x-1" />
          </div>
        </div>
      </Card.Root>
    </a>
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
